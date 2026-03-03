use std::path::{Path, PathBuf};

use crate::error::WasmError;
use crate::loader;
use crate::manifest::PackageManifest;

/// Where a package was resolved from.
#[derive(Debug, Clone)]
pub enum PackageSource {
    /// Local filesystem path (e.g., `./packages/my-package`).
    Local(PathBuf),
    /// OCI/npm registry package name (stub — not yet implemented).
    Registry(String),
    /// Git repository (stub — not yet implemented).
    Git { url: String, ref_: Option<String> },
}

/// Resolve a package specifier to a source location.
///
/// Currently only supports local paths (starting with `./` or `/`).
/// Remote registry support is deferred.
pub fn resolve_package_source(specifier: &str) -> Result<PackageSource, WasmError> {
    if specifier.starts_with("./") || specifier.starts_with("../") || specifier.starts_with('/') {
        Ok(PackageSource::Local(PathBuf::from(specifier)))
    } else {
        Err(WasmError::WasmLoadFailed {
            package: specifier.to_string(),
            message: format!(
                "remote package resolution is not yet supported — use a local path (e.g., `./{specifier}`)"
            ),
        })
    }
}

/// Discover Wasm packages from the project configuration.
///
/// For each package specifier in the config, resolves the source and loads the manifest.
/// Returns successfully loaded manifests and any errors encountered.
pub fn discover_packages(
    plugin_specifiers: &[String],
    project_root: &Path,
) -> (Vec<PackageManifest>, Vec<WasmError>) {
    let mut manifests = Vec::new();
    let mut errors = Vec::new();

    for specifier in plugin_specifiers {
        match resolve_package_source(specifier) {
            Ok(PackageSource::Local(relative_path)) => {
                let plugin_dir = project_root.join(&relative_path);
                let manifest_path = plugin_dir.join("manifest.json");

                match loader::load_manifest(&manifest_path) {
                    Ok(manifest) => manifests.push(manifest),
                    Err(e) => errors.push(e),
                }
            }
            Ok(PackageSource::Registry(_)) | Ok(PackageSource::Git { .. }) => {
                errors.push(WasmError::ManifestNotFound {
                    path: std::path::PathBuf::from(specifier),
                });
            }
            Err(e) => errors.push(e),
        }
    }

    (manifests, errors)
}

/// Check if a package specifier looks like a local path.
pub fn is_local_path(specifier: &str) -> bool {
    specifier.starts_with("./") || specifier.starts_with("../") || specifier.starts_with('/')
}

/// Check if a package specifier is a known built-in module.
pub fn is_builtin_package(specifier: &str) -> bool {
    matches!(specifier, "@specforge/product" | "@specforge/governance")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_local_path() {
        let source = resolve_package_source("./plugins/my-plugin").unwrap();
        assert!(matches!(source, PackageSource::Local(_)));
    }

    #[test]
    fn resolve_relative_parent_path() {
        let source = resolve_package_source("../shared-plugin").unwrap();
        assert!(matches!(source, PackageSource::Local(_)));
    }

    #[test]
    fn resolve_absolute_path() {
        let source = resolve_package_source("/opt/plugins/test").unwrap();
        assert!(matches!(source, PackageSource::Local(_)));
    }

    #[test]
    fn resolve_remote_unsupported() {
        let result = resolve_package_source("@specforge/hexagonal");
        assert!(result.is_err());
    }

    #[test]
    fn discover_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let (manifests, errors) = discover_packages(&[], dir.path());
        assert!(manifests.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn discover_missing_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let specifiers = vec!["./nonexistent-plugin".to_string()];
        let (manifests, errors) = discover_packages(&specifiers, dir.path());
        assert!(manifests.is_empty());
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn is_local_path_checks() {
        assert!(is_local_path("./plugins/test"));
        assert!(is_local_path("../shared"));
        assert!(is_local_path("/absolute/path"));
        assert!(!is_local_path("@specforge/product"));
        assert!(!is_local_path("my-plugin"));
    }

    #[test]
    fn is_builtin_checks() {
        assert!(is_builtin_package("@specforge/product"));
        assert!(is_builtin_package("@specforge/governance"));
        assert!(!is_builtin_package("@specforge/hexagonal"));
        assert!(!is_builtin_package("./local-plugin"));
    }
}
