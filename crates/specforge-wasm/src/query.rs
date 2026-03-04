use std::path::PathBuf;

use crate::manifest::PackageManifest;

/// Compose query extensions from multiple plugins into a single query string.
///
/// Base queries come first, followed by plugin extensions in load order.
/// Query files are resolved relative to each plugin's manifest directory.
pub fn compose_queries(
    base_query: &str,
    plugins: &[&PackageManifest],
) -> String {
    let mut composed = base_query.to_string();

    for plugin in plugins {
        for ext in &plugin.query_extensions {
            composed.push_str(&format!(
                "\n\n;; --- Query extension from {} ---\n",
                plugin.package
            ));
            let ext_path = plugin
                .manifest_path
                .parent()
                .map(|p| p.join(&ext.path))
                .unwrap_or_else(|| PathBuf::from(&ext.path));
            match std::fs::read_to_string(&ext_path) {
                Ok(content) => composed.push_str(&content),
                Err(_) => composed.push_str(&format!(
                    ";; WARNING: could not read {}\n",
                    ext.path
                )),
            }
        }
    }

    composed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{QueryExtension, SandboxPolicy};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_manifest_with_queries(
        package: &str,
        queries: Vec<QueryExtension>,
    ) -> PackageManifest {
        PackageManifest {
            package: package.to_string(),
            manifest_version: "1".to_string(),
            kind: crate::manifest::PluginKind::Plugin,
            contributes: crate::manifest::PackageContributions::default(),
            wasm: "plugin.wasm".to_string(),
            description: String::new(),
            version: "1.0.0".to_string(),
            enhancements: vec![],
            dynamic_edge_types: vec![],
            entity_kinds: vec![],
            provider: None,
            generator: None,
            sandbox: SandboxPolicy::default(),
            peer_dependencies: HashMap::new(),
            query_extensions: queries,
            manifest_path: PathBuf::new(),
            wasm_path: PathBuf::new(),
        }
    }

    #[test]
    fn compose_no_extensions() {
        let base = "(base_query)";
        let result = compose_queries(base, &[]);
        assert_eq!(result, base);
    }

    #[test]
    fn compose_reads_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let query_dir = dir.path().join("queries");
        std::fs::create_dir_all(&query_dir).unwrap();
        std::fs::write(query_dir.join("custom.scm"), "(custom_pattern)").unwrap();

        let manifest_path = dir.path().join("manifest.json");
        std::fs::write(&manifest_path, "{}").unwrap();

        let mut plugin = make_manifest_with_queries(
            "@specforge/test",
            vec![QueryExtension {
                path: "queries/custom.scm".to_string(),
                description: "Custom validation queries".to_string(),
            }],
        );
        plugin.manifest_path = manifest_path;

        let result = compose_queries("(base)", &[&plugin]);
        assert!(result.contains("(base)"));
        assert!(result.contains("(custom_pattern)"));
        assert!(result.contains("@specforge/test"));
    }

    #[test]
    fn compose_missing_file_warns() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join("manifest.json");
        std::fs::write(&manifest_path, "{}").unwrap();

        let mut plugin = make_manifest_with_queries(
            "@specforge/test",
            vec![QueryExtension {
                path: "nonexistent.scm".to_string(),
                description: "Missing query file".to_string(),
            }],
        );
        plugin.manifest_path = manifest_path;

        let result = compose_queries("(base)", &[&plugin]);
        assert!(result.contains("WARNING"));
        assert!(result.contains("nonexistent.scm"));
    }
}
