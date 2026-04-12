use std::collections::HashMap;

use specforge_common::{Diagnostic, Severity};

use crate::{KindRegistry, ManifestV2};

#[derive(Debug, Clone)]
pub struct RegisteredGrammar {
    pub entity_kind: String,
    pub grammar_wasm_path: String,
    pub export_name: Option<String>,
    pub source_extension: String,
}

#[derive(Debug, Clone)]
pub struct RegisteredBodyParser {
    pub entity_kind: String,
    pub export_name: String,
    pub source_extension: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarConflictPolicy {
    FirstWins,
    LastWins,
    Error,
}

pub fn register_grammar_contributions(
    manifests: &[ManifestV2],
    kind_registry: &KindRegistry,
    policy: GrammarConflictPolicy,
    wasm_path_exists: &dyn Fn(&str) -> bool,
) -> (Vec<RegisteredGrammar>, Vec<Diagnostic>) {
    let mut registered: HashMap<String, RegisteredGrammar> = HashMap::new();
    let mut diagnostics = Vec::new();

    for manifest in manifests {
        for contrib in &manifest.grammar_contributions {
            if !kind_registry.contains(&contrib.entity_kind) {
                diagnostics.push(Diagnostic {
                    code: "W024".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Grammar contribution for unregistered kind '{}' in extension '{}'",
                        contrib.entity_kind, manifest.name
                    ),
                    span: None,
                    suggestion: Some(format!(
                        "Ensure kind '{}' is declared in entity_kinds before contributing grammar",
                        contrib.entity_kind
                    )),
                });
                continue;
            }

            if !wasm_path_exists(&contrib.grammar_wasm_path) {
                diagnostics.push(Diagnostic {
                    code: "W025".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Grammar .wasm path '{}' not accessible for kind '{}' in extension '{}'",
                        contrib.grammar_wasm_path, contrib.entity_kind, manifest.name
                    ),
                    span: None,
                    suggestion: None,
                });
                continue;
            }

            let new_grammar = RegisteredGrammar {
                entity_kind: contrib.entity_kind.clone(),
                grammar_wasm_path: contrib.grammar_wasm_path.clone(),
                export_name: contrib.export_name.clone(),
                source_extension: manifest.name.clone(),
            };

            if let Some(existing) = registered.get(&contrib.entity_kind) {
                match policy {
                    GrammarConflictPolicy::FirstWins => {
                        diagnostics.push(Diagnostic {
                            code: "W024".to_string(),
                            severity: Severity::Warning,
                            message: format!(
                                "Grammar conflict for kind '{}': '{}' already registered by '{}', ignoring '{}'",
                                contrib.entity_kind, existing.source_extension, existing.source_extension, manifest.name
                            ),
                            span: None,
                            suggestion: None,
                        });
                    }
                    GrammarConflictPolicy::LastWins => {
                        diagnostics.push(Diagnostic {
                            code: "W024".to_string(),
                            severity: Severity::Warning,
                            message: format!(
                                "Grammar conflict for kind '{}': replacing '{}' with '{}'",
                                contrib.entity_kind, existing.source_extension, manifest.name
                            ),
                            span: None,
                            suggestion: None,
                        });
                        registered.insert(contrib.entity_kind.clone(), new_grammar);
                    }
                    GrammarConflictPolicy::Error => {
                        diagnostics.push(Diagnostic {
                            code: "E029".to_string(),
                            severity: Severity::Error,
                            message: format!(
                                "Grammar conflict for kind '{}': already registered by '{}', also declared by '{}'",
                                contrib.entity_kind, existing.source_extension, manifest.name
                            ),
                            span: None,
                            suggestion: Some("Set grammar_policy to 'first_wins' or 'last_wins' in compiler config".to_string()),
                        });
                    }
                }
            } else {
                registered.insert(contrib.entity_kind.clone(), new_grammar);
            }
        }
    }

    let mut grammars: Vec<RegisteredGrammar> = registered.into_values().collect();
    grammars.sort_by(|a, b| a.entity_kind.cmp(&b.entity_kind));
    (grammars, diagnostics)
}

