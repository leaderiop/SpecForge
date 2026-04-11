use specforge_common::{Diagnostic, Severity};
use specforge_registry::ManifestV2;
use std::path::{Path, PathBuf};

/// Parsed extension specifier from a project configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionSpecifier {
    Registry { name: String, version: String },
    Local { path: PathBuf },
    Git { url: String, rev: Option<String> },
}

/// A resolved extension: manifest + source location.
#[derive(Debug, Clone)]
pub struct ResolvedExtension {
    pub manifest: ManifestV2,
    pub source: ExtensionSpecifier,
    pub manifest_path: PathBuf,
}

/// Trait for resolving extension specifiers to loaded manifests.
pub trait ExtensionSource: Send + Sync {
    fn resolve(&self, spec: &ExtensionSpecifier) -> Result<ResolvedExtension, Diagnostic>;
}

/// Parse an extension specifier string into a structured type.
///
/// Formats:
/// - `name@version` → Registry { name, version }
/// - `./path/to/ext` or `/absolute/path` → Local { path }
/// - `git+https://...` optionally `#rev` → Git { url, rev }
pub fn parse_extension_specifier(input: &str) -> Result<ExtensionSpecifier, Diagnostic> {
    let input = input.trim();

    if input.is_empty() {
        return Err(Diagnostic {
            code: "E032".to_string(),
            severity: Severity::Error,
            message: "empty extension specifier".to_string(),
            span: None,
            suggestion: Some("provide a specifier like '@scope/name@1.0.0', './local/path', or 'git+https://...'".to_string()),
        });
    }

    // Git specifier
    if let Some(rest) = input.strip_prefix("git+") {
        let (url, rev) = if let Some(hash_pos) = rest.rfind('#') {
            let url = &rest[..hash_pos];
            let rev = &rest[hash_pos + 1..];
            (url.to_string(), Some(rev.to_string()))
        } else {
            (rest.to_string(), None)
        };
        return Ok(ExtensionSpecifier::Git { url, rev });
    }

    // Local path specifier
    if input.starts_with("./") || input.starts_with("../") || input.starts_with('/') {
        return Ok(ExtensionSpecifier::Local {
            path: PathBuf::from(input),
        });
    }

    // Registry specifier: name@version
    if let Some(at_pos) = input.rfind('@') {
        // Handle scoped packages like @scope/name@version
        // Don't split at position 0 (that's the scope prefix @)
        if at_pos > 0 {
            let name = &input[..at_pos];
            let version = &input[at_pos + 1..];
            if !version.is_empty() {
                return Ok(ExtensionSpecifier::Registry {
                    name: name.to_string(),
                    version: version.to_string(),
                });
            }
        }
    }

    Err(Diagnostic {
        code: "E032".to_string(),
        severity: Severity::Error,
        message: format!("invalid extension specifier: '{}'", input),
        span: None,
        suggestion: Some("use format: 'name@version', './local/path', or 'git+https://...'".to_string()),
    })
}

