use std::path::{Path, PathBuf};

use specforge_common::{CompilerConfig, Diagnostic, FieldRegistry, SourceSpan, ValidationCode};

use crate::error::WasmError;
use crate::loader;
use crate::manifest::PackageManifest;
use crate::warm::WarmInstancePool;

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

/// Discover Wasm packages, validate peer deps, topologically sort, register enhancements,
/// and reconcile the warm instance pool.
///
/// This is the shared orchestration used by the CLI pipeline, watch mode, and LSP server
/// to avoid duplicating the discover → validate → sort → register → reconcile sequence.
pub fn discover_and_reconcile(
    pool: &mut WarmInstancePool,
    config: &CompilerConfig,
    project_root: &Path,
    registry: &mut FieldRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let wasm_specifiers = &config.wasm_package_specifiers;

    if wasm_specifiers.is_empty() {
        return diagnostics;
    }

    // Discover manifests from specifiers
    let (manifests, discover_errors) = discover_packages(wasm_specifiers, project_root);

    for err in discover_errors {
        diagnostics.push(Diagnostic::new(
            ValidationCode::E019,
            SourceSpan::file_start("specforge.json"),
            err.to_string(),
        ));
    }

    if manifests.is_empty() {
        return diagnostics;
    }

    // Validate peer dependencies
    let peer_errors = crate::peer_deps::validate_peer_dependencies(&manifests);
    for err in &peer_errors {
        let code = match err {
            WasmError::CycleDetected { .. } => ValidationCode::E021,
            _ => ValidationCode::E020,
        };
        diagnostics.push(Diagnostic::new(
            code,
            SourceSpan::file_start("specforge.json"),
            err.to_string(),
        ));
    }

    // Topological sort
    let sorted = match crate::peer_deps::topological_sort(&manifests) {
        Ok(order) => order
            .into_iter()
            .map(|i| manifests[i].clone())
            .collect::<Vec<_>>(),
        Err(err) => {
            diagnostics.push(Diagnostic::new(
                ValidationCode::E021,
                SourceSpan::file_start("specforge.json"),
                err.to_string(),
            ));
            manifests
        }
    };

    // Register enhancements from Wasm plugins in the field registry
    for m in &sorted {
        if !m.enhancements.is_empty() {
            registry.register_plugin(
                &m.package,
                &m.enhancements,
                &m.dynamic_edge_types,
                &config.enhancement_policy,
                &config.enhancement_overrides,
            );
        }
    }

    // Lock file: verify or create
    let lock_path = project_root.join("specforge.lock");
    if lock_path.exists() {
        if let Ok(lock) = crate::lock::LockFile::read(&lock_path) {
            for mismatch in lock.verify(&sorted) {
                diagnostics.push(Diagnostic::new(
                    ValidationCode::W025,
                    SourceSpan::file_start("specforge.lock"),
                    mismatch.to_string(),
                ));
            }
        }
    } else if !sorted.is_empty() {
        let lock = crate::lock::LockFile::from_manifests(&sorted);
        let _ = lock.write(&lock_path);
    }

    // Reconcile warm pool (only reloads if wasm hashes changed)
    let reconcile_errors = pool.reconcile(sorted);
    for err in reconcile_errors {
        diagnostics.push(Diagnostic::new(
            ValidationCode::E019,
            SourceSpan::file_start("specforge.json"),
            err.to_string(),
        ));
    }

    diagnostics
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
