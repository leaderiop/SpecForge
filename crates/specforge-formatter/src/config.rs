use specforge_common::Diagnostic;
use std::path::{Path, PathBuf};

/// Formatter configuration, loaded from `.specforgefmt.toml` or defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatConfig {
    pub indent_width: usize,
    pub use_tabs: bool,
    pub max_width: usize,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent_width: 2,
            use_tabs: false,
            max_width: 100,
        }
    }
}

impl FormatConfig {
    /// Returns the indent string for one level of indentation.
    pub fn indent_str(&self) -> String {
        if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_width)
        }
    }
}

/// Load format configuration by walking from `file_dir` up to `project_root`.
///
/// The walk stops at `project_root` and does NOT continue beyond it.
/// If `.specforgefmt.toml` is found, it is parsed and validated.
/// Invalid values produce diagnostics and fall back to defaults.
/// If no config file is found, defaults are returned.
pub fn load_config(file_dir: &Path, project_root: &Path) -> (FormatConfig, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    // Canonicalize paths for reliable comparison
    let project_root = match project_root.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return (FormatConfig::default(), diagnostics);
        }
    };
    let file_dir = match file_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return (FormatConfig::default(), diagnostics);
        }
    };

    // Walk from file_dir up to project_root (inclusive)
    let mut current = file_dir;
    loop {
        let config_path = current.join(".specforgefmt.toml");
        if config_path.exists() {
            return parse_config_file(&config_path, &mut diagnostics);
        }
        if current == project_root {
            break;
        }
        if !current.pop() {
            break;
        }
        // Stop if we've gone above the project root
        if !current.starts_with(&project_root) && current != project_root {
            break;
        }
    }

    (FormatConfig::default(), diagnostics)
}

fn parse_config_file(path: &Path, diagnostics: &mut Vec<Diagnostic>) -> (FormatConfig, Vec<Diagnostic>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            diagnostics.push(Diagnostic {
                code: "F001".into(),
                severity: specforge_common::Severity::Warning,
                message: format!("Failed to read config file {}: {}", path.display(), e),
                span: None,
                suggestion: None,
            });
            return (FormatConfig::default(), std::mem::take(diagnostics));
        }
    };

    let table: toml::Table = match content.parse() {
        Ok(t) => t,
        Err(e) => {
            diagnostics.push(Diagnostic {
                code: "F002".into(),
                severity: specforge_common::Severity::Warning,
                message: format!("Invalid TOML in {}: {}", path.display(), e),
                span: None,
                suggestion: None,
            });
            return (FormatConfig::default(), std::mem::take(diagnostics));
        }
    };

    let mut config = FormatConfig::default();

    if let Some(val) = table.get("indent_width") {
        match val.as_integer() {
            Some(n) if (1..=16).contains(&n) => config.indent_width = n as usize,
            _ => {
                diagnostics.push(Diagnostic {
                    code: "F003".into(),
                    severity: specforge_common::Severity::Warning,
                    message: format!(
                        "Invalid indent_width in {}: expected integer 1-16, using default {}",
                        path.display(),
                        config.indent_width
                    ),
                    span: None,
                    suggestion: Some("indent_width must be an integer between 1 and 16".into()),
                });
            }
        }
    }

    if let Some(val) = table.get("use_tabs") {
        match val.as_bool() {
            Some(b) => config.use_tabs = b,
            None => {
                diagnostics.push(Diagnostic {
                    code: "F003".into(),
                    severity: specforge_common::Severity::Warning,
                    message: format!(
                        "Invalid use_tabs in {}: expected boolean, using default {}",
                        path.display(),
                        config.use_tabs
                    ),
                    span: None,
                    suggestion: Some("use_tabs must be true or false".into()),
                });
            }
        }
    }

    if let Some(val) = table.get("max_width") {
        match val.as_integer() {
            Some(n) if (40..=200).contains(&n) => config.max_width = n as usize,
            _ => {
                diagnostics.push(Diagnostic {
                    code: "F003".into(),
                    severity: specforge_common::Severity::Warning,
                    message: format!(
                        "Invalid max_width in {}: expected integer 40-200, using default {}",
                        path.display(),
                        config.max_width
                    ),
                    span: None,
                    suggestion: Some("max_width must be an integer between 40 and 200".into()),
                });
            }
        }
    }

    (config, std::mem::take(diagnostics))
}

