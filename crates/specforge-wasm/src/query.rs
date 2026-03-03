use crate::manifest::PackageManifest;

/// Compose query extensions from multiple plugins into a single query string.
///
/// Base queries come first, followed by plugin extensions in load order.
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
            // In a full implementation, we'd read the file at ext.path
            // For now, we just note the extension
            composed.push_str(&format!(";; Extension: {} ({})\n", ext.path, ext.description));
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
    fn compose_with_extensions() {
        let base = "(base_query)";
        let plugin = make_manifest_with_queries(
            "@specforge/test",
            vec![QueryExtension {
                path: "queries/custom.scm".to_string(),
                description: "Custom validation queries".to_string(),
            }],
        );
        let result = compose_queries(base, &[&plugin]);
        assert!(result.contains("(base_query)"));
        assert!(result.contains("@specforge/test"));
        assert!(result.contains("custom.scm"));
    }
}
