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
    if !VALID_PLUGINS.contains(&args.plugin.as_str()) {
        eprintln!(
            "specforge: unknown plugin \"{}\"\nvalid plugins: {}",
            args.plugin,
            VALID_PLUGINS.join(", ")
        );
        return 1;
    }

    let spec_path = args.path.join("specforge.spec");
    if !spec_path.exists() {
        eprintln!(
            "specforge: no specforge.spec found in {}\nhint: run `specforge init` first",
            args.path.display()
        );
        return 1;
    }

    let content = match fs::read_to_string(&spec_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("specforge: error reading {}: {e}", spec_path.display());
            return 1;
        }
    };

    match add_plugin_to_spec(&content, &args.plugin) {
        AddResult::Added(new_content) => {
            if let Err(e) = fs::write(&spec_path, new_content) {
                eprintln!("specforge: error writing {}: {e}", spec_path.display());
                return 1;
            }
            eprintln!("specforge: added {}", args.plugin);
            0
        }
        AddResult::AlreadyPresent => {
            eprintln!("specforge: {} is already installed", args.plugin);
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

    fn result_name(r: &AddResult) -> &'static str {
        match r {
            AddResult::Added(_) => "Added",
            AddResult::AlreadyPresent => "AlreadyPresent",
            AddResult::NoPluginsBlock => "NoPluginsBlock",
        }
    }
}
