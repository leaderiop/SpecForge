use std::collections::HashMap;
use std::fmt;

use specforge_common::{CompilerConfig, GenConfig};
use specforge_emitter::GeneratedFile;
use specforge_graph::SpecGraph;
use specforge_parser::SpecFile;

/// Context passed to every generator.
pub struct GenerateContext<'a> {
    pub graph: &'a SpecGraph,
    pub files: &'a [SpecFile],
    pub config: &'a CompilerConfig,
    pub gen_config: &'a GenConfig,
}

/// A typed error produced when an external plugin generator fails.
#[derive(Debug, Clone)]
pub enum PackageError {
    NotFound { package_name: String, message: String },
    ExecutionFailed { package_name: String, message: String, stderr: Option<String> },
    InvalidOutput { package_name: String, message: String },
    SpawnFailed { package_name: String, message: String },
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { package_name, message } => {
                write!(f, "package `{package_name}` not found: {message}")
            }
            Self::ExecutionFailed { package_name, message, stderr } => {
                write!(f, "package `{package_name}` failed: {message}")?;
                if let Some(stderr) = stderr {
                    write!(f, "\n  stderr: {stderr}")?;
                }
                Ok(())
            }
            Self::InvalidOutput { package_name, message } => {
                write!(f, "package `{package_name}` produced invalid output: {message}")
            }
            Self::SpawnFailed { package_name, message } => {
                write!(f, "package `{package_name}` spawn failed: {message}")
            }
        }
    }
}

/// The result of a generation run.
pub struct GenerateResult {
    pub files: Vec<GeneratedFile>,
    pub warnings: Vec<String>,
    pub errors: Vec<PackageError>,
    pub entity_checksums: HashMap<String, String>,
}

/// Trait implemented by all code generators (built-in and external).
pub trait Generator {
    fn name(&self) -> &str;
    fn generate(&self, ctx: &GenerateContext) -> GenerateResult;
}

/// Compute a deterministic SHA-256 checksum of an entity's fields.
/// Used for entity-level incremental tracking.
pub fn compute_entity_checksum(fields: &specforge_common::FieldMap) -> String {
    use sha2::{Digest, Sha256};
    // Serialize fields deterministically via serde_json (FieldMap preserves insertion order)
    let serialized = serde_json::to_string(fields).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(serialized.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Resolve a generator by name. Returns a built-in generator for known names,
/// or a `SubprocessGenerator` for unknown names.
pub fn resolve_generator(name: &str) -> Box<dyn Generator> {
    match name {
        "typescript" => Box::new(crate::typescript::TypeScriptGenerator),
        "json-schema" => Box::new(crate::json_schema::JsonSchemaGenerator),
        "rust" => Box::new(crate::rust::RustGenerator),
        _ => Box::new(crate::subprocess::SubprocessGenerator {
            name: name.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_builtin_typescript() {
        let generator = resolve_generator("typescript");
        assert_eq!(generator.name(), "typescript");
    }

    #[test]
    fn resolve_builtin_json_schema() {
        let generator = resolve_generator("json-schema");
        assert_eq!(generator.name(), "json-schema");
    }

    #[test]
    fn resolve_builtin_rust() {
        let generator = resolve_generator("rust");
        assert_eq!(generator.name(), "rust");
    }

    #[test]
    fn resolve_unknown_is_subprocess() {
        let generator = resolve_generator("custom-gen");
        assert_eq!(generator.name(), "custom-gen");
    }

    #[test]
    fn package_error_display_not_found() {
        let err = PackageError::NotFound {
            package_name: "foo".to_string(),
            message: "not on PATH".to_string(),
        };
        assert_eq!(err.to_string(), "package `foo` not found: not on PATH");
    }

    #[test]
    fn package_error_display_execution_failed_with_stderr() {
        let err = PackageError::ExecutionFailed {
            package_name: "bar".to_string(),
            message: "exit code 1".to_string(),
            stderr: Some("segfault".to_string()),
        };
        let s = err.to_string();
        assert!(s.contains("package `bar` failed: exit code 1"));
        assert!(s.contains("stderr: segfault"));
    }

    #[test]
    fn package_error_display_invalid_output() {
        let err = PackageError::InvalidOutput {
            package_name: "baz".to_string(),
            message: "expected array".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "package `baz` produced invalid output: expected array"
        );
    }

    #[test]
    fn package_error_display_spawn_failed() {
        let err = PackageError::SpawnFailed {
            package_name: "qux".to_string(),
            message: "permission denied".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "package `qux` spawn failed: permission denied"
        );
    }

    #[test]
    fn unchanged_entity_identical_checksum() {
        use specforge_common::{FieldMap, FieldValue};

        let mut fields = FieldMap::new();
        fields.insert("name", FieldValue::String("test".to_string()));
        fields.insert("risk", FieldValue::Enum("high".to_string()));

        let checksum1 = compute_entity_checksum(&fields);
        let checksum2 = compute_entity_checksum(&fields);

        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 64); // SHA-256 hex digest
    }

    #[test]
    fn changed_entity_different_checksum() {
        use specforge_common::{FieldMap, FieldValue};

        let mut fields1 = FieldMap::new();
        fields1.insert("name", FieldValue::String("original".to_string()));

        let mut fields2 = FieldMap::new();
        fields2.insert("name", FieldValue::String("modified".to_string()));

        let checksum1 = compute_entity_checksum(&fields1);
        let checksum2 = compute_entity_checksum(&fields2);

        assert_ne!(checksum1, checksum2);
    }
}
