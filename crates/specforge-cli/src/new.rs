//! `specforge new --extension <name>` — scaffold an SDK-authored extension
//! project (spec #21 follow-through / SDK adoption).
//!
//! The generated project mirrors `fixtures/greet-extension`: a cdylib crate
//! depending on `specforge-extension-sdk`, targeting `wasm32-unknown-unknown`,
//! with a `src/lib.rs` skeleton that builds and describes out of the box.

use crate::OutputFormat;
use serde_json::json;
use std::path::{Path, PathBuf};

pub fn run(name: &str, extension: bool, path: &Path, format: OutputFormat) -> i32 {
    if !extension {
        print_error(
            format,
            "only `--extension` scaffolding is supported right now",
            "E065",
        );
        return 1;
    }

    if let Err(message) = validate_name(name) {
        print_error(format, &message, "E065");
        return 1;
    }

    let dir = target_dir(path, name);
    if dir.exists() {
        print_error(
            format,
            &format!("destination '{}' already exists", dir.display()),
            "E065",
        );
        return 1;
    }

    if let Err(message) = scaffold(&dir, name) {
        print_error(format, &message, "E066");
        return 1;
    }

    match format {
        OutputFormat::Json => {
            let output = json!({
                "action": "new",
                "kind": "extension",
                "name": name,
                "path": dir.display().to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            println!("scaffolded extension '{}' in {}", name, dir.display());
            println!();
            println!("next steps:");
            println!("  cd {}", dir.display());
            println!("  cargo build --release --target wasm32-unknown-unknown");
            let wasm = format!(
                "./target/wasm32-unknown-unknown/release/{}.wasm",
                crate_name(name).replace('-', "_")
            );
            println!("  specforge add {wasm}   # local-path install once built");
        }
    }
    0
}

/// Validate an extension name: `@scope/name` (npm-style) or `name`.
fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("extension name must not be empty".to_string());
    }
    if let Some(rest) = name.strip_prefix('@')
        && (!rest.contains('/') || rest.starts_with('/') || rest.ends_with('/'))
    {
        return Err(format!("scoped name '{}' must look like @scope/name", name));
    }
    Ok(())
}

/// The crate- and directory-safe name: the last path segment, sanitized.
fn crate_name(name: &str) -> String {
    let last = name.rsplit('/').next().unwrap_or(name);
    let sanitized: String = last
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if sanitized.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("ext-{}", sanitized)
    } else {
        sanitized
    }
}

fn target_dir(path: &Path, name: &str) -> PathBuf {
    path.join(crate_name(name))
}

fn scaffold(dir: &Path, name: &str) -> Result<(), String> {
    let crate_name = crate_name(name);
    let src = dir.join("src");
    std::fs::create_dir_all(&src)
        .map_err(|e| format!("failed to create {}: {}", src.display(), e))?;
    std::fs::create_dir_all(dir.join(".cargo"))
        .map_err(|e| format!("failed to create .cargo: {}", e))?;

    let cargo_toml = format!(
        r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
specforge-extension-sdk = "0.1"
wit-bindgen = "0.30"
"#
    );
    std::fs::write(dir.join("Cargo.toml"), cargo_toml)
        .map_err(|e| format!("failed to write Cargo.toml: {}", e))?;

    std::fs::write(
        dir.join(".cargo").join("config.toml"),
        "[build]\ntarget = \"wasm32-wasip2\"\n",
    )
    .map_err(|e| format!("failed to write .cargo/config.toml: {}", e))?;

    let lib_rs = format!(
        r#"//! {name} — a SpecForge extension authored with the extension SDK.
//!
//! Build:  cargo build --release --target wasm32-wasip2
//! Install: specforge add ./path/to/this/dir

use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "{name}",
    version = "0.1.0",
    short = "TODO: one-line description"
)]
struct Extension;

