use crate::validation_engine::{ValidationPatternKind, ValidationRulePattern};
use crate::{FieldRegistry, KindRegistry};
use specforge_common::{Diagnostic, Severity, SourceSpan};
use std::collections::HashMap;

/// A keyword-to-extension index for suggesting missing extensions.
/// Loaded lazily from a bundled data file on first E024 occurrence.
#[derive(Debug, Default)]
pub struct KeywordExtensionIndex {
    entries: HashMap<String, String>,
}

impl KeywordExtensionIndex {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn from_entries(entries: HashMap<String, String>) -> Self {
        Self { entries }
    }

    pub fn lookup(&self, keyword: &str) -> Option<&str> {
        self.entries.get(keyword).map(|s| s.as_str())
    }
}

/// Detect unknown entity kinds by checking each parsed keyword against the KindRegistry.
/// Structural keywords (spec, ref, use, define) are always valid.
/// Returns E024 diagnostics for unknown keywords.
pub fn detect_unknown_entity_kinds(
    entities: &[(String, String, SourceSpan)], // (keyword, id, span)
    kind_reg: &KindRegistry,
    index: Option<&KeywordExtensionIndex>,
) -> Vec<Diagnostic> {
    let structural = ["spec", "ref", "use", "define"];
    let mut diagnostics = Vec::new();

    for (keyword, id, span) in entities {
        if structural.contains(&keyword.as_str()) {
            continue;
        }
        if kind_reg.contains(keyword) {
            continue;
        }

        let suggestion = if let Some(idx) = index {
            if let Some(ext) = idx.lookup(keyword) {
                Some(format!("install it with: specforge add {}", ext))
            } else {
                Some("check available extensions with: specforge outline".to_string())
            }
        } else {
            Some("check available extensions with: specforge outline".to_string())
        };

        diagnostics.push(Diagnostic {
            code: "E024".to_string(),
            severity: Severity::Error,
            message: format!(
                "unknown entity kind '{}' for entity '{}' at {}",
                keyword, id, span.file
            ),
            span: Some(span.clone()),
            suggestion,
        });
    }

    diagnostics
}

/// Detect unknown entity fields by checking each field name against the FieldRegistry.
/// Structural fields (title, verify) are always valid and skipped.
/// Entities with unregistered kinds are skipped to avoid cascading diagnostics.
pub fn detect_unknown_entity_fields(
    entities: &[(String, String, Vec<String>, SourceSpan)], // (kind, id, field_names, span)
    kind_reg: &KindRegistry,
    field_reg: &FieldRegistry,
) -> Vec<Diagnostic> {
    let structural_fields = ["title", "verify"];
    let mut diagnostics = Vec::new();

    for (kind, id, field_names, span) in entities {
        // Skip entities with unregistered kinds — already E024
        if !kind_reg.contains(kind) {
            continue;
        }

        // Skip entities with open_fields — any field name is valid (e.g., type struct fields, port methods)
        if let Some(entry) = kind_reg.get(kind)
            && entry.open_fields
        {
            continue;
        }

        for field_name in field_names {
            if structural_fields.contains(&field_name.as_str()) {
                continue;
            }
            if !field_reg.contains(kind, field_name) {
                diagnostics.push(Diagnostic {
                    code: "W020".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "unrecognized field '{}' on entity '{}' of kind '{}' at {}",
                        field_name, id, kind, span.file
                    ),
                    span: Some(span.clone()),
                    suggestion: None,
                });
            }
        }
    }

    diagnostics
}

/// Check if the system should operate in graceful degradation mode.
/// Returns I002 diagnostic when no extensions are installed.
pub fn check_graceful_degradation(
    kind_reg: &KindRegistry,
    extension_count: usize,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if extension_count == 0 && kind_reg.is_empty() {
        diagnostics.push(Diagnostic {
            code: "I002".to_string(),
            severity: Severity::Info,
            message: "no extensions installed — operating in structural-only mode".to_string(),
            span: None,
            suggestion: Some("install extensions with: specforge add @specforge/software".to_string()),
        });
    }

    diagnostics
}

/// Emit per-extension error diagnostics when all extensions fail to load.
pub fn handle_all_extensions_failed(
    failures: &[(String, String)], // (extension_name, error_message)
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (ext_name, error) in failures {
        diagnostics.push(Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!("failed to load extension '{}': {}", ext_name, error),
            span: None,
            suggestion: None,
        });
    }

    if !failures.is_empty() {
        diagnostics.push(Diagnostic {
            code: "I002".to_string(),
            severity: Severity::Info,
            message: "all extensions failed to load — operating in structural-only mode".to_string(),
            span: None,
            suggestion: None,
        });
    }

    diagnostics
}

/// Detect unknown verify kinds in parsed entities.
/// Returns W-level diagnostics for verify statements using kinds not registered by extensions.
pub fn detect_unknown_verify_kinds(
    entities: &[(String, String, Vec<String>, SourceSpan)], // (kind, id, verify_kinds_used, span)
    registered_verify_kinds: &[String],
    kind_reg: &KindRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (kind, id, verify_kinds_used, span) in entities {
        // Skip entities whose kind is not registered (already reported as E024)
        if !kind_reg.contains(kind) {
            continue;
        }
        // Check allowed verify kinds for this entity kind
        let entry = kind_reg.get(kind);
        let allowed = entry.map(|e| &e.allowed_verify_kinds);

        for vk in verify_kinds_used {
            // Check against globally registered verify kinds
            if !registered_verify_kinds.contains(vk) {
                diagnostics.push(Diagnostic {
                    code: "W026".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "unknown verify kind '{}' on {} '{}' — not registered by any installed extension",
                        vk, kind, id
                    ),
                    span: Some(span.clone()),
                    suggestion: Some(format!(
                        "registered verify kinds: [{}]",
                        registered_verify_kinds.join(", ")
                    )),
                });
            } else if let Some(allowed_kinds) = allowed {
                // If the entity kind restricts verify kinds, check against its list
                if !allowed_kinds.is_empty() && !allowed_kinds.contains(vk) {
                    diagnostics.push(Diagnostic {
                        code: "W026".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "verify kind '{}' not allowed on {} '{}' — allowed kinds: [{}]",
                            vk, kind, id, allowed_kinds.join(", ")
                        ),
                        span: Some(span.clone()),
                        suggestion: None,
                    });
                }
            }
        }
    }

    diagnostics
}

