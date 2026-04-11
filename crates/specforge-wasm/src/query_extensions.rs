use specforge_common::{Diagnostic, Severity};

/// A validated query extension ready for registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryExtension {
    pub extension_name: String,
    pub file_kind: QueryFileKind,
    pub pattern: String,
}

/// The kind of tree-sitter query file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryFileKind {
    Highlights,
    Locals,
    Injections,
    Custom(String),
}

/// Raw query extension input before validation.
#[derive(Debug, Clone)]
pub struct RawQueryExtension {
    pub file_kind: String,
    pub pattern: String,
}

/// Validate query extensions from raw input.
/// Returns (valid_extensions, warning_diagnostics).
/// Invalid patterns produce warnings but do not block loading.
pub fn validate_query_extensions(
    extension_name: &str,
    raw_extensions: &[RawQueryExtension],
) -> (Vec<QueryExtension>, Vec<Diagnostic>) {
    let mut valid = Vec::new();
    let mut warnings = Vec::new();

    for raw in raw_extensions {
        // Validate pattern: non-empty and no null bytes
        if raw.pattern.is_empty() {
            warnings.push(Diagnostic {
                code: "W031".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "extension '{}': query extension pattern for '{}' is empty",
                    extension_name, raw.file_kind
                ),
                span: None,
                suggestion: Some("provide a non-empty tree-sitter query pattern".to_string()),
            });
            continue;
        }

        if raw.pattern.contains('\0') {
            warnings.push(Diagnostic {
                code: "W031".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "extension '{}': query extension pattern for '{}' contains null bytes",
                    extension_name, raw.file_kind
                ),
                span: None,
                suggestion: Some("remove null bytes from the query pattern".to_string()),
            });
            continue;
        }

        let file_kind = match raw.file_kind.as_str() {
            "highlights" => QueryFileKind::Highlights,
            "locals" => QueryFileKind::Locals,
            "injections" => QueryFileKind::Injections,
            other => QueryFileKind::Custom(other.to_string()),
        };

        valid.push(QueryExtension {
            extension_name: extension_name.to_string(),
            file_kind,
            pattern: raw.pattern.clone(),
        });
    }

    (valid, warnings)
}

