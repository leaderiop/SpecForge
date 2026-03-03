use clap::Args;
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

    /// Directory to initialize in
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Run the init command. Returns exit code.
pub fn run(args: InitArgs) -> i32 {
    let dir = &args.path;

    // Create spec directory structure
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("specforge: error creating directory: {e}");
        return 1;
    }

    let spec_path = dir.join("specforge.spec");
    if spec_path.exists() {
        eprintln!("specforge: {} already exists", spec_path.display());
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

    let content = generate_spec(&name, namespace.as_deref(), &plugins);

    if let Err(e) = fs::write(&spec_path, content) {
        eprintln!("specforge: error writing {}: {e}", spec_path.display());
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

/// Generate the specforge.spec file content.
pub fn generate_spec(name: &str, namespace: Option<&str>, plugins: &[String]) -> String {
    let namespace_line = namespace
        .map(|ns| format!("  namespace \"{ns}\"\n"))
        .unwrap_or_default();

    if plugins.is_empty() {
        format!(
            "spec \"{name}\" {{\n{namespace_line}  version \"1.0\"\n  plugins []\n}}\n"
        )
    } else {
        let plugin_lines: String = plugins
            .iter()
            .map(|p| format!("    \"{p}\",\n"))
            .collect();
        format!(
            "spec \"{name}\" {{\n{namespace_line}  version \"1.0\"\n  plugins [\n{plugin_lines}  ]\n}}\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_spec_file() {
        let dir = tempfile::tempdir().unwrap();
        let result = run(InitArgs {
            name: Some("testproject".to_string()),
            namespace: None,
            plugins: vec![],
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 0);

        let spec_path = dir.path().join("specforge.spec");
        assert!(spec_path.exists());

        let content = fs::read_to_string(spec_path).unwrap();
        assert!(content.contains("testproject"));
        assert!(content.contains("plugins []"));
    }

    #[test]
    fn init_fails_if_exists() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("specforge.spec"), "existing").unwrap();

        let result = run(InitArgs {
            name: Some("test".to_string()),
            namespace: None,
            plugins: vec![],
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
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 0);

        let content = fs::read_to_string(dir.path().join("specforge.spec")).unwrap();
        assert!(content.contains("\"@specforge/product\""));
        assert!(content.contains("\"@specforge/governance\""));
    }

    #[test]
    fn init_with_namespace() {
        let dir = tempfile::tempdir().unwrap();
        let result = run(InitArgs {
            name: Some("auth-service".to_string()),
            namespace: Some("@auth-service".to_string()),
            plugins: vec![],
            path: dir.path().to_path_buf(),
        });
        assert_eq!(result, 0);

        let content = fs::read_to_string(dir.path().join("specforge.spec")).unwrap();
        assert!(content.contains("auth-service"));
        assert!(content.contains("namespace \"@auth-service\""));
    }

    #[test]
    fn generate_spec_without_plugins() {
        let content = generate_spec("test", None, &[]);
        assert!(content.contains("plugins []"));
        assert!(content.contains(r#"spec "test""#));
        assert!(!content.contains("namespace"));
    }

    #[test]
    fn generate_spec_with_namespace() {
        let content = generate_spec("test", Some("@test"), &[]);
        assert!(content.contains(r#"namespace "@test""#));
    }

    #[test]
    fn generate_spec_with_plugins() {
        let plugins = vec![
            "@specforge/product".to_string(),
            "@specforge/governance".to_string(),
        ];
        let content = generate_spec("test", None, &plugins);
        assert!(content.contains("\"@specforge/product\""));
        assert!(content.contains("\"@specforge/governance\""));
        // Must be well-formed (plugins on separate lines)
        assert!(content.contains("plugins ["));
        assert!(content.contains("  ]"));
    }
}