pub fn register_body_parser_contributions(
    manifests: &[ManifestV2],
    kind_registry: &KindRegistry,
    wasm_export_exists: &dyn Fn(&str, &str) -> bool,
) -> (Vec<RegisteredBodyParser>, Vec<Diagnostic>) {
    let mut registered: HashMap<String, RegisteredBodyParser> = HashMap::new();
    let mut diagnostics = Vec::new();

    for manifest in manifests {
        for contrib in &manifest.body_parser_contributions {
            if !kind_registry.contains(&contrib.entity_kind) {
                diagnostics.push(Diagnostic {
                    code: "W024".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Body parser contribution for unregistered kind '{}' in extension '{}'",
                        contrib.entity_kind, manifest.name
                    ),
                    span: None,
                    suggestion: Some(format!(
                        "Ensure kind '{}' is declared in entity_kinds before contributing body parser",
                        contrib.entity_kind
                    )),
                });
                continue;
            }

            if !wasm_export_exists(&manifest.wasm_path, &contrib.export_name) {
                diagnostics.push(Diagnostic {
                    code: "W025".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Body parser export '{}' not found in '{}' for kind '{}' in extension '{}'",
                        contrib.export_name, manifest.wasm_path, contrib.entity_kind, manifest.name
                    ),
                    span: None,
                    suggestion: None,
                });
                continue;
            }

            if let Some(existing) = registered.get(&contrib.entity_kind) {
                diagnostics.push(Diagnostic {
                    code: "E029".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Duplicate body parser for kind '{}': already registered by '{}', also declared by '{}'",
                        contrib.entity_kind, existing.source_extension, manifest.name
                    ),
                    span: None,
                    suggestion: Some("At most one body parser per entity kind is allowed".to_string()),
                });
                continue;
            }

            registered.insert(
                contrib.entity_kind.clone(),
                RegisteredBodyParser {
                    entity_kind: contrib.entity_kind.clone(),
                    export_name: contrib.export_name.clone(),
                    source_extension: manifest.name.clone(),
                },
            );
        }
    }

    let mut parsers: Vec<RegisteredBodyParser> = registered.into_values().collect();
    parsers.sort_by(|a, b| a.entity_kind.cmp(&b.entity_kind));
    (parsers, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BodyParserContribution, ExtensionContributions, GrammarContribution, KindRegistryEntry,
        ManifestV2,
    };

    fn make_kind_entry(name: &str, ext: &str) -> KindRegistryEntry {
        KindRegistryEntry {
            kind_name: name.to_string(),
            description: None,
            source_extension: ext.to_string(),
            testable: false,
            singleton: false,
            supports_verify: false,
            allowed_verify_kinds: vec![],
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            open_fields: false,
        }
    }

    fn make_manifest(name: &str, grammar: Vec<GrammarContribution>, body_parsers: Vec<BodyParserContribution>) -> ManifestV2 {
        ManifestV2 {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            manifest_version: 2,
            wasm_path: format!("{}.wasm", name),
            contributes: ExtensionContributions::default(),
            entity_kinds: vec![],
            edge_types: vec![],
            validation_rules: vec![],
            verify_kinds: vec![],
            fields: vec![],
            incremental: None,
            reserved_keywords: vec![],
            migration_hook: None,
            peer_dependencies: vec![],
            sandbox_policy: None,
            host_api_version: None,
            entity_enhancements: vec![],
            starter_template: None,
            grammar_contributions: grammar,
            body_parser_contributions: body_parsers,
            ext_short: None,
            query_scope: None,
            collector_contributions: vec![],
            surfaces: None,
        }
    }

    // -- register_grammar_contributions --

    // B:register_grammar_contributions — verify unit "grammar contribution registered for valid entity kind"
    #[test]
    fn test_grammar_contribution_registered_for_valid_entity_kind() {
        let mut kind_reg = KindRegistry::new();
        kind_reg.register(make_kind_entry("behavior", "@specforge/software"));

        let manifest = make_manifest(
            "@specforge/software",
            vec![GrammarContribution {
                entity_kind: "behavior".to_string(),
                grammar_wasm_path: "behavior-grammar.wasm".to_string(),
                export_name: None,
            }],
            vec![],
        );

        let (grammars, diags) = register_grammar_contributions(
            &[manifest],
            &kind_reg,
            GrammarConflictPolicy::FirstWins,
            &|_| true,
        );

        assert_eq!(grammars.len(), 1);
        assert_eq!(grammars[0].entity_kind, "behavior");
        assert_eq!(grammars[0].source_extension, "@specforge/software");
        assert!(diags.is_empty());
    }

    // B:register_grammar_contributions — verify unit "grammar contribution for unregistered kind produces warning"
    #[test]
    fn test_grammar_contribution_for_unregistered_kind_produces_warning() {
        let kind_reg = KindRegistry::new();

        let manifest = make_manifest(
            "@specforge/software",
            vec![GrammarContribution {
                entity_kind: "nonexistent".to_string(),
                grammar_wasm_path: "grammar.wasm".to_string(),
                export_name: None,
            }],
            vec![],
        );

        let (grammars, diags) = register_grammar_contributions(
            &[manifest],
            &kind_reg,
            GrammarConflictPolicy::FirstWins,
            &|_| true,
        );

        assert_eq!(grammars.len(), 0);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W024");
        assert!(diags[0].message.contains("unregistered kind"));
    }

    // B:register_grammar_contributions — verify unit "grammar conflict detected and policy applied"
    #[test]
    fn test_grammar_conflict_detected_and_policy_applied() {
        let mut kind_reg = KindRegistry::new();
        kind_reg.register(make_kind_entry("behavior", "@specforge/software"));

        let m1 = make_manifest(
            "@ext/a",
            vec![GrammarContribution {
                entity_kind: "behavior".to_string(),
                grammar_wasm_path: "a-grammar.wasm".to_string(),
                export_name: None,
            }],
            vec![],
        );
        let m2 = make_manifest(
            "@ext/b",
            vec![GrammarContribution {
                entity_kind: "behavior".to_string(),
                grammar_wasm_path: "b-grammar.wasm".to_string(),
                export_name: None,
            }],
            vec![],
        );

        // FirstWins policy
        let (grammars, diags) = register_grammar_contributions(
            &[m1.clone(), m2.clone()],
            &kind_reg,
            GrammarConflictPolicy::FirstWins,
            &|_| true,
        );
        assert_eq!(grammars.len(), 1);
        assert_eq!(grammars[0].source_extension, "@ext/a");
        assert_eq!(diags.len(), 1);

        // LastWins policy
        let (grammars_lw, diags_lw) = register_grammar_contributions(
            &[m1.clone(), m2.clone()],
            &kind_reg,
            GrammarConflictPolicy::LastWins,
            &|_| true,
        );
        assert_eq!(grammars_lw.len(), 1);
        assert_eq!(grammars_lw[0].source_extension, "@ext/b"); // last wins
        assert_eq!(diags_lw.len(), 1);

        // Error policy
        let (grammars_e, diags_e) = register_grammar_contributions(
            &[m1, m2],
            &kind_reg,
            GrammarConflictPolicy::Error,
            &|_| true,
        );
        assert_eq!(grammars_e.len(), 1); // first one registered, second triggers error
        let error_diags: Vec<_> = diags_e.iter().filter(|d| d.code == "E029").collect();
        assert_eq!(error_diags.len(), 1);
    }

    // B:register_grammar_contributions — verify unit "grammar .wasm path validated as accessible"
    #[test]
    fn test_grammar_wasm_path_validated_as_accessible() {
        let mut kind_reg = KindRegistry::new();
        kind_reg.register(make_kind_entry("behavior", "@specforge/software"));

        let manifest = make_manifest(
            "@specforge/software",
            vec![GrammarContribution {
                entity_kind: "behavior".to_string(),
                grammar_wasm_path: "missing.wasm".to_string(),
                export_name: None,
            }],
            vec![],
        );

        let (grammars, diags) = register_grammar_contributions(
            &[manifest],
            &kind_reg,
            GrammarConflictPolicy::FirstWins,
            &|path| path != "missing.wasm",
        );

        assert_eq!(grammars.len(), 0);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W025");
        assert!(diags[0].message.contains("not accessible"));
    }

    // B:register_grammar_contributions — verify contract "requires/ensures consistency for grammar contribution registration"
    #[test]
    fn test_grammar_contribution_registration_contract() {
        // requires: extension_manifests_loaded_fired + kind_registry_available
        let mut kind_reg = KindRegistry::new();
        kind_reg.register(make_kind_entry("behavior", "@specforge/software"));
        kind_reg.register(make_kind_entry("feature", "@specforge/software"));

        let manifest = make_manifest(
            "@specforge/software",
            vec![
                GrammarContribution {
                    entity_kind: "behavior".to_string(),
                    grammar_wasm_path: "behavior-grammar.wasm".to_string(),
                    export_name: Some("parse_behavior".to_string()),
                },
                GrammarContribution {
                    entity_kind: "nonexistent".to_string(),
                    grammar_wasm_path: "ne.wasm".to_string(),
                    export_name: None,
                },
            ],
            vec![],
        );

        let (grammars, diags) = register_grammar_contributions(
            &[manifest],
            &kind_reg,
            GrammarConflictPolicy::FirstWins,
            &|_| true,
        );

        // ensures: grammar_contributions_registered — valid ones stored
        assert_eq!(grammars.len(), 1);
        assert_eq!(grammars[0].entity_kind, "behavior");
        assert_eq!(grammars[0].export_name.as_deref(), Some("parse_behavior"));

        // ensures: conflicts_resolved — invalid one warned
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W024");
    }

    // -- register_body_parser_contributions --

    // B:register_body_parser_contributions — verify unit "body parser contribution registered for valid entity kind"
    #[test]
    fn test_body_parser_contribution_registered_for_valid_entity_kind() {
        let mut kind_reg = KindRegistry::new();
        kind_reg.register(make_kind_entry("behavior", "@specforge/software"));

        let manifest = make_manifest(
            "@specforge/software",
            vec![],
            vec![BodyParserContribution {
                entity_kind: "behavior".to_string(),
                export_name: "parse_behavior_body".to_string(),
            }],
        );

        let (parsers, diags) = register_body_parser_contributions(
            &[manifest],
            &kind_reg,
            &|_, _| true,
        );

        assert_eq!(parsers.len(), 1);
        assert_eq!(parsers[0].entity_kind, "behavior");
        assert_eq!(parsers[0].export_name, "parse_behavior_body");
        assert!(diags.is_empty());
    }

    // B:register_body_parser_contributions — verify unit "body parser for unregistered kind produces warning"
    #[test]
    fn test_body_parser_for_unregistered_kind_produces_warning() {
        let kind_reg = KindRegistry::new();

        let manifest = make_manifest(
            "@specforge/software",
            vec![],
            vec![BodyParserContribution {
                entity_kind: "nonexistent".to_string(),
                export_name: "parse_body".to_string(),
            }],
        );

        let (parsers, diags) = register_body_parser_contributions(
            &[manifest],
            &kind_reg,
            &|_, _| true,
        );

        assert_eq!(parsers.len(), 0);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W024");
    }

    // B:register_body_parser_contributions — verify unit "duplicate body parser for same kind produces error"
    #[test]
    fn test_duplicate_body_parser_for_same_kind_produces_error() {
        let mut kind_reg = KindRegistry::new();
        kind_reg.register(make_kind_entry("behavior", "@specforge/software"));

        let m1 = make_manifest(
            "@ext/a",
            vec![],
            vec![BodyParserContribution {
                entity_kind: "behavior".to_string(),
                export_name: "parse_a".to_string(),
            }],
        );
        let m2 = make_manifest(
            "@ext/b",
            vec![],
            vec![BodyParserContribution {
                entity_kind: "behavior".to_string(),
                export_name: "parse_b".to_string(),
            }],
        );

        let (parsers, diags) = register_body_parser_contributions(
            &[m1, m2],
            &kind_reg,
            &|_, _| true,
        );

        assert_eq!(parsers.len(), 1); // first wins
        assert_eq!(parsers[0].source_extension, "@ext/a");
        let errors: Vec<_> = diags.iter().filter(|d| d.code == "E029").collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Duplicate body parser"));
    }

    // B:register_body_parser_contributions — verify unit "export_name verified against Wasm binary"
    #[test]
    fn test_export_name_verified_against_wasm_binary() {
        let mut kind_reg = KindRegistry::new();
        kind_reg.register(make_kind_entry("behavior", "@specforge/software"));

        let manifest = make_manifest(
            "@specforge/software",
            vec![],
            vec![BodyParserContribution {
                entity_kind: "behavior".to_string(),
                export_name: "missing_export".to_string(),
            }],
        );

        let (parsers, diags) = register_body_parser_contributions(
            &[manifest],
            &kind_reg,
            &|_, export| export != "missing_export",
        );

        assert_eq!(parsers.len(), 0);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W025");
        assert!(diags[0].message.contains("not found"));
    }

    // B:register_body_parser_contributions — verify contract "requires/ensures consistency for body parser contribution registration"
    #[test]
    fn test_body_parser_contribution_registration_contract() {
        // requires: extension_manifests_loaded_fired + kind_registry_available
        let mut kind_reg = KindRegistry::new();
        kind_reg.register(make_kind_entry("behavior", "@specforge/software"));
        kind_reg.register(make_kind_entry("feature", "@specforge/software"));

        let manifest = make_manifest(
            "@specforge/software",
            vec![],
            vec![
                BodyParserContribution {
                    entity_kind: "behavior".to_string(),
                    export_name: "parse_behavior_body".to_string(),
                },
                BodyParserContribution {
                    entity_kind: "nonexistent".to_string(),
                    export_name: "parse_ne".to_string(),
                },
            ],
        );

        let (parsers, diags) = register_body_parser_contributions(
            &[manifest],
            &kind_reg,
            &|_, _| true,
        );

        // ensures: body_parsers_registered — valid ones stored
        assert_eq!(parsers.len(), 1);
        assert_eq!(parsers[0].entity_kind, "behavior");

        // ensures: one_parser_per_kind_enforced — invalid one warned
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W024");
    }
}
