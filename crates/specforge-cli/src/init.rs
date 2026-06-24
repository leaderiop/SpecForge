use serde_json::json;
use specforge_common::find_project_root;
use std::path::Path;

pub fn run(path: &Path, name: Option<&str>, version: Option<&str>, extensions: &[String], format: &str) -> i32 {
    // Check for existing project
    if let Some(existing) = find_project_root(path) {
        eprintln!(
            "error: project already exists at {}",
            existing.display()
        );
        return 1;
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
    let config_str = serde_json::to_string_pretty(&config).expect("serialize JSON output");
    if let Err(e) = std::fs::write(&config_path, format!("{config_str}\n")) {
        eprintln!("error: failed to write specforge.json: {e}");
        return 1;
    }

    // Create spec_root directory
    let spec_dir = path.join(spec_root);
    if let Err(e) = std::fs::create_dir_all(&spec_dir) {
        eprintln!("error: failed to create spec directory: {e}");
        return 1;
    }

    // Append specforge-infer.json to .gitignore (create if missing)
    let gitignore_path = path.join(".gitignore");
    let needs_entry = match std::fs::read_to_string(&gitignore_path) {
        Ok(content) => !content.lines().any(|l| l.trim() == "specforge-infer.json"),
        Err(_) => true,
    };
    if needs_entry {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)
            .ok();
        if let Some(ref mut f) = file {
            use std::io::Write;
            let _ = writeln!(f, "specforge-infer.json");
        }
    }

    // Write starter spec file — choose template based on installed extensions
    let has_software = extensions.iter().any(|e| e.contains("software"));
    let has_product = extensions.iter().any(|e| e.contains("product"));
    let spec_id = sanitize_entity_id(&project_name);
    let (starter_filename, starter_content) = if has_software {
        ("hello.spec", generate_software_starter(&spec_id))
    } else if has_product {
        ("hello.spec", generate_product_starter(&spec_id))
    } else {
        ("hello.spec", generate_starter_spec(&spec_id))
    };
    let starter_path = spec_dir.join(starter_filename);
    if let Err(e) = std::fs::write(&starter_path, starter_content) {
        eprintln!("error: failed to write starter spec file: {e}");
        return 1;
    }

    // Output
    match format {
        "json" => {
            let output = json!({
                "project_root": path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
                "config_path": config_path,
                "spec_file_path": starter_path,
                "extensions_installed": extensions,
            });
            println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
        }
        _ => {
            println!("Initialized project '{}' at {}", project_name, path.display());
            println!("  specforge.json");
            println!("  {}/hello.spec", spec_root);
            if extensions.is_empty() {
                println!("\nNo extensions installed. Add one with: specforge add <extension>");
            }
            println!("\nNext steps:");
            println!("  specforge check    # validate your spec files");
            println!("  specforge export   # export the graph");
        }
    }

    0
}

fn validate_project_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("name must not be empty");
    }
    if name.len() > 214 {
        return Err("name must not exceed 214 characters");
    }
    if name.starts_with('.') || name.starts_with('-') {
        return Err("name must not start with '.' or '-'");
    }
    if name.contains(|c: char| c.is_whitespace()) {
        return Err("name must not contain whitespace");
    }
    Ok(())
}

fn validate_extension_specifier(spec: &str) -> Result<(), &'static str> {
    // Extension specifiers must follow @scope/name or @scope/name@version format
    let base = spec.split('@').filter(|s| !s.is_empty()).collect::<Vec<_>>();
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
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
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

fn generate_software_starter(project_name: &str) -> String {
    format!(
        r#"// {project_name} — software specification
//
// Uses @specforge/software entity kinds: behavior, type, event, port, invariant.
//
// Try: specforge check

spec "{project_name}" {{
  version "0.1.0"
}}

type user "User account" {{
  status draft
}}

behavior authenticate_user "Authenticate a user with credentials" {{
  status   draft
  category "auth"
  contract "Given valid credentials, returns an auth token"
  produces [user_logged_in]

  verify "rejects invalid password"
  verify "returns token on success"
}}

event user_logged_in "User successfully logged in" {{
  payload user
}}
"#
    )
}

fn generate_product_starter(project_name: &str) -> String {
    format!(
        r#"// {project_name} — product specification
//
// Uses @specforge/product entity kinds: feature, journey, milestone, deliverable, etc.
//
// Try: specforge check

spec "{project_name}" {{
  version "0.1.0"
}}

persona developer "Software developer" {{
  technical_level expert
  status          active
}}

channel cli "Command-line interface" {{
  status active
}}

feature user_auth "User authentication" {{
  status   proposed
  priority high
  problem  "Users need secure access to the system"
}}

journey onboarding "New user onboarding" {{
  persona  developer
  channels [cli]
  features [user_auth]
}}

module core "Core module" {{
  features [user_auth]
}}

milestone mvp "Minimum Viable Product" {{
  status        planned
  features      [user_auth]
  modules       [core]
  exit_criteria ["Core auth flow works end-to-end"]
}}

deliverable app "Application" {{
  status        draft
  artifact_type cli
  journeys      [onboarding]
  modules       [core]
  milestones    [mvp]
}}
"#
    )
}