impl Contributions for Extension {{
    fn contribute(c: &mut ContributionsBuilder) {{
        // Contribute an entity kind (delete if not needed):
        c.kind("thing", |k| {{
            k.description("TODO: what this entity represents");
            k.field("description", |f| {{
                f.field_type(FieldType::String)
                    .description("Free-form description");
            }});
        }});

        // Contribute a validation rule (delete if not needed):
        c.rule("W900", |r| {{
            r.check(CheckKind::MissingRequiredField);
            r.target_kind("thing");
            r.field("description");
            r.severity(ValidationSeverity::Warning);
            r.message_template("thing '{{id}}' is missing a description");
        }});
    }}
}}

fn dispatch(_export: &str, _input: &[u8]) -> Option<Result<Vec<u8>, String>> {{
    None
}}

specforge_extension_sdk::component_guest!(
    build = specforge_extension_build,
    handler = dispatch
);
"#
    );
    std::fs::write(src.join("lib.rs"), lib_rs)
        .map_err(|e| format!("failed to write src/lib.rs: {}", e))?;
    Ok(())
}

fn print_error(format: OutputFormat, message: &str, code: &str) {
    match format {
        OutputFormat::Json => {
            let output = json!({"error": message, "code": code});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => eprintln!("error[{}]: {}", code, message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_takes_last_segment_and_sanitizes() {
        assert_eq!(crate_name("@you/my-ext"), "my-ext");
        assert_eq!(crate_name("plain"), "plain");
        assert_eq!(crate_name("@weird scope/name!"), "name-");
    }

    #[test]
    fn crate_name_prefixes_leading_digit() {
        assert_eq!(crate_name("@x/4th-kind"), "ext-4th-kind");
    }

    #[test]
    fn scoped_names_require_a_slash() {
        assert!(validate_name("@justscope").is_err());
        assert!(validate_name("@ok/name").is_ok());
        assert!(validate_name("unscoped").is_ok());
        assert!(validate_name("   ").is_err());
    }

    #[test]
    fn scaffold_writes_buildable_shape() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("my-ext");
        scaffold(&project, "@you/my-ext").unwrap();

        let cargo = std::fs::read_to_string(project.join("Cargo.toml")).unwrap();
        assert!(cargo.contains(r#"name = "my-ext""#));
        assert!(cargo.contains("crate-type = [\"cdylib\"]"));
        assert!(cargo.contains("specforge-extension-sdk"));

        let config = std::fs::read_to_string(project.join(".cargo/config.toml")).unwrap();
        assert!(config.contains("wasm32-wasip2"));

        let lib = std::fs::read_to_string(project.join("src/lib.rs")).unwrap();
        assert!(lib.contains("name = \"@you/my-ext\""));
        assert!(lib.contains("#[specforge_extension_sdk::extension("));
        assert!(lib.contains("impl Contributions for Extension"));
        assert!(lib.contains("component_guest!"));
    }

    /// Every field type and check kind the scaffold names is one the
    /// host's registry build reads, so a freshly scaffolded extension loads
    /// without W019/W112 (the scaffold once used a check name it rejected).
    #[test]
    fn scaffold_uses_only_vocabulary_the_host_reads() {
        use specforge_protocol_types::{CheckKind, FieldType};

        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("my-ext");
        scaffold(&project, "@you/my-ext").unwrap();
        let lib = std::fs::read_to_string(project.join("src/lib.rs")).unwrap();

        let named = |prefix: &str| -> Vec<String> {
            lib.match_indices(prefix)
                .map(|(at, _)| {
                    lib[at + prefix.len()..]
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric())
                        .collect()
                })
                .collect()
        };
        let checks = named("CheckKind::");
        let types = named("FieldType::");
        assert!(!checks.is_empty() && !types.is_empty(), "{lib}");
        for name in checks {
            assert!(
                CheckKind::ALL.iter().any(|c| format!("{c:?}") == name),
                "scaffold names unknown CheckKind::{name}"
            );
        }
        for name in types {
            assert!(
                FieldType::ALL.iter().any(|t| format!("{t:?}") == name),
                "scaffold names unknown FieldType::{name}"
            );
        }
    }
}