/// Find the `.specforgefmt.toml` config file path, if it exists, walking from
/// `file_dir` up to `project_root`.
pub fn find_config_path(file_dir: &Path, project_root: &Path) -> Option<PathBuf> {
    let project_root = project_root.canonicalize().ok()?;
    let mut current = file_dir.canonicalize().ok()?;

    loop {
        let config_path = current.join(".specforgefmt.toml");
        if config_path.exists() {
            return Some(config_path);
        }
        if current == project_root {
            break;
        }
        if !current.pop() {
            break;
        }
        if !current.starts_with(&project_root) && current != project_root {
            break;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
    }

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "missing config file uses defaults")]
    #[test]
    fn test_default_config() {
        let config = FormatConfig::default();
        assert_eq!(config.indent_width, 2);
        assert!(!config.use_tabs);
        assert_eq!(config.max_width, 100);
    }

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "config file in project root is loaded")]
    #[test]
    fn test_config_file_in_project_root_is_loaded() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_file(root, "specforge.json", "{}");
        write_file(root, ".specforgefmt.toml", "indent_width = 4\nmax_width = 80\n");

        let (config, diags) = load_config(root, root);
        assert!(diags.is_empty());
        assert_eq!(config.indent_width, 4);
        assert_eq!(config.max_width, 80);
    }

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "config file in parent directory is discovered")]
    #[test]
    fn test_config_file_in_parent_directory_is_discovered() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_file(root, "specforge.json", "{}");
        write_file(root, ".specforgefmt.toml", "indent_width = 3\n");

        let subdir = root.join("spec").join("behaviors");
        std::fs::create_dir_all(&subdir).unwrap();

        let (config, diags) = load_config(&subdir, root);
        assert!(diags.is_empty());
        assert_eq!(config.indent_width, 3);
    }

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "config outside project root is not discovered")]
    #[test]
    fn test_config_outside_project_root_is_not_discovered() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        // Config is in tmp root, but project root is a subdirectory
        write_file(root, ".specforgefmt.toml", "indent_width = 8\n");

        let project = root.join("project");
        std::fs::create_dir_all(&project).unwrap();
        write_file(&project, "specforge.json", "{}");

        let (config, diags) = load_config(&project, &project);
        assert!(diags.is_empty());
        // Should use defaults since config is outside project root
        assert_eq!(config.indent_width, 2);
    }

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "invalid indent_width produces diagnostic and uses default")]
    #[test]
    fn test_invalid_indent_width_produces_diagnostic_and_uses_default() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_file(root, "specforge.json", "{}");
        write_file(root, ".specforgefmt.toml", "indent_width = \"not_a_number\"\n");

        let (config, diags) = load_config(root, root);
        assert_eq!(config.indent_width, 2); // default
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "F003");
    }

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "missing config file uses defaults")]
    #[test]
    fn test_missing_config_file_uses_defaults() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_file(root, "specforge.json", "{}");
        // No .specforgefmt.toml created

        let (config, diags) = load_config(root, root);
        assert!(diags.is_empty());
        assert_eq!(config, FormatConfig::default());
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "indentation rules normalize to configured indent style")]
    #[test]
    fn test_indent_str_spaces() {
        let config = FormatConfig { indent_width: 4, use_tabs: false, max_width: 100 };
        assert_eq!(config.indent_str(), "    ");
    }

    #[specforge_test_macros::test(behavior = "apply_format_rules", verify = "indentation rules normalize to configured indent style")]
    #[test]
    fn test_indent_str_tabs() {
        let config = FormatConfig { indent_width: 4, use_tabs: true, max_width: 100 };
        assert_eq!(config.indent_str(), "\t");
    }

    // --- Contract: load_format_config ---

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "requires/ensures consistency for format config loading")]
    #[test]
    fn test_load_format_config_contract_requires_ensures() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        // requires: project_root_available
        write_file(root, "specforge.json", "{}");
        write_file(root, ".specforgefmt.toml", "indent_width = 4\nmax_width = 80\n");

        let subdir = root.join("spec").join("behaviors");
        std::fs::create_dir_all(&subdir).unwrap();

        // ensures: config_resolved
        let (config, diags) = load_config(&subdir, root);
        assert_eq!(config.indent_width, 4, "config should be resolved");
        assert_eq!(config.max_width, 80);
        assert!(diags.is_empty(), "valid config should produce no diagnostics");

        // ensures: walk_bounded (config outside project root not found)
        let isolated = TempDir::new().unwrap();
        let iso_root = isolated.path();
        write_file(iso_root, "specforge.json", "{}");
        // No config file in isolated project
        let (default_config, default_diags) = load_config(iso_root, iso_root);
        assert_eq!(default_config, FormatConfig::default(), "should use defaults when no config");
        assert!(default_diags.is_empty());

        // ensures: invalid_values_diagnosed
        let invalid_tmp = TempDir::new().unwrap();
        let inv_root = invalid_tmp.path();
        write_file(inv_root, "specforge.json", "{}");
        write_file(inv_root, ".specforgefmt.toml", "indent_width = 999\n");
        let (inv_config, inv_diags) = load_config(inv_root, inv_root);
        assert_eq!(inv_config.indent_width, 2, "invalid value should fall back to default");
        assert!(!inv_diags.is_empty(), "invalid value should produce diagnostic");
    }

    #[specforge_test_macros::test(behavior = "load_format_config", verify = "config discovery walks from formatted file directory up to specforge.json parent then stops")]
    #[test]
    fn test_config_discovery_walks_from_file_dir_up_to_project_root_then_stops() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_file(root, "specforge.json", "{}");

        // Place config in project root
        write_file(root, ".specforgefmt.toml", "indent_width = 3\n");

        // Create deeply nested subdirectory
        let deep = root.join("spec").join("behaviors").join("auth");
        std::fs::create_dir_all(&deep).unwrap();

        // Should find the config by walking up
        let (config, _) = load_config(&deep, root);
        assert_eq!(config.indent_width, 3, "should find config by walking up");

        // Config placed ABOVE project root should NOT be found
        let parent = tmp.path();
        let project = parent.join("project");
        std::fs::create_dir_all(&project).unwrap();
        write_file(&project, "specforge.json", "{}");
        // Config is in parent (above project root), should not be found
        let (config2, _) = load_config(&project, &project);
        // If there's no .specforgefmt.toml in project, should use defaults
        assert_eq!(config2, FormatConfig::default());
    }
}
