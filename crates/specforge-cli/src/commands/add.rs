use crate::pipeline;
use clap::Args;
use std::fs;
use std::path::PathBuf;

/// Known plugin packages.
const VALID_PLUGINS: &[&str] = &["@specforge/product", "@specforge/governance"];

#[derive(Args)]
pub struct AddArgs {
    /// Plugin package name (e.g. @specforge/product)
    pub plugin: String,

    /// Path to project root (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Run the add command. Returns exit code.
pub fn run(args: AddArgs) -> i32 {
    // Accept built-in plugins and local path plugins (starting with ./ or ../)
    let is_valid = VALID_PLUGINS.contains(&args.plugin.as_str())
        || specforge_wasm::discover::is_local_path(&args.plugin);

    if !is_valid {
        eprintln!(
            "specforge: unknown plugin \"{}\"\nvalid plugins: {}\nhint: use a local path (e.g., ./plugins/my-plugin) for Wasm plugins",
            args.plugin,
            VALID_PLUGINS.join(", ")
        );
        return 1;
    }

    // Check for specforge.json first, then fall back to specforge.spec
    let project_root = pipeline::find_project_root(&args.path);

    match project_root {
        Some(pipeline::ProjectRoot::Json(json_path)) => {
            add_to_json(&json_path, &args.plugin)
        }
        Some(pipeline::ProjectRoot::Spec(spec_path)) => {
            add_to_spec(&spec_path, &args.plugin)
        }
        None => {
            // Fall back to looking for specforge.spec in path directly
            let spec_path = args.path.join("specforge.spec");
            if spec_path.exists() {
                add_to_spec(&spec_path, &args.plugin)
            } else {
                eprintln!(
                    "specforge: no specforge.json or specforge.spec found in {}\nhint: run `specforge init` first",
                    args.path.display()
                );
                1
            }
        }
    }
}

fn add_to_json(json_path: &std::path::Path, plugin: &str) -> i32 {
    let mut config = match pipeline::load_json_config(json_path) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("specforge: {msg}");
            return 1;
        }
    };

    if config.plugins.contains(&plugin.to_string()) {
        eprintln!("specforge: {} is already installed", plugin);
        return 0;
    }

    config.plugins.push(plugin.to_string());

    let json = match serde_json::to_string_pretty(&config) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("specforge: error serializing config: {e}");
            return 1;
        }
    };

    if let Err(e) = fs::write(json_path, format!("{json}\n")) {
        eprintln!("specforge: error writing {}: {e}", json_path.display());
        return 1;
    }

    eprintln!("specforge: added {plugin}");
    0
}

fn add_to_spec(spec_path: &std::path::Path, plugin: &str) -> i32 {
    let content = match fs::read_to_string(spec_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("specforge: error reading {}: {e}", spec_path.display());
            return 1;
        }
    };

    match add_plugin_to_spec(&content, plugin) {
        AddResult::Added(new_content) => {
            if let Err(e) = fs::write(spec_path, new_content) {
                eprintln!("specforge: error writing {}: {e}", spec_path.display());
                return 1;
            }
            eprintln!("specforge: added {}", plugin);
            0
        }
        AddResult::AlreadyPresent => {
            eprintln!("specforge: {} is already installed", plugin);
            0
        }
        AddResult::NoPluginsBlock => {
            eprintln!("specforge: could not find `plugins` block in specforge.spec");
            1
        }
    }
}

enum AddResult {
    Added(String),
    AlreadyPresent,
    NoPluginsBlock,
}