/// Compose tree-sitter query files from base queries plus extension contributions.
/// Base queries come first, followed by each extension's pattern in order.
pub fn compose_query_files(
    base_queries: &str,
    extensions: &[QueryExtension],
) -> String {
    let mut result = base_queries.to_string();
    for ext in extensions {
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }
        result.push_str(&ext.pattern);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // B:provide_extension_query_extensions — verify unit "valid query extension stored in extension registration"
    #[test]
    fn test_valid_query_extension_stored() {
        let raw = vec![RawQueryExtension {
            file_kind: "highlights".to_string(),
            pattern: "(identifier) @variable".to_string(),
        }];

        let (valid, warnings) = validate_query_extensions("@ext/test", &raw);
        assert!(warnings.is_empty());
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].extension_name, "@ext/test");
        assert_eq!(valid[0].file_kind, QueryFileKind::Highlights);
        assert_eq!(valid[0].pattern, "(identifier) @variable");
    }

    // B:provide_extension_query_extensions — verify unit "invalid query pattern produces warning diagnostic"
    #[test]
    fn test_invalid_query_pattern_produces_warning() {
        let raw = vec![
            RawQueryExtension {
                file_kind: "highlights".to_string(),
                pattern: String::new(), // empty
            },
            RawQueryExtension {
                file_kind: "locals".to_string(),
                pattern: "valid pattern".to_string(),
            },
        ];

        let (valid, warnings) = validate_query_extensions("@ext/test", &raw);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "W031");
        assert_eq!(warnings[0].severity, Severity::Warning);
        assert!(warnings[0].message.contains("empty"));
        // Valid one still passes
        assert_eq!(valid.len(), 1);
    }

    // B:provide_extension_query_extensions — verify unit "invalid pattern does not block extension loading"
    #[test]
    fn test_invalid_pattern_does_not_block_loading() {
        let raw = vec![
            RawQueryExtension {
                file_kind: "highlights".to_string(),
                pattern: "has\0null".to_string(), // invalid
            },
            RawQueryExtension {
                file_kind: "injections".to_string(),
                pattern: "(comment) @injection".to_string(), // valid
            },
        ];

        let (valid, warnings) = validate_query_extensions("@ext/test", &raw);
        // Invalid pattern produces warning but does not prevent valid ones from loading
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("null bytes"));
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].file_kind, QueryFileKind::Injections);
    }

    // B:provide_extension_query_extensions — verify unit "query extensions file kind parsing"
    #[test]
    fn test_query_extensions_file_kind_parsing() {
        let raw = vec![
            RawQueryExtension { file_kind: "highlights".to_string(), pattern: "a".to_string() },
            RawQueryExtension { file_kind: "locals".to_string(), pattern: "b".to_string() },
            RawQueryExtension { file_kind: "injections".to_string(), pattern: "c".to_string() },
            RawQueryExtension { file_kind: "folds".to_string(), pattern: "d".to_string() },
        ];

        let (valid, warnings) = validate_query_extensions("@ext/test", &raw);
        assert!(warnings.is_empty());
        assert_eq!(valid.len(), 4);
        assert_eq!(valid[0].file_kind, QueryFileKind::Highlights);
        assert_eq!(valid[1].file_kind, QueryFileKind::Locals);
        assert_eq!(valid[2].file_kind, QueryFileKind::Injections);
        assert_eq!(valid[3].file_kind, QueryFileKind::Custom("folds".to_string()));
    }

    // -- compose_query_files --

    fn make_ext(name: &str, pattern: &str) -> QueryExtension {
        QueryExtension {
            extension_name: name.to_string(),
            file_kind: QueryFileKind::Highlights,
            pattern: pattern.to_string(),
        }
    }

    // B:compose_query_files_from_extensions — verify unit "base queries come first in composed output"
    #[test]
    fn test_compose_base_queries_first() {
        let base = "(identifier) @variable";
        let exts = vec![make_ext("@ext/a", "(string) @string")];
        let result = compose_query_files(base, &exts);
        assert!(result.starts_with(base));
    }

    // B:compose_query_files_from_extensions — verify unit "extension query extensions appended in load order"
    #[test]
    fn test_compose_extensions_appended_in_order() {
        let base = ";; base";
        let exts = vec![
            make_ext("@ext/first", ";; first"),
            make_ext("@ext/second", ";; second"),
            make_ext("@ext/third", ";; third"),
        ];
        let result = compose_query_files(base, &exts);
        let first_pos = result.find(";; first").unwrap();
        let second_pos = result.find(";; second").unwrap();
        let third_pos = result.find(";; third").unwrap();
        assert!(first_pos < second_pos);
        assert!(second_pos < third_pos);
    }

    // B:compose_query_files_from_extensions — verify unit "#match? predicates work in composed query"
    #[test]
    fn test_compose_preserves_match_predicates() {
        let base = "(identifier) @variable";
        let exts = vec![make_ext(
            "@ext/a",
            "((identifier) @constant\n (#match? @constant \"^[A-Z]\"))",
        )];
        let result = compose_query_files(base, &exts);
        assert!(result.contains("#match?"));
        assert!(result.contains("@constant"));
    }

    // B:compose_query_files_from_extensions — verify unit "composition is deterministic across runs"
    #[test]
    fn test_compose_deterministic() {
        let base = "(identifier) @variable";
        let exts = vec![
            make_ext("@ext/a", "(string) @string"),
            make_ext("@ext/b", "(comment) @comment"),
        ];
        let r1 = compose_query_files(base, &exts);
        let r2 = compose_query_files(base, &exts);
        assert_eq!(r1, r2);
    }

    // B:compose_query_files_from_extensions — verify contract
    #[test]
    fn test_compose_query_files_contract() {
        // ensures: empty extensions returns base unchanged
        let base = "(identifier) @variable";
        let result = compose_query_files(base, &[]);
        assert_eq!(result, base);

        // ensures: base comes first, extensions appended in order
        let exts = vec![make_ext("@a", "ext_a"), make_ext("@b", "ext_b")];
        let result = compose_query_files(base, &exts);
        assert!(result.starts_with(base));
        let base_end = result.find("ext_a").unwrap();
        let ext_b = result.find("ext_b").unwrap();
        assert!(base_end < ext_b);

        // ensures: empty base with extensions still works
        let result = compose_query_files("", &exts);
        assert!(result.contains("ext_a"));
        assert!(result.contains("ext_b"));
    }
}