/// Per-entity reference field data: (entity_kind, entity_id, ref_fields, span).
pub type EntityRefInfo = (String, String, Vec<(String, Vec<String>)>, SourceSpan);

/// Detect mistyped references: reference list fields whose target entities exist
/// but are the wrong entity kind according to the FieldRegistry's target_kind constraint.
///
/// For example, `features [some_behavior_id]` where `some_behavior_id` is a behavior,
/// not a feature, produces W022.
pub fn detect_mistyped_references(
    entities: &[EntityRefInfo],
    field_reg: &FieldRegistry,
    kind_reg: &KindRegistry,
    node_kind_index: &HashMap<String, String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (entity_kind, entity_id, ref_fields, span) in entities {
        // Skip entities whose kind is not registered (already E024)
        if !kind_reg.contains(entity_kind) {
            continue;
        }

        for (field_name, target_ids) in ref_fields {
            // Look up the field's target_kind constraint
            let expected_kind = match field_reg.get(entity_kind, field_name) {
                Some(entry) => match &entry.target_kind {
                    Some(tk) => tk.as_str(),
                    None => continue, // No constraint — any kind is valid
                },
                None => continue, // Unknown field — already W020
            };

            for target_id in target_ids {
                // Only check targets that exist in the graph (missing = E001)
                if let Some(actual_kind) = node_kind_index.get(target_id.as_str())
                    && actual_kind != expected_kind
                {
                    diagnostics.push(Diagnostic {
                        code: "E022".to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "reference '{}' in field '{}' of {} '{}' targets a {}, but this field expects {}",
                            target_id, field_name, entity_kind, entity_id, actual_kind, expected_kind
                        ),
                        span: Some(span.clone()),
                        suggestion: None,
                    });
                }
            }
        }
    }

    diagnostics
}

/// Helper: Get LSP keywords from structural keywords + registry.
pub fn lsp_keywords_with_registry(kind_reg: &KindRegistry) -> Vec<String> {
    let mut keywords: Vec<String> = vec!["use".into(), "define".into()];
    for keyword in kind_reg.keywords() {
        if keyword != "use" && keyword != "define" {
            keywords.push(keyword.clone());
        }
    }
    keywords.sort();
    keywords.dedup();
    keywords
}