/// Insert a plugin into the spec file content's `plugins [...]` block.
fn add_plugin_to_spec(content: &str, plugin: &str) -> AddResult {
    // Check if plugin is already present
    if content.contains(&format!("\"{plugin}\"")) {
        return AddResult::AlreadyPresent;
    }

    // Find `plugins [` and the matching `]`
    let Some(plugins_start) = content.find("plugins [") else {
        return AddResult::NoPluginsBlock;
    };
    let bracket_open = plugins_start + "plugins [".len();

    // Find the closing `]` for this block
    let rest = &content[bracket_open..];
    let Some(relative_close) = rest.find(']') else {
        return AddResult::NoPluginsBlock;
    };
    let bracket_close = bracket_open + relative_close;

    let inside = &content[bracket_open..bracket_close];

    let new_content = if inside.trim().is_empty() {
        // Empty plugins list: `plugins []` → `plugins [\n    "plugin",\n  ]`
        let mut result = String::new();
        result.push_str(&content[..plugins_start]);
        result.push_str("plugins [\n");
        result.push_str(&format!("    \"{plugin}\",\n"));
        result.push_str("  ");
        result.push_str(&content[bracket_close..]);
        result
    } else {
        // Non-empty list: insert before closing `]`
        let mut result = String::new();
        result.push_str(&content[..bracket_close]);
        result.push_str(&format!("    \"{plugin}\",\n  "));
        result.push_str(&content[bracket_close..]);
        result
    };

    AddResult::Added(new_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_plugin_to_empty_list() {
        let input = r#"spec "test" {
  infix "TE"
  version "1.0"
  plugins []
}
"#;
        match add_plugin_to_spec(input, "@specforge/product") {
            AddResult::Added(output) => {
                assert!(output.contains("\"@specforge/product\""));
                assert!(output.contains("plugins ["));
                // Verify it's well-formed
                assert!(output.contains(r#"spec "test""#));
            }
            other => panic!("expected Added, got {:?}", result_name(&other)),
        }
    }

    #[test]
    fn add_plugin_to_existing_list() {
        let input = r#"spec "test" {
  infix "TE"
  version "1.0"
  plugins [
    "@specforge/product",
  ]
}
"#;
        match add_plugin_to_spec(input, "@specforge/governance") {
            AddResult::Added(output) => {
                assert!(output.contains("\"@specforge/product\""));
                assert!(output.contains("\"@specforge/governance\""));
            }
            other => panic!("expected Added, got {:?}", result_name(&other)),
        }
    }

    #[test]
    fn add_duplicate_is_noop() {
        let input = r#"spec "test" {
  infix "TE"
  version "1.0"
  plugins [
    "@specforge/product",
  ]
}
"#;
        assert!(matches!(
            add_plugin_to_spec(input, "@specforge/product"),
            AddResult::AlreadyPresent
        ));
    }

    #[test]
    fn add_unknown_plugin_fails() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("specforge.spec"),
            r#"spec "t" { infix "TT" version "1.0" plugins [] }"#,
        )
        .unwrap();

        let result = run(AddArgs {
            plugin: "@specforge/unknown".to_string(),
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 1);
    }

    #[test]
    fn add_no_spec_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        let result = run(AddArgs {
            plugin: "@specforge/product".to_string(),
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 1);
    }

    #[test]
    fn add_plugin_then_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let spec_path = dir.path().join("specforge.spec");
        fs::write(
            &spec_path,
            r#"spec "test" {
  infix "TE"
  version "1.0"
  plugins []
}
"#,
        )
        .unwrap();

        let result = run(AddArgs {
            plugin: "@specforge/product".to_string(),
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 0);

        let content = fs::read_to_string(&spec_path).unwrap();
        assert!(content.contains("\"@specforge/product\""));

        // Add second plugin
        let result = run(AddArgs {
            plugin: "@specforge/governance".to_string(),
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 0);

        let content = fs::read_to_string(&spec_path).unwrap();
        assert!(content.contains("\"@specforge/product\""));
        assert!(content.contains("\"@specforge/governance\""));
    }

    #[test]
    fn add_plugin_to_json() {
        let dir = tempfile::tempdir().unwrap();
        let json_path = dir.path().join("specforge.json");
        let config = specforge_common::SpecForgeJsonConfig::minimal("test");
        fs::write(&json_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let exit = add_to_json(&json_path, "@specforge/product");
        assert_eq!(exit, 0);

        let updated: specforge_common::SpecForgeJsonConfig =
            serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
        assert!(updated.plugins.contains(&"@specforge/product".to_string()));
    }

    #[test]
    fn add_duplicate_to_json_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let json_path = dir.path().join("specforge.json");
        let mut config = specforge_common::SpecForgeJsonConfig::minimal("test");
        config.plugins.push("@specforge/product".to_string());
        fs::write(&json_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let exit = add_to_json(&json_path, "@specforge/product");
        assert_eq!(exit, 0); // noop, not error
    }

    fn result_name(r: &AddResult) -> &'static str {
        match r {
            AddResult::Added(_) => "Added",
            AddResult::AlreadyPresent => "AlreadyPresent",
            AddResult::NoPluginsBlock => "NoPluginsBlock",
        }
    }
}
