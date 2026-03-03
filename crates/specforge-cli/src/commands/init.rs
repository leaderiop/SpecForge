use clap::Args;
use specforge_common::SpecForgeJsonConfig;
use std::fs;
use std::path::PathBuf;

/// Known plugins that can be selected during init.
const KNOWN_PLUGINS: &[(&str, &str)] = &[
    (
        "@specforge/product",
        "Capabilities, deliverables, roadmaps, libraries, glossary",
    ),
    (
        "@specforge/governance",
        "Decisions (ADRs), constraints, failure modes (FMEA)",
    ),
];

#[derive(Args)]
pub struct InitArgs {
    /// Project name (interactive prompt if omitted)
    #[arg(long)]
    pub name: Option<String>,

    /// Optional namespace for cross-project references (e.g., `@auth-service`)
    #[arg(long)]
    pub namespace: Option<String>,

    /// Plugins to install (non-interactive; may be repeated)
    #[arg(long = "plugin")]
    pub plugins: Vec<String>,

    /// Path to .spec files relative to project root (default ".")
    #[arg(long, default_value = ".")]
    pub spec_root: String,

    /// Directory to initialize in
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Run the init command. Returns exit code.
pub fn run(args: InitArgs) -> i32 {
    let dir = &args.path;

    // Create directory structure
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("specforge: error creating directory: {e}");
        return 1;
    }

    let json_path = dir.join("specforge.json");
    if json_path.exists() {
        eprintln!("specforge: {} already exists", json_path.display());
        return 1;
    }

    // Determine whether we're interactive or non-interactive.
    // Non-interactive when --name is supplied.
    let is_interactive = args.name.is_none();

    let (name, namespace, plugins) = if is_interactive {
        match prompt_interactive(&args) {
            Ok(values) => values,
            Err(e) => {
                eprintln!("specforge: {e}");
                return 1;
            }
        }
    } else {
        let name = args.name.unwrap();
        (name, args.namespace, args.plugins)
    };

    let content = generate_json_config(&name, namespace.as_deref(), &plugins, &args.spec_root);

    if let Err(e) = fs::write(&json_path, content) {
        eprintln!("specforge: error writing {}: {e}", json_path.display());
        return 1;
    }

    eprintln!(
        "specforge: initialized project '{}' in {}",
        name,
        dir.display()
    );
    0
}

/// Prompt the user interactively for name and plugins.
fn prompt_interactive(args: &InitArgs) -> Result<(String, Option<String>, Vec<String>), String> {
    let name = match &args.name {
        Some(n) => n.clone(),
        None => {
            let default_name = std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "myproject".to_string());

            inquire::Text::new("Project name:")
                .with_default(&default_name)
                .prompt()
                .map_err(|e| format!("prompt cancelled: {e}"))?
        }
    };

    let namespace = match &args.namespace {
        Some(ns) => Some(ns.clone()),
        None => {
            let value = inquire::Text::new("Namespace (optional, e.g. @auth-service):")
                .prompt_skippable()
                .map_err(|e| format!("prompt cancelled: {e}"))?
                .filter(|s| !s.is_empty());
            value
        }
    };

    let plugins = if args.plugins.is_empty() {
        let options: Vec<String> = KNOWN_PLUGINS
            .iter()
            .map(|(name, desc)| format!("{name} — {desc}"))
            .collect();

        let selected = inquire::MultiSelect::new("Plugins to install:", options)
            .prompt()
            .map_err(|e| format!("prompt cancelled: {e}"))?;

        selected
            .iter()
            .filter_map(|s| {
                KNOWN_PLUGINS
                    .iter()
                    .find(|(name, desc)| *s == format!("{name} — {desc}"))
                    .map(|(name, _)| name.to_string())
            })
            .collect()
    } else {
        args.plugins.clone()
    };

    Ok((name, namespace, plugins))
}

/// Generate the specforge.json content.
pub fn generate_json_config(
    name: &str,
    namespace: Option<&str>,
    plugins: &[String],
    spec_root: &str,
) -> String {
    let mut config = SpecForgeJsonConfig::minimal(name);
    config.spec_root = spec_root.to_string();

    if let Some(ns) = namespace {
        config.namespace = Some(ns.to_string());
    }

    config.plugins = plugins.to_vec();

    serde_json::to_string_pretty(&config).unwrap() + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_json_config() {
        let dir = tempfile::tempdir().unwrap();
        let result = run(InitArgs {
            name: Some("testproject".to_string()),
            namespace: None,
            plugins: vec![],
            spec_root: ".".to_string(),
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 0);

        let json_path = dir.path().join("specforge.json");
        assert!(json_path.exists());

        let content = fs::read_to_string(&json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["name"], "testproject");
        assert_eq!(parsed["version"], "1.0");
        assert_eq!(parsed["spec_root"], ".");
    }

    #[test]
    fn init_fails_if_exists() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("specforge.json"), "{}").unwrap();

        let result = run(InitArgs {
            name: Some("test".to_string()),
            namespace: None,
            plugins: vec![],
            spec_root: ".".to_string(),
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 1);
    }

    #[test]
    fn init_with_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let result = run(InitArgs {
            name: Some("myapp".to_string()),
            namespace: None,
            plugins: vec![
                "@specforge/product".to_string(),
                "@specforge/governance".to_string(),
            ],
            spec_root: ".".to_string(),
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 0);

        let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
        assert!(content.contains("@specforge/product"));
        assert!(content.contains("@specforge/governance"));
    }

    #[test]
    fn init_with_namespace() {
        let dir = tempfile::tempdir().unwrap();
        let result = run(InitArgs {
            name: Some("auth-service".to_string()),
            namespace: Some("@auth-service".to_string()),
            plugins: vec![],
            spec_root: ".".to_string(),
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 0);

        let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["name"], "auth-service");
        assert_eq!(parsed["namespace"], "@auth-service");
    }

    #[test]
    fn generate_json_without_plugins() {
        let content = generate_json_config("test", None, &[], ".");
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["name"], "test");
        assert_eq!(parsed["version"], "1.0");
        assert_eq!(parsed["plugins"], serde_json::json!([]));
        assert!(parsed.get("namespace").is_none());
    }

    #[test]
    fn generate_json_with_namespace() {
        let content = generate_json_config("test", Some("@test"), &[], ".");
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["namespace"], "@test");
    }

    #[test]
    fn generate_json_with_plugins() {
        let plugins = vec![
            "@specforge/product".to_string(),
            "@specforge/governance".to_string(),
        ];
        let content = generate_json_config("test", None, &plugins, ".");
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let plugin_arr = parsed["plugins"].as_array().unwrap();
        assert_eq!(plugin_arr.len(), 2);
        assert_eq!(plugin_arr[0], "@specforge/product");
        assert_eq!(plugin_arr[1], "@specforge/governance");
    }

    #[test]
    fn generate_json_with_spec_root() {
        let content = generate_json_config("test", None, &[], "./spec");
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["spec_root"], "./spec");
    }

    #[test]
    fn generated_json_is_valid_config() {
        let content = generate_json_config(
            "full-test",
            Some("@ns"),
            &["@specforge/product".to_string()],
            "./specs",
        );
        let config: SpecForgeJsonConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(config.name, "full-test");
        assert_eq!(config.namespace.as_deref(), Some("@ns"));
        assert_eq!(config.spec_root, "./specs");
        assert_eq!(config.plugins.len(), 1);
    }
}