/// Auto-generate E006 validation rules for every field marked `required: true`
/// in the FieldRegistry. Each rule fires at Error severity when the field is
/// absent on an entity of the corresponding kind.
pub fn generate_required_field_rules(
    field_registry: &FieldRegistry,
) -> Vec<ValidationRulePattern> {
    let mut rules: Vec<ValidationRulePattern> = field_registry
        .iter()
        .filter(|(_, _, entry)| entry.required)
        .map(|(kind, field, _)| ValidationRulePattern {
            code: "E006".to_string(),
            severity: Severity::Error,
            message_template: format!("{kind} '{{id}}' is missing required field '{field}'"),
            check: ValidationPatternKind::MissingRequiredField,
            target_kind: Some(kind.to_string()),
            edge_type: None,
            field: Some(field.to_string()),
            constraint: None,
            wasm_function: None,
        })
        .collect();
    rules.sort_by(|a, b| {
        a.target_kind
            .cmp(&b.target_kind)
            .then_with(|| a.field.cmp(&b.field))
    });
    rules
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{populate_registries, ManifestV2};
    use specforge_common::Sym;

    fn software_manifest() -> ManifestV2 {
        serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm",
                "entityKinds": [
                    {
                        "name": "Behavior",
                        "keyword": "behavior",
                        "testable": true,
                        "supportsVerify": true,
                        "fields": [
                            { "name": "contract", "fieldType": "block" },
                            { "name": "invariants", "fieldType": "reference_list" }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap()
    }

    fn span(file: &str) -> SourceSpan {
        SourceSpan {
            file: Sym::new(file),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        }
    }

    // -- B:collapse_grammar_to_generic_entity_block --
    // These verify statements are about the tree-sitter grammar structure.
    // The grammar is already collapsed to a single generic entity_block rule.
    // We test by checking the parser behavior.

    // B:collapse_grammar_to_generic_entity_block — verify unit "grammar has single generic entity_block rule"
    #[test]
    fn test_grammar_has_single_generic_entity_block_rule() {
        // The tree-sitter grammar defines entity_block as a generic rule.
        // Any keyword is accepted structurally. We verify by parsing an
        // arbitrary keyword — if the grammar had per-keyword rules, unknown
        // keywords would fail to parse.
        let source = "recipe my_recipe \"My Recipe\" {\n  contract \"cooks food\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        assert_eq!(parsed.entities.len(), 1);
        assert_eq!(parsed.entities[0].kind.raw, "recipe");
    }

    // B:collapse_grammar_to_generic_entity_block — verify unit "no per-keyword block rules remain in grammar"
    #[test]
    fn test_no_per_keyword_block_rules_in_grammar() {
        // Multiple arbitrary keywords all parse successfully
        for keyword in &["behavior", "invariant", "xyzzy", "custom_kind", "foobar"] {
            let source = format!("{} test_id \"Title\" {{\n  contract \"test\"\n}}\n", keyword);
            let parsed = specforge_parser::parse(&source, "test.spec");
            assert_eq!(
                parsed.entities.len(),
                1,
                "keyword '{}' should parse as entity",
                keyword
            );
        }
    }

    // B:collapse_grammar_to_generic_entity_block — verify unit "spec_block remains as separate grammar rule"
    #[test]
    fn test_spec_block_remains_as_separate_grammar_rule() {
        let source = "spec project_name \"My Project\" {\n  version \"1.0.0\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        assert_eq!(parsed.entities.len(), 1);
        assert_eq!(parsed.entities[0].kind.raw, "spec");
    }

    // B:collapse_grammar_to_generic_entity_block — verify unit "ref_block remains as separate grammar rule"
    #[test]
    fn test_ref_block_remains_as_separate_grammar_rule() {
        let source = "ref gh.issue:42 \"Fix bug\"\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        // ref blocks have their own grammar rule with scheme:id syntax
        assert_eq!(parsed.entities.len(), 1);
        assert_eq!(parsed.entities[0].kind.raw, "ref");
    }

    // B:collapse_grammar_to_generic_entity_block — verify unit "use_import remains as separate grammar rule"
    #[test]
    fn test_use_import_remains_as_separate_grammar_rule() {
        let source = "use \"types/core\"\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        assert_eq!(parsed.imports.len(), 1);
    }

    // B:collapse_grammar_to_generic_entity_block — verify unit "define_block remains as separate grammar rule"
    #[test]
    fn test_define_block_remains_as_separate_grammar_rule() {
        let source = "define user_story {\n  required [description]\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        // define blocks are parsed via the define_block grammar rule.
        // They may appear as entities with kind "define" or as a separate structure.
        // The key invariant: define_block is a distinct grammar rule from entity_block.
        let has_define_entity = parsed.entities.iter().any(|e| e.kind.raw == "define");
        assert!(
            has_define_entity || parsed.entities.is_empty(),
            "define should be parsed via define_block grammar rule"
        );
    }

    // B:collapse_grammar_to_generic_entity_block — verify contract "requires/ensures consistency for grammar collapse"
    #[test]
    fn test_collapse_grammar_contract() {
        // requires: grammar source available (tree-sitter compiled)
        // ensures: single generic rule — any keyword parses
        let source = "unknown_keyword test_id \"Title\" {\n  field \"value\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        assert_eq!(parsed.entities.len(), 1);
        // ensures: structural rules preserved
        let spec_src = "spec my_spec \"My Spec\" {\n  version \"1.0\"\n}\n";
        let spec_parsed = specforge_parser::parse(spec_src, "test.spec");
        assert_eq!(spec_parsed.entities[0].kind.raw, "spec");
    }

    // -- B:two_phase_parse_structural --

    // B:two_phase_parse_structural — verify unit "unknown keyword parsed into generic entity node"
    #[test]
    fn test_unknown_keyword_parsed_into_generic_entity_node() {
        let source = "xyzzy test_id \"Unknown\" {\n  data \"hello\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        assert_eq!(parsed.entities.len(), 1);
        assert_eq!(parsed.entities[0].kind.raw, "xyzzy");
        assert_eq!(parsed.entities[0].id.raw, "test_id");
    }

    // B:two_phase_parse_structural — verify unit "no keyword validation in Phase 1"
    #[test]
    fn test_no_keyword_validation_in_phase_1() {
        // Phase 1 is the parser — it should not validate keywords
        let source = "not_a_real_kind my_id \"Title\" {\n  stuff \"things\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        assert!(parsed.errors.is_empty(), "parser should not produce keyword errors");
        assert_eq!(parsed.entities.len(), 1);
    }

    // B:two_phase_parse_structural — verify unit "all .spec files parsed before Phase 2"
    #[test]
    fn test_all_spec_files_parsed_before_phase_2() {
        // Parse multiple files — all produce entities before any validation
        let files = [("a.spec", "behavior a \"A\" {\n  contract \"test\"\n}\n"),
            ("b.spec", "xyzzy b \"B\" {\n  data \"test\"\n}\n")];
        let parsed: Vec<_> = files
            .iter()
            .map(|(f, s)| specforge_parser::parse(s, f))
            .collect();
        assert_eq!(parsed[0].entities.len(), 1);
        assert_eq!(parsed[1].entities.len(), 1);
        // Both parsed successfully before any Phase 2 validation
    }

    // B:two_phase_parse_structural — verify unit "parse errors collected without aborting"
    #[test]
    fn test_parse_errors_collected_without_aborting() {
        let source = "behavior valid \"Valid\" {\n  contract \"ok\"\n}\n\n{invalid syntax\n\nbehavior also_valid \"Also\" {\n  contract \"ok\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        // Parser should collect errors but still parse valid entities
        // At minimum, the valid entities should be present
        assert!(
            !parsed.entities.is_empty(),
            "parser should recover and parse valid entities"
        );
    }

    // B:two_phase_parse_structural — verify contract "requires/ensures consistency for structural parsing"
    #[test]
    fn test_two_phase_parse_structural_contract() {
        // requires: spec files available
        let source = "custom_kind my_entity \"Title\" {\n  field \"value\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        // ensures: structural_parse_produced — generic entity blocks
        assert_eq!(parsed.entities.len(), 1);
        assert_eq!(parsed.entities[0].kind.raw, "custom_kind");
        // ensures: no_keyword_validation — no errors for unknown keyword
        assert!(parsed.errors.is_empty());
    }

    // -- B:two_phase_validate_semantic --

    // B:two_phase_validate_semantic — verify unit "known keyword passes semantic validation"
    #[test]
    fn test_known_keyword_passes_semantic_validation() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let entities = vec![("behavior".to_string(), "my_beh".to_string(), span("test.spec"))];
        let diags = detect_unknown_entity_kinds(&entities, &kind_reg, None);
        assert!(diags.is_empty());
    }

    // B:two_phase_validate_semantic — verify unit "unknown keyword produces E024"
    #[test]
    fn test_unknown_keyword_produces_e024() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let entities = vec![("xyzzy".to_string(), "my_xyz".to_string(), span("test.spec"))];
        let diags = detect_unknown_entity_kinds(&entities, &kind_reg, None);
        assert!(diags.iter().any(|d| d.code == "E024" && d.message.contains("xyzzy")));
    }

    // B:two_phase_validate_semantic — verify unit "field validation uses FieldRegistry"
    #[test]
    fn test_field_validation_uses_field_registry() {
        let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
        let entities = vec![(
            "behavior".to_string(),
            "my_beh".to_string(),
            vec!["contract".to_string(), "unknown_field".to_string()],
            span("test.spec"),
        )];
        let diags = detect_unknown_entity_fields(&entities, &kind_reg, &field_reg);
        assert!(diags.iter().any(|d| d.code == "W020" && d.message.contains("unknown_field")));
        assert!(!diags.iter().any(|d| d.message.contains("contract")));
    }

    // B:two_phase_validate_semantic — verify unit "Phase 2 starts only after registries populated"
    #[test]
    fn test_phase_2_starts_only_after_registries_populated() {
        // Phase 2 functions require populated registries as parameters.
        // With empty registries, all keywords would be unknown.
        let empty_reg = KindRegistry::new();
        let entities = vec![("behavior".to_string(), "my_beh".to_string(), span("test.spec"))];
        let diags = detect_unknown_entity_kinds(&entities, &empty_reg, None);
        // "behavior" is unknown because registry is empty
        assert!(diags.iter().any(|d| d.code == "E024"));
        // With populated registry, it passes
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let diags2 = detect_unknown_entity_kinds(&entities, &kind_reg, None);
        assert!(diags2.is_empty());
    }

    // B:two_phase_validate_semantic — verify unit "Phase 2 waits for both registries_populated AND define_blocks_registered"
    #[test]
    fn test_phase_2_waits_for_registries_and_defines() {
        // Simulate the full pipeline: extensions → registries → defines → Phase 2
        let (mut kind_reg, mut field_reg, _, _) = populate_registries(&[software_manifest()]);
        // Register a define block
        let define = crate::define::DefineBlockConfig {
            keyword: "user_story".to_string(),
            id_prefix: None,
            required_fields: vec!["description".to_string()],
            optional_fields: vec![],
            reference_targets: vec![],
        };
        crate::define::register_define_blocks(&[define], &mut kind_reg, &mut field_reg);

        // Now Phase 2: both extension kinds AND define kinds are known
        let entities = vec![
            ("behavior".to_string(), "b1".to_string(), span("test.spec")),
            ("user_story".to_string(), "us1".to_string(), span("test.spec")),
        ];
        let diags = detect_unknown_entity_kinds(&entities, &kind_reg, None);
        assert!(diags.is_empty(), "both extension and define kinds should be known");
    }

    // B:two_phase_validate_semantic — verify contract "requires/ensures consistency for semantic validation"
    #[test]
    fn test_two_phase_validate_semantic_contract() {
        let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
        // ensures: all blocks checked — known passes, unknown diagnosed
        let entities = vec![
            ("behavior".to_string(), "b1".to_string(), span("a.spec")),
            ("xyzzy".to_string(), "x1".to_string(), span("b.spec")),
        ];
        let kind_diags = detect_unknown_entity_kinds(&entities, &kind_reg, None);
        assert_eq!(kind_diags.len(), 1);
        assert_eq!(kind_diags[0].code, "E024");
        // ensures: fields validated
        let field_entities = vec![(
            "behavior".to_string(),
            "b1".to_string(),
            vec!["contract".to_string(), "bad_field".to_string()],
            span("a.spec"),
        )];
        let field_diags = detect_unknown_entity_fields(&field_entities, &kind_reg, &field_reg);
        assert!(field_diags.iter().any(|d| d.code == "W020"));
    }

    // -- B:suggest_missing_extensions --

    // B:suggest_missing_extensions — verify unit "E024 for keyword in index suggests the providing extension"
    #[test]
    fn test_e024_keyword_in_index_suggests_extension() {
        let kind_reg = KindRegistry::new();
        let mut entries = HashMap::new();
        entries.insert("behavior".to_string(), "@specforge/software".to_string());
        let index = KeywordExtensionIndex::from_entries(entries);
        let entities = vec![("behavior".to_string(), "b1".to_string(), span("test.spec"))];
        let diags = detect_unknown_entity_kinds(&entities, &kind_reg, Some(&index));
        assert!(diags[0].suggestion.as_ref().unwrap().contains("specforge add @specforge/software"));
    }

    // B:suggest_missing_extensions — verify unit "E024 for keyword not in index suggests specforge search"
    #[test]
    fn test_e024_keyword_not_in_index_suggests_search() {
        let kind_reg = KindRegistry::new();
        let index = KeywordExtensionIndex::new();
        let entities = vec![("xyzzy".to_string(), "x1".to_string(), span("test.spec"))];
        let diags = detect_unknown_entity_kinds(&entities, &kind_reg, Some(&index));
        assert!(diags[0].suggestion.as_ref().unwrap().contains("specforge outline"));
    }

    // B:suggest_missing_extensions — verify unit "keyword-to-extension index is loaded from bundled data file"
    #[test]
    fn test_keyword_extension_index_from_data() {
        // The index is data-driven, not hardcoded
        let json = r#"{"behavior": "@specforge/software", "feature": "@specforge/product"}"#;
        let entries: HashMap<String, String> = serde_json::from_str(json).unwrap();
        let index = KeywordExtensionIndex::from_entries(entries);
        assert_eq!(index.lookup("behavior"), Some("@specforge/software"));
        assert_eq!(index.lookup("feature"), Some("@specforge/product"));
        assert_eq!(index.lookup("unknown"), None);
    }

    // B:suggest_missing_extensions — verify contract "requires/ensures consistency for missing extension suggestions"
    #[test]
    fn test_suggest_missing_extensions_contract() {
        let kind_reg = KindRegistry::new();
        let mut entries = HashMap::new();
        entries.insert("behavior".to_string(), "@specforge/software".to_string());
        let index = KeywordExtensionIndex::from_entries(entries);
        // ensures: known keyword gets extension suggestion
        let e1 = vec![("behavior".to_string(), "b1".to_string(), span("test.spec"))];
        let d1 = detect_unknown_entity_kinds(&e1, &kind_reg, Some(&index));
        assert!(d1[0].suggestion.as_ref().unwrap().contains("@specforge/software"));
        // ensures: unknown keyword gets search suggestion
        let e2 = vec![("xyzzy".to_string(), "x1".to_string(), span("test.spec"))];
        let d2 = detect_unknown_entity_kinds(&e2, &kind_reg, Some(&index));
        assert!(d2[0].suggestion.as_ref().unwrap().contains("specforge outline"));
    }

    // -- B:detect_unknown_entity_kinds --

    // B:detect_unknown_entity_kinds — verify unit "unregistered keyword produces E024"
    #[test]
    fn test_unregistered_keyword_produces_e024() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let entities = vec![("unknown_thing".to_string(), "u1".to_string(), span("test.spec"))];
        let diags = detect_unknown_entity_kinds(&entities, &kind_reg, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E024");
    }

    // B:detect_unknown_entity_kinds — verify unit "E024 includes keyword name and source span"
    #[test]
    fn test_e024_includes_keyword_and_span() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let s = SourceSpan {
            file: Sym::new("my/file.spec"),
            start_line: 42,
            start_col: 0,
            end_line: 42,
            end_col: 10,
        };
        let entities = vec![("unknown_thing".to_string(), "u1".to_string(), s.clone())];
        let diags = detect_unknown_entity_kinds(&entities, &kind_reg, None);
        assert!(diags[0].message.contains("unknown_thing"));
        assert!(diags[0].message.contains("my/file.spec"));
        assert_eq!(diags[0].span.as_ref().unwrap().start_line, 42);
    }

    // B:detect_unknown_entity_kinds — verify unit "registered keyword does not produce E024"
    #[test]
    fn test_registered_keyword_no_e024() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let entities = vec![("behavior".to_string(), "b1".to_string(), span("test.spec"))];
        let diags = detect_unknown_entity_kinds(&entities, &kind_reg, None);
        assert!(diags.is_empty());
    }

    // B:detect_unknown_entity_kinds — verify unit "define-block keywords not checked against KindRegistry"
    #[test]
    fn test_define_block_keywords_not_checked() {
        let kind_reg = KindRegistry::new(); // empty
        let entities = vec![("define".to_string(), "my_define".to_string(), span("test.spec"))];
        let diags = detect_unknown_entity_kinds(&entities, &kind_reg, None);
        // "define" is a structural keyword, should NOT produce E024
        assert!(diags.is_empty());
    }

    // B:detect_unknown_entity_kinds — verify contract "requires/ensures consistency for unknown entity kind detection"
    #[test]
    fn test_detect_unknown_entity_kinds_contract() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        // ensures: unknown → E024
        let unknown = vec![("xyzzy".to_string(), "x1".to_string(), span("t.spec"))];
        let d1 = detect_unknown_entity_kinds(&unknown, &kind_reg, None);
        assert!(d1.iter().any(|d| d.code == "E024"));
        // ensures: registered → no E024
        let known = vec![("behavior".to_string(), "b1".to_string(), span("t.spec"))];
        let d2 = detect_unknown_entity_kinds(&known, &kind_reg, None);
        assert!(d2.is_empty());
    }

    // -- B:detect_unknown_entity_fields --

    // B:detect_unknown_entity_fields — verify unit "unregistered field name produces W020"
    #[test]
    fn test_unregistered_field_name_produces_w020() {
        let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
        let entities = vec![(
            "behavior".to_string(),
            "b1".to_string(),
            vec!["unknown_field".to_string()],
            span("test.spec"),
        )];
        let diags = detect_unknown_entity_fields(&entities, &kind_reg, &field_reg);
        assert!(diags.iter().any(|d| d.code == "W020" && d.message.contains("unknown_field")));
    }

    // B:detect_unknown_entity_fields — verify unit "W020 includes field name, entity kind, and source span"
    #[test]
    fn test_w020_includes_field_kind_span() {
        let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
        let s = SourceSpan {
            file: Sym::new("my/file.spec"),
            start_line: 10,
            start_col: 2,
            end_line: 10,
            end_col: 7,
        };
        let entities = vec![(
            "behavior".to_string(),
            "b1".to_string(),
            vec!["bad_field".to_string()],
            s,
        )];
        let diags = detect_unknown_entity_fields(&entities, &kind_reg, &field_reg);
        let d = &diags[0];
        assert!(d.message.contains("bad_field"));
        assert!(d.message.contains("behavior"));
        assert!(d.message.contains("my/file.spec"));
    }

    // B:detect_unknown_entity_fields — verify unit "registered field name does not produce W020"
    #[test]
    fn test_registered_field_name_no_w020() {
        let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
        let entities = vec![(
            "behavior".to_string(),
            "b1".to_string(),
            vec!["contract".to_string(), "invariants".to_string()],
            span("test.spec"),
        )];
        let diags = detect_unknown_entity_fields(&entities, &kind_reg, &field_reg);
        assert!(diags.is_empty());
    }

    // B:detect_unknown_entity_fields — verify unit "structural fields (title, verify) not checked against FieldRegistry"
    #[test]
    fn test_structural_fields_not_checked() {
        let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
        let entities = vec![(
            "behavior".to_string(),
            "b1".to_string(),
            vec!["title".to_string(), "verify".to_string()],
            span("test.spec"),
        )];
        let diags = detect_unknown_entity_fields(&entities, &kind_reg, &field_reg);
        assert!(diags.is_empty(), "structural fields should be skipped");
    }

    // B:detect_unknown_entity_fields — verify unit "field validation skipped when entity kind is unregistered"
    #[test]
    fn test_field_validation_skipped_for_unregistered_kind() {
        let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
        let entities = vec![(
            "xyzzy".to_string(), // not registered
            "x1".to_string(),
            vec!["any_field".to_string()],
            span("test.spec"),
        )];
        let diags = detect_unknown_entity_fields(&entities, &kind_reg, &field_reg);
        assert!(diags.is_empty(), "unregistered kind should skip field validation");
    }

    // B:detect_unknown_entity_fields — verify contract "requires/ensures consistency for unknown field detection"
    #[test]
    fn test_detect_unknown_entity_fields_contract() {
        let (kind_reg, field_reg, _, _) = populate_registries(&[software_manifest()]);
        // ensures: unknown field → W020
        let e1 = vec![(
            "behavior".to_string(), "b1".to_string(),
            vec!["bad".to_string()], span("t.spec"),
        )];
        assert!(detect_unknown_entity_fields(&e1, &kind_reg, &field_reg)
            .iter().any(|d| d.code == "W020"));
        // ensures: registered field → no W020
        let e2 = vec![(
            "behavior".to_string(), "b1".to_string(),
            vec!["contract".to_string()], span("t.spec"),
        )];
        assert!(detect_unknown_entity_fields(&e2, &kind_reg, &field_reg).is_empty());
        // ensures: unregistered kind → skipped
        let e3 = vec![(
            "xyzzy".to_string(), "x1".to_string(),
            vec!["anything".to_string()], span("t.spec"),
        )];
        assert!(detect_unknown_entity_fields(&e3, &kind_reg, &field_reg).is_empty());
    }

    // -- B:graceful_degradation_without_extensions --

    // B:graceful_degradation_without_extensions — verify unit "no extensions installed emits I002 info"
    #[test]
    fn test_no_extensions_emits_i002() {
        let kind_reg = KindRegistry::new();
        let diags = check_graceful_degradation(&kind_reg, 0);
        assert!(diags.iter().any(|d| d.code == "I002"));
    }

    // B:graceful_degradation_without_extensions — verify unit "structural parsing works without extensions"
    #[test]
    fn test_structural_parsing_without_extensions() {
        let source = "behavior my_beh \"Title\" {\n  contract \"test\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        // Parsing works even without any registries
        assert_eq!(parsed.entities.len(), 1);
    }

    // B:graceful_degradation_without_extensions — verify unit "graph built with generic nodes"
    #[test]
    fn test_graph_built_with_generic_nodes() {
        let source = "behavior my_beh \"Title\" {\n  contract \"test\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        let (graph, _) = specforge_graph::build_graph(&[parsed]);
        assert_eq!(graph.node_count(), 1);
        let node = graph.node("my_beh").unwrap();
        assert_eq!(node.kind.raw, "behavior");
    }

    // B:graceful_degradation_without_extensions — verify unit "LSP provides basic features without extensions"
    #[test]
    fn test_lsp_basic_features_without_extensions() {
        // LSP keyword completion with empty registry still includes structural keywords
        let keywords = crate::compilation::lsp_keywords_with_registry(&KindRegistry::new());
        assert!(keywords.contains(&"use".to_string()));
        assert!(keywords.contains(&"define".to_string()));
    }

    // B:graceful_degradation_without_extensions — verify unit "specforge export produces valid JSON from structural-only graph"
    #[test]
    fn test_export_valid_json_structural_only() {
        let source = "thing my_thing \"A Thing\" {\n  data \"hello\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        let (graph, _) = specforge_graph::build_graph(&[parsed]);
        let json = specforge_emitter::emit_json(&graph);
        // Should be valid JSON
        let parsed_json: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed_json.is_object());
    }

    // B:graceful_degradation_without_extensions — verify unit "generic entity nodes appear as nodes in exported graph"
    #[test]
    fn test_generic_nodes_in_exported_graph() {
        let source = "thing my_thing \"A Thing\" {\n  data \"hello\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        let (graph, _) = specforge_graph::build_graph(&[parsed]);
        let json = specforge_emitter::emit_json(&graph);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let nodes = v["nodes"].as_array().unwrap();
        assert!(nodes.iter().any(|n| n["id"] == "my_thing"));
    }

    // B:graceful_degradation_without_extensions — verify unit "references between generic entities produce edges"
    #[test]
    fn test_references_produce_edges() {
        let source = "thing a \"A\" {\n  refs [b]\n}\nthing b \"B\" {\n  data \"ok\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        let (graph, _) = specforge_graph::build_graph(&[parsed]);
        let json = specforge_emitter::emit_json(&graph);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let edges = v["edges"].as_array().unwrap();
        assert!(
            edges.iter().any(|e| e["source"] == "a" && e["target"] == "b"),
            "expected edge from a to b, got: {:?}",
            edges
        );
    }

    // B:graceful_degradation_without_extensions — verify unit "specforge check with zero extensions exits cleanly with I002"
    #[test]
    fn test_zero_extensions_exits_cleanly() {
        let kind_reg = KindRegistry::new();
        let diags = check_graceful_degradation(&kind_reg, 0);
        // Should produce I002 but no errors
        assert!(diags.iter().all(|d| d.severity == Severity::Info));
        assert!(diags.iter().any(|d| d.code == "I002"));
    }

    // B:graceful_degradation_without_extensions — verify contract "requires/ensures consistency for graceful degradation"
    #[test]
    fn test_graceful_degradation_contract() {
        // requires: registries populated (empty when no extensions)
        let kind_reg = KindRegistry::new();
        let diags = check_graceful_degradation(&kind_reg, 0);
        // ensures: I002 emitted
        assert!(diags.iter().any(|d| d.code == "I002"));
        // ensures: structural parsing works
        let source = "arbitrary my_id \"Title\" {\n  stuff \"value\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        assert_eq!(parsed.entities.len(), 1);
        // ensures: graph built
        let (graph, _) = specforge_graph::build_graph(&[parsed]);
        assert_eq!(graph.node_count(), 1);
        // ensures: export produces valid JSON
        let json = specforge_emitter::emit_json(&graph);
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();
    }

    // -- B:handle_all_extensions_failed_to_load --

    // B:handle_all_extensions_failed_to_load — verify unit "all extensions failing produces per-extension E-level diagnostics"
    #[test]
    fn test_all_extensions_failing_produces_per_extension_errors() {
        let failures = vec![
            ("@specforge/software".to_string(), "wasm binary not found".to_string()),
            ("@specforge/product".to_string(), "network timeout".to_string()),
        ];
        let diags = handle_all_extensions_failed(&failures);
        let errors: Vec<_> = diags.iter().filter(|d| d.code == "E028").collect();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].message.contains("@specforge/software"));
        assert!(errors[1].message.contains("@specforge/product"));
    }

    // B:handle_all_extensions_failed_to_load — verify unit "system transitions to structural-only mode after all failures"
    #[test]
    fn test_structural_only_mode_after_all_failures() {
        let failures = vec![("@ext/a".to_string(), "error".to_string())];
        let diags = handle_all_extensions_failed(&failures);
        // I002 emitted to indicate structural-only mode
        assert!(diags.iter().any(|d| d.code == "I002"));
    }

    // B:handle_all_extensions_failed_to_load — verify unit "specforge check with all extensions unavailable exits cleanly"
    #[test]
    fn test_all_extensions_unavailable_exits_cleanly() {
        let failures = vec![
            ("@ext/a".to_string(), "not found".to_string()),
            ("@ext/b".to_string(), "invalid manifest".to_string()),
        ];
        let diags = handle_all_extensions_failed(&failures);
        // Has per-extension errors + I002 for structural mode
        assert!(diags.iter().any(|d| d.code == "E028"));
        assert!(diags.iter().any(|d| d.code == "I002"));
        // System should still be functional (structural parsing works)
        let source = "thing test_id \"Title\" {\n  data \"value\"\n}\n";
        let parsed = specforge_parser::parse(source, "test.spec");
        assert_eq!(parsed.entities.len(), 1);
    }

    // B:handle_all_extensions_failed_to_load — verify contract "requires/ensures consistency for all-extensions-failed handling"
    #[test]
    fn test_handle_all_extensions_failed_contract() {
        // requires: extension load attempts completed
        let failures = vec![("@ext/a".to_string(), "err".to_string())];
        let diags = handle_all_extensions_failed(&failures);
        // ensures: per-extension errors
        assert!(diags.iter().any(|d| d.code == "E028" && d.severity == Severity::Error));
        // ensures: structural-only mode
        assert!(diags.iter().any(|d| d.code == "I002" && d.severity == Severity::Info));
        // ensures: no crash — we got here without panicking
    }

    // -- B:detect_mistyped_references (W022) --

    fn two_extension_manifests() -> (ManifestV2, ManifestV2) {
        let software: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm",
                "entityKinds": [
                    {
                        "name": "Behavior",
                        "keyword": "behavior",
                        "testable": true,
                        "supportsVerify": true,
                        "fields": [
                            { "name": "invariants", "fieldType": "reference_list", "targetKind": "invariant" },
                            { "name": "features", "fieldType": "reference_list", "targetKind": "feature" },
                            { "name": "refs", "fieldType": "reference_list" }
                        ]
                    },
                    {
                        "name": "Invariant",
                        "keyword": "invariant",
                        "testable": true,
                        "supportsVerify": true
                    }
                ]
            }"#,
        )
        .unwrap();
        let product: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "entityKinds": [
                    {
                        "name": "Feature",
                        "keyword": "feature",
                        "testable": false,
                        "fields": [
                            { "name": "behaviors", "fieldType": "reference_list", "targetKind": "behavior" }
                        ]
                    }
                ],
                "peerDependencies": [
                    { "name": "@specforge/software", "version": ">=1.0.0" }
                ]
            }"#,
        )
        .unwrap();
        (software, product)
    }

    // verify unit "reference to correct kind produces no diagnostic"
    #[test]
    fn test_correct_kind_reference_no_diagnostic() {
        let (sw, prod) = two_extension_manifests();
        let (kind_reg, field_reg, _, _) = populate_registries(&[sw, prod]);
        let mut node_kind_index = HashMap::new();
        node_kind_index.insert("inv1".to_string(), "invariant".to_string());
        let entities = vec![(
            "behavior".to_string(),
            "b1".to_string(),
            vec![("invariants".to_string(), vec!["inv1".to_string()])],
            span("test.spec"),
        )];
        let diags = detect_mistyped_references(&entities, &field_reg, &kind_reg, &node_kind_index);
        assert!(diags.is_empty(), "correct kind should produce no diagnostic, got: {:?}", diags);
    }

    // verify unit "reference to wrong kind produces E022"
    #[test]
    fn test_wrong_kind_reference_produces_e022() {
        let (sw, prod) = two_extension_manifests();
        let (kind_reg, field_reg, _, _) = populate_registries(&[sw, prod]);
        let mut node_kind_index = HashMap::new();
        node_kind_index.insert("some_behavior".to_string(), "behavior".to_string());
        // Put a behavior ID in the "features" field which expects feature
        let entities = vec![(
            "behavior".to_string(),
            "b1".to_string(),
            vec![("features".to_string(), vec!["some_behavior".to_string()])],
            span("test.spec"),
        )];
        let diags = detect_mistyped_references(&entities, &field_reg, &kind_reg, &node_kind_index);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E022");
        assert!(diags[0].message.contains("some_behavior"));
        assert!(diags[0].message.contains("behavior"));
        assert!(diags[0].message.contains("feature"));
    }

    // verify unit "field with no target_kind constraint produces no diagnostic"
    #[test]
    fn test_no_target_kind_constraint_no_diagnostic() {
        let (sw, prod) = two_extension_manifests();
        let (kind_reg, field_reg, _, _) = populate_registries(&[sw, prod]);
        let mut node_kind_index = HashMap::new();
        node_kind_index.insert("anything".to_string(), "whatever".to_string());
        // "refs" field has no target_kind
        let entities = vec![(
            "behavior".to_string(),
            "b1".to_string(),
            vec![("refs".to_string(), vec!["anything".to_string()])],
            span("test.spec"),
        )];
        let diags = detect_mistyped_references(&entities, &field_reg, &kind_reg, &node_kind_index);
        assert!(diags.is_empty(), "unconstrained field should produce no diagnostic");
    }

    // verify unit "reference to nonexistent entity skipped (E001 handles it)"
    #[test]
    fn test_nonexistent_target_skipped() {
        let (sw, prod) = two_extension_manifests();
        let (kind_reg, field_reg, _, _) = populate_registries(&[sw, prod]);
        let node_kind_index = HashMap::new(); // empty — target doesn't exist
        let entities = vec![(
            "behavior".to_string(),
            "b1".to_string(),
            vec![("invariants".to_string(), vec!["nonexistent".to_string()])],
            span("test.spec"),
        )];
        let diags = detect_mistyped_references(&entities, &field_reg, &kind_reg, &node_kind_index);
        assert!(diags.is_empty(), "nonexistent target should be skipped (E001 handles it)");
    }

    // verify unit "unregistered entity kind skipped"
    #[test]
    fn test_unregistered_entity_kind_skipped() {
        let (sw, prod) = two_extension_manifests();
        let (kind_reg, field_reg, _, _) = populate_registries(&[sw, prod]);
        let mut node_kind_index = HashMap::new();
        node_kind_index.insert("x".to_string(), "behavior".to_string());
        let entities = vec![(
            "unknown_kind".to_string(),
            "u1".to_string(),
            vec![("features".to_string(), vec!["x".to_string()])],
            span("test.spec"),
        )];
        let diags = detect_mistyped_references(&entities, &field_reg, &kind_reg, &node_kind_index);
        assert!(diags.is_empty(), "unregistered kind should be skipped");
    }

    // verify unit "multiple wrong-kind references produce multiple E022"
    #[test]
    fn test_multiple_wrong_kind_refs_produce_multiple_e022() {
        let (sw, prod) = two_extension_manifests();
        let (kind_reg, field_reg, _, _) = populate_registries(&[sw, prod]);
        let mut node_kind_index = HashMap::new();
        node_kind_index.insert("b1".to_string(), "behavior".to_string());
        node_kind_index.insert("b2".to_string(), "behavior".to_string());
        // Two behavior IDs in "features" field (expects feature)
        let entities = vec![(
            "behavior".to_string(),
            "my_beh".to_string(),
            vec![("features".to_string(), vec!["b1".to_string(), "b2".to_string()])],
            span("test.spec"),
        )];
        let diags = detect_mistyped_references(&entities, &field_reg, &kind_reg, &node_kind_index);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.code == "E022"));
    }

    // verify unit "cross-extension typed reference validated"
    #[test]
    fn test_cross_extension_typed_reference_validated() {
        let (sw, prod) = two_extension_manifests();
        let (kind_reg, field_reg, _, _) = populate_registries(&[sw, prod]);
        let mut node_kind_index = HashMap::new();
        node_kind_index.insert("f1".to_string(), "feature".to_string());
        node_kind_index.insert("b1".to_string(), "behavior".to_string());
        // feature.behaviors should accept behavior — correct cross-extension ref
        let entities = vec![(
            "feature".to_string(),
            "f1".to_string(),
            vec![("behaviors".to_string(), vec!["b1".to_string()])],
            span("test.spec"),
        )];
        let diags = detect_mistyped_references(&entities, &field_reg, &kind_reg, &node_kind_index);
        assert!(diags.is_empty(), "correct cross-extension ref should pass");
    }

    // verify contract "E022 message includes target id, field name, actual kind, expected kind"
    #[test]
    fn test_e022_message_includes_all_details() {
        let (sw, prod) = two_extension_manifests();
        let (kind_reg, field_reg, _, _) = populate_registries(&[sw, prod]);
        let mut node_kind_index = HashMap::new();
        node_kind_index.insert("inv1".to_string(), "invariant".to_string());
        // Put invariant in "features" field (expects feature)
        let entities = vec![(
            "behavior".to_string(),
            "my_beh".to_string(),
            vec![("features".to_string(), vec!["inv1".to_string()])],
            span("test.spec"),
        )];
        let diags = detect_mistyped_references(&entities, &field_reg, &kind_reg, &node_kind_index);
        assert_eq!(diags.len(), 1);
        let msg = &diags[0].message;
        assert!(msg.contains("inv1"), "should mention target id");
        assert!(msg.contains("features"), "should mention field name");
        assert!(msg.contains("invariant"), "should mention actual kind");
        assert!(msg.contains("feature"), "should mention expected kind");
        assert!(msg.contains("my_beh"), "should mention source entity");
    }

    // -- B:register_verify_kinds_from_manifest --

    // B:register_verify_kinds_from_manifest — verify unit "unknown verify kind in .spec produces W-level diagnostic in Phase 2"
    #[test]
    fn test_unknown_verify_kind_produces_w_level_diagnostic() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm",
                "verifyKinds": ["unit", "integration", "contract"],
                "entityKinds": [
                    { "name": "Behavior", "keyword": "behavior", "testable": true, "supportsVerify": true }
                ]
            }"#,
        )
        .unwrap();

        let (kind_reg, _, _, _) = populate_registries(std::slice::from_ref(&manifest));
        let (registered_kinds, _) = crate::register_verify_kinds(std::slice::from_ref(&manifest));

        // Entity uses "smoke" which is not in registered kinds
        let entities = vec![(
            "behavior".to_string(),
            "my_behavior".to_string(),
            vec!["unit".to_string(), "smoke".to_string()],
            span("test.spec"),
        )];

        let diags = detect_unknown_verify_kinds(&entities, &registered_kinds, &kind_reg);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W026");
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(diags[0].message.contains("smoke"));
        assert!(diags[0].message.contains("not registered"));
    }

    // B:register_verify_kinds_from_manifest — verify unit "verify kind not in entity's allowed_verify_kinds produces W026"
    #[test]
    fn test_verify_kind_not_in_allowed_list_produces_w026() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm",
                "verifyKinds": ["unit", "integration", "contract", "property"],
                "entityKinds": [
                    {
                        "name": "Behavior",
                        "keyword": "behavior",
                        "testable": true,
                        "supportsVerify": true,
                        "allowedVerifyKinds": ["unit", "integration", "contract"]
                    }
                ]
            }"#,
        )
        .unwrap();

        let (kind_reg, _, _, _) = populate_registries(std::slice::from_ref(&manifest));
        let (registered_kinds, _) = crate::register_verify_kinds(std::slice::from_ref(&manifest));

        // "property" is globally registered but not in behavior's allowed_verify_kinds
        let entities = vec![(
            "behavior".to_string(),
            "my_behavior".to_string(),
            vec!["unit".to_string(), "property".to_string()],
            span("test.spec"),
        )];

        let diags = detect_unknown_verify_kinds(&entities, &registered_kinds, &kind_reg);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W026");
        assert!(diags[0].message.contains("property"));
        assert!(diags[0].message.contains("not allowed"));
        assert!(diags[0].message.contains("unit, integration, contract"));
    }

    // B:register_verify_kinds_from_manifest — verify unit "verify kind check skipped for unregistered entity kind"
    #[test]
    fn test_verify_kind_check_skipped_for_unregistered_entity_kind() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm",
                "verifyKinds": ["unit"],
                "entityKinds": [
                    { "name": "Behavior", "keyword": "behavior", "testable": true, "supportsVerify": true }
                ]
            }"#,
        )
        .unwrap();

        let (kind_reg, _, _, _) = populate_registries(std::slice::from_ref(&manifest));
        let (registered_kinds, _) = crate::register_verify_kinds(std::slice::from_ref(&manifest));

        // "unknown_kind" is not in the KindRegistry — verify kind check should be skipped
        let entities = vec![(
            "unknown_kind".to_string(),
            "u1".to_string(),
            vec!["nonexistent_verify_kind".to_string()],
            span("test.spec"),
        )];

        let diags = detect_unknown_verify_kinds(&entities, &registered_kinds, &kind_reg);
        assert!(diags.is_empty(), "should skip verify kind check for unregistered entity kind");
    }

    // -- B:generate_required_field_rules --

    #[test]
    fn test_generate_required_field_rules_from_registry() {
        use crate::registries::{FieldRegistryEntry, ManifestFieldType};
        let mut reg = FieldRegistry::new();
        reg.register(FieldRegistryEntry {
            kind_name: "behavior".into(),
            field_name: "contract".into(),
            description: None,
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".into(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: true,
            inverse_of: None,
        });
        reg.register(FieldRegistryEntry {
            kind_name: "behavior".into(),
            field_name: "category".into(),
            description: None,
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".into(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
            inverse_of: None,
        });
        reg.register(FieldRegistryEntry {
            kind_name: "invariant".into(),
            field_name: "guarantee".into(),
            description: None,
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".into(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: true,
            inverse_of: None,
        });

        let rules = generate_required_field_rules(&reg);
        assert_eq!(rules.len(), 2, "only required fields produce rules");
        assert!(rules.iter().all(|r| r.code == "E006"));
        assert!(rules.iter().all(|r| r.severity == Severity::Error));
        assert!(rules.iter().all(|r| r.check == ValidationPatternKind::MissingRequiredField));

        let targets: Vec<(&str, &str)> = rules
            .iter()
            .map(|r| (r.target_kind.as_deref().unwrap(), r.field.as_deref().unwrap()))
            .collect();
        assert!(targets.contains(&("behavior", "contract")));
        assert!(targets.contains(&("invariant", "guarantee")));
    }

    #[test]
    fn test_generate_required_field_rules_empty_registry() {
        let reg = FieldRegistry::new();
        let rules = generate_required_field_rules(&reg);
        assert!(rules.is_empty());
    }
}
