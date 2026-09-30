use crate::OutputFormat;
use serde_json::json;
use specforge_common::{find_project_root, validate_project_name};
use std::path::Path;

pub fn run(
    path: &Path,
    name: Option<&str>,
    version: Option<&str>,
    extensions: &[String],
    format: OutputFormat,
) -> i32 {
    // Test obligations (`verify`) on software kinds come from @specforge/testing
    // (ADR 0002), so enabling software enables it too.
    let mut extensions = extensions.to_vec();
    if extensions.iter().any(|e| e == "@specforge/software")
        && !extensions.iter().any(|e| e == "@specforge/testing")
    {
        extensions.push("@specforge/testing".to_string());
    }
    // The project's test runners get the extensions that collect their
    // results.
    if extensions.iter().any(|e| e == "@specforge/testing") {
        for runner in detected_runners(path) {
            if !extensions.iter().any(|e| e == runner) {
                extensions.push(runner.to_string());
            }
        }
    }
    let extensions = extensions.as_slice();

    // Only a project in this very directory blocks init. One further up
    // doesn't: the new project is separate, and commands run inside it
    // resolve to it because the nearest project wins.
    if ["specforge.json", "specforge.spec"]
        .iter()
        .any(|marker| path.join(marker).exists())
    {
        eprintln!("error: project already exists at {}", path.display());
        return 1;
    }
    if let Some(enclosing) = path.parent().and_then(find_project_root)
        && format != OutputFormat::Json
    {
        eprintln!(
            "note: {} is inside the project at {}; the new project is separate",
            path.display(),
            enclosing.display()
        );
    }

    // Determine project name: --name flag, or directory name
    let project_name = match name {
        Some(n) => n.to_string(),
        None => path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-project")
            .to_string(),
    };

    // Validate project name
    if let Err(msg) = validate_project_name(&project_name) {
        eprintln!("error: invalid project name '{}': {}", project_name, msg);
        return 1;
    }

    // Validate extension specifiers
    for ext in extensions {
        if let Err(msg) = validate_extension_specifier(ext) {
            eprintln!("error: unresolvable extension '{}': {}", ext, msg);
            return 1;
        }
    }

    let project_version = version.unwrap_or("0.1.0");
    let spec_root = "spec";

    // Build specforge.json
    let config = json!({
        "$schema": "https://specforge.dev/schema/specforge.json",
        "name": project_name,
        "version": project_version,
        "spec_root": spec_root,
        "extensions": extensions,
    });

    // Write specforge.json
    let config_path = path.join("specforge.json");
    if let Err(e) = specforge_ops::config::write(path, &config) {
        eprintln!("error: {}", e.message);
        return 1;
    }

    // Create spec_root directory
    let spec_dir = path.join(spec_root);
    if let Err(e) = std::fs::create_dir_all(&spec_dir) {
        eprintln!("error: failed to create spec directory: {e}");
        return 1;
    }

    // Append the generated files to .gitignore (create if missing): the
    // inference cache, the report `collect` writes, and its working dir.
    let gitignore_path = path.join(".gitignore");
    let existing = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
    let missing: Vec<&str> = [
        "specforge-infer.json",
        "specforge-report.json",
        ".specforge/",
    ]
    .into_iter()
    .filter(|entry| !existing.lines().any(|l| l.trim() == *entry))
    .collect();
    if !missing.is_empty() {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)
            .ok();
        if let Some(ref mut f) = file {
            use std::io::Write;
            if !existing.is_empty() && !existing.ends_with('\n') {
                let _ = writeln!(f);
            }
            for entry in missing {
                let _ = writeln!(f, "{entry}");
            }
        }
    }

    // Write the starter spec file: an enabled extension's template, or the
    // structural starter when none contributes one.
    let spec_id = sanitize_entity_id(&project_name);
    let starter_content = match contributed_starter_template(path) {
        Some(template) => template.replace("{project}", &spec_id),
        None => generate_starter_spec(&spec_id),
    };
    let starter_path = spec_dir.join("hello.spec");
    if let Err(e) = std::fs::write(&starter_path, starter_content) {
        eprintln!("error: failed to write starter spec file: {e}");
        return 1;
    }

    // Output
    match format {
        OutputFormat::Json => {
            let output = json!({
                "project_root": path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
                "config_path": config_path,
                "spec_file_path": starter_path,
                "extensions_installed": extensions,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            println!(
                "Initialized project '{}' at {}",
                project_name,
                path.display()
            );
            println!("  specforge.json");
            println!("  {}/hello.spec", spec_root);
            if extensions.is_empty() {
                println!("\nNo extensions installed. Add one with: specforge add <extension>");
            }
            println!("\nNext steps:");
            println!("  specforge check    # validate your spec files");
            println!("  specforge export   # export the graph");
            if extensions
                .iter()
                .any(|e| e == "@specforge/cargo-test" || e == "@specforge/vitest")
            {
                println!("  specforge collect  # run the tests and record what they prove");
            }
        }
    }

    0
}

fn validate_extension_specifier(spec: &str) -> Result<(), &'static str> {
    // Extension specifiers must follow @scope/name or @scope/name@version format
    let base = spec
        .split('@')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if base.is_empty() {
        return Err("extension specifier must not be empty");
    }
    // Must start with @
    if !spec.starts_with('@') {
        return Err("extension specifier must start with '@' (e.g., @specforge/software)");
    }
    // Strip leading @ and optional trailing @version
    let without_at = &spec[1..];
    let name_part = if let Some(idx) = without_at.find('@') {
        &without_at[..idx]
    } else {
        without_at
    };
    // Must contain scope/name
    if !name_part.contains('/') {
        return Err("extension specifier must be @scope/name (e.g., @specforge/software)");
    }
    Ok(())
}