/// Discover extensions by walking an extensions directory.
/// Finds manifest.json files and attempts to parse them.
/// Invalid manifests are reported as diagnostics but don't prevent discovery.
pub fn discover_extensions(
    extensions_dir: &Path,
) -> (Vec<ResolvedExtension>, Vec<Diagnostic>) {
    let mut resolved = Vec::new();
    let mut diagnostics = Vec::new();

    let entries = match std::fs::read_dir(extensions_dir) {
        Ok(entries) => entries,
        Err(e) => {
            diagnostics.push(Diagnostic {
                code: "W029".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "cannot read extensions directory '{}': {}",
                    extensions_dir.display(),
                    e
                ),
                span: None,
                suggestion: None,
            });
            return (resolved, diagnostics);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }

        match std::fs::read_to_string(&manifest_path) {
            Ok(content) => match serde_json::from_str::<ManifestV2>(&content) {
                Ok(manifest) => {
                    resolved.push(ResolvedExtension {
                        source: ExtensionSpecifier::Local {
                            path: path.clone(),
                        },
                        manifest,
                        manifest_path,
                    });
                }
                Err(e) => {
                    diagnostics.push(Diagnostic {
                        code: "W029".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "invalid manifest at '{}': {}",
                            manifest_path.display(),
                            e
                        ),
                        span: None,
                        suggestion: None,
                    });
                }
            },
            Err(e) => {
                diagnostics.push(Diagnostic {
                    code: "W029".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "cannot read manifest at '{}': {}",
                        manifest_path.display(),
                        e
                    ),
                    span: None,
                    suggestion: None,
                });
            }
        }
    }

    (resolved, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    // -- parse_extension_specifier --

    // B:parse_extension_specifier — verify unit "parses name@version registry specifier"
    #[test]
    fn test_parses_registry_specifier() {
        let spec = parse_extension_specifier("@specforge/software@1.0.0").unwrap();
        assert_eq!(
            spec,
            ExtensionSpecifier::Registry {
                name: "@specforge/software".to_string(),
                version: "1.0.0".to_string(),
            }
        );
    }

    // B:parse_extension_specifier — verify unit "parses local path specifier"
    #[test]
    fn test_parses_local_path_specifier() {
        let spec = parse_extension_specifier("./extensions/custom").unwrap();
        assert_eq!(
            spec,
            ExtensionSpecifier::Local {
                path: PathBuf::from("./extensions/custom"),
            }
        );

        let abs = parse_extension_specifier("/absolute/path/ext").unwrap();
        assert_eq!(
            abs,
            ExtensionSpecifier::Local {
                path: PathBuf::from("/absolute/path/ext"),
            }
        );
    }

    // B:parse_extension_specifier — verify unit "parses git+https specifier with optional rev"
    #[test]
    fn test_parses_git_specifier() {
        let spec = parse_extension_specifier("git+https://github.com/org/ext").unwrap();
        assert_eq!(
            spec,
            ExtensionSpecifier::Git {
                url: "https://github.com/org/ext".to_string(),
                rev: None,
            }
        );

        let with_rev =
            parse_extension_specifier("git+https://github.com/org/ext#v1.0.0").unwrap();
        assert_eq!(
            with_rev,
            ExtensionSpecifier::Git {
                url: "https://github.com/org/ext".to_string(),
                rev: Some("v1.0.0".to_string()),
            }
        );
    }

    // B:parse_extension_specifier — verify unit "rejects invalid specifier"
    #[test]
    fn test_rejects_invalid_specifier() {
        let err = parse_extension_specifier("").unwrap_err();
        assert_eq!(err.code, "E032");

        let err2 = parse_extension_specifier("just-a-name").unwrap_err();
        assert_eq!(err2.code, "E032");
    }

    // -- discover_extensions --

    // B:discover_extensions — verify unit "discovers valid manifests in extensions dir"
    #[test]
    fn test_discovers_valid_manifests() {
        let dir = TempDir::new().unwrap();
        let ext_dir = dir.path().join("my-ext");
        std::fs::create_dir(&ext_dir).unwrap();

        let manifest = r#"{
            "name": "@test/my-ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "ext.wasm"
        }"#;
        let mut f = std::fs::File::create(ext_dir.join("manifest.json")).unwrap();
        f.write_all(manifest.as_bytes()).unwrap();

        let (resolved, diags) = discover_extensions(dir.path());
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].manifest.name, "@test/my-ext");
    }

    // B:discover_extensions — verify unit "skips invalid manifests with warning"
    #[test]
    fn test_skips_invalid_manifests_with_warning() {
        let dir = TempDir::new().unwrap();

        // Valid extension
        let valid_dir = dir.path().join("valid-ext");
        std::fs::create_dir(&valid_dir).unwrap();
        let valid_manifest = r#"{
            "name": "@test/valid",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "ext.wasm"
        }"#;
        std::fs::write(valid_dir.join("manifest.json"), valid_manifest).unwrap();

        // Invalid extension (bad JSON)
        let invalid_dir = dir.path().join("invalid-ext");
        std::fs::create_dir(&invalid_dir).unwrap();
        std::fs::write(invalid_dir.join("manifest.json"), "not valid json").unwrap();

        let (resolved, diags) = discover_extensions(dir.path());
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].manifest.name, "@test/valid");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W029");
        assert!(diags[0].message.contains("invalid manifest"));
    }
}