fn sanitize_entity_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The starter template contributed by the extensions enabled in the
/// `specforge.json` at `path`, read from their manifests. When several
/// contribute one, the extension listed first in `extensions` wins, so the
/// user picks by ordering the list. `None` when no enabled extension
/// declares a template (or none could be loaded).
fn contributed_starter_template(path: &Path) -> Option<String> {
    let config = specforge_common::load_project_config(path);
    let runtime = crate::pipeline::build_runtime(path);
    // Load failures only cost the extension its template; `check` reports them.
    let mut ignored = Vec::new();
    specforge_emitter::compile::load_extensions(&config.extensions, &runtime, &mut ignored)
        .into_iter()
        .find_map(|manifest| manifest.starter_template)
}

fn generate_starter_spec(project_name: &str) -> String {
    format!(
        r#"// {project_name} — starter spec file
//
// This file uses only structural syntax that the core compiler
// understands without any extensions. Install extensions to unlock
// domain-specific entity types.
//
// Try: specforge check

spec "{project_name}" {{
  version "0.1.0"
}}
"#
    )
}

/// Runner extensions for the test runners the project at `path` uses.
fn detected_runners(path: &Path) -> Vec<&'static str> {
    let mut runners = Vec::new();
    if path.join("Cargo.toml").is_file() {
        runners.push("@specforge/cargo-test");
    }
    if uses_vitest(path) {
        runners.push("@specforge/vitest");
    }
    runners
}

/// A vitest config file, or vitest among package.json's dependencies (a
/// project may configure it inside vite.config.*).
fn uses_vitest(path: &Path) -> bool {
    let config = std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| {
            e.file_name().to_str().is_some_and(|n| {
                n.starts_with("vitest.config.") || n.starts_with("vitest.workspace.")
            })
        });
    config
        || std::fs::read_to_string(path.join("package.json"))
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .is_some_and(|pkg| {
                ["dependencies", "devDependencies"]
                    .iter()
                    .any(|deps| pkg[deps].get("vitest").is_some())
            })
}
