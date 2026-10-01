use crate::host_functions::CallSite;
use crate::runtime::{WasmCallResult, WasmRuntime};
use specforge_common::{Diagnostic, Severity};
use specforge_registry::{FieldEnhancement, ManifestV2};
use std::collections::HashSet;

/// Dispatch contribution exports for an extension based on its manifest.
/// Routes to the correct namespaced Wasm export function.
pub fn dispatch_contribution_exports(
    extension_name: &str,
    call_site: CallSite,
    runtime: &dyn WasmRuntime,
    input: &[u8],
) -> Result<Vec<u8>, Diagnostic> {
    let export_name = match call_site {
        CallSite::Validator => format!("{}_validate", extension_name.replace('/', "__")),
        CallSite::Renderer => format!("{}_render", extension_name.replace('/', "__")),
        CallSite::Provider => format!("{}_provide", extension_name.replace('/', "__")),
        CallSite::Parser => format!("{}_parse", extension_name.replace('/', "__")),
        CallSite::Collector => format!("{}_collect", extension_name.replace('/', "__")),
        CallSite::Analyzer => format!("{}_analyze", extension_name.replace('/', "__")),
    };

    match runtime.call_export(extension_name, &export_name, input) {
        WasmCallResult::Ok(output) => Ok(output),
        WasmCallResult::Trap(trap) => Err(Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': {}() trapped: {} — {}",
                extension_name, export_name, trap.kind, trap.message
            ),
            span: None,
            suggestion: None,
        }),
    }
}

/// Register entity enhancements from an extension into a collected set.
/// Detects conflicts when two extensions enhance the same kind with the same field name.
pub fn register_entity_enhancements(
    manifest: &ManifestV2,
    existing: &mut Vec<(String, FieldEnhancement)>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for enhancement in &manifest.entity_enhancements {
        // Check for field name conflicts with existing enhancements on the same target kind
        for field in &enhancement.fields {
            let conflict = existing.iter().any(|(_, existing_enh)| {
                existing_enh.target_kind == enhancement.target_kind
                    && existing_enh.source_extension != manifest.name
                    && existing_enh.fields.iter().any(|ef| ef.name == field.name)
            });

            if conflict {
                diagnostics.push(Diagnostic {
                    code: "E017".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "extension '{}': entity enhancement conflict — field '{}' on kind '{}' already enhanced by another extension",
                        manifest.name, field.name, enhancement.target_kind
                    ),
                    span: None,
                    suggestion: Some("rename the field or coordinate with the conflicting extension".to_string()),
                });
            }
        }

        existing.push((manifest.name.clone(), enhancement.clone()));
    }

    diagnostics
}

/// Structural DSL keywords that can never be used as entity kind names.
const STRUCTURAL_RESERVED: &[&str] = &["spec", "ref", "use", "define", "verify", "true", "false"];

/// Regex-like validation for entity kind identifiers: `^[a-z][a-z0-9_]{1,59}$`
fn is_valid_entity_kind_identifier(name: &str) -> bool {
    if name.len() < 2 || name.len() > 60 {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Check if an entity kind name is reserved by any installed extension,
/// conflicts with structural DSL keywords, or has invalid identifier characters.
pub fn reject_reserved_entity_kind(
    kind_name: &str,
    manifests: &[ManifestV2],
) -> Option<Diagnostic> {
    // Check structural DSL keywords first
    if STRUCTURAL_RESERVED.contains(&kind_name) {
        return Some(Diagnostic {
            code: "E035".to_string(),
            severity: Severity::Error,
            message: format!(
                "entity kind '{}' is a reserved structural keyword",
                kind_name
            ),
            span: None,
            suggestion: Some("choose a different entity kind name".to_string()),
        });
    }

    // Check identifier validity
    if !is_valid_entity_kind_identifier(kind_name) {
        return Some(Diagnostic {
            code: "E035".to_string(),
            severity: Severity::Error,
            message: format!(
                "entity kind '{}' is not a valid identifier (must match [a-z][a-z0-9_]{{1,59}})",
                kind_name
            ),
            span: None,
            suggestion: Some("use lowercase letters, digits, and underscores only".to_string()),
        });
    }

    // Check extension-reserved keywords
    for manifest in manifests {
        if manifest.reserved_keywords.iter().any(|k| k == kind_name) {
            return Some(Diagnostic {
                code: "E035".to_string(),
                severity: Severity::Error,
                message: format!(
                    "entity kind '{}' is reserved by extension '{}'",
                    kind_name, manifest.name
                ),
                span: None,
                suggestion: Some("choose a different entity kind name".to_string()),
            });
        }
    }
    None
}

/// Compute the required Wasm export names based on manifest contribution flags.
pub fn required_contribution_exports(manifest: &ManifestV2) -> Vec<String> {
    let slug = manifest.name.replace('@', "").replace('/', "__");
    let mut exports = Vec::new();

    if manifest.contributes.validators {
        exports.push(format!("{}_validate", slug));
    }
    if manifest.contributes.renderers {
        exports.push(format!("{}_render", slug));
    }
    if manifest.contributes.providers {
        exports.push(format!("{}_provide", slug));
    }
    if manifest.contributes.parsers {
        exports.push(format!("{}_parse", slug));
    }
    if manifest.contributes.collectors {
        exports.push(format!("collect__{}", slug));
    }

    exports
}

/// Validate that all required contribution exports are present in the available exports.
pub fn validate_contribution_exports(
    manifest: &ManifestV2,
    available_exports: &[String],
) -> Vec<Diagnostic> {
    let required = required_contribution_exports(manifest);
    let available_set: HashSet<&str> = available_exports.iter().map(|s| s.as_str()).collect();
    let mut diagnostics = Vec::new();

    for export in &required {
        if !available_set.contains(export.as_str()) {
            diagnostics.push(Diagnostic {
                code: "E020".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': declared contribution export '{}' is missing from Wasm module",
                    manifest.name, export
                ),
                span: None,
                suggestion: Some(format!(
                    "add #[export_name = \"{}\"] to the Wasm module",
                    export
                )),
            });
        }
    }

    diagnostics
}

/// Toggle state for extension contributions.
#[derive(Debug, Clone)]
pub struct ContributionToggle {
    pub extension_name: String,
    pub disabled: HashSet<String>,
}

/// Check if a contribution is disabled by the toggle configuration.
pub fn is_contribution_disabled(
    toggles: &[ContributionToggle],
    extension_name: &str,
    contribution_type: &str,
) -> bool {
    toggles
        .iter()
        .any(|t| t.extension_name == extension_name && t.disabled.contains(contribution_type))
}

/// Policy for resolving enhancement conflicts between extensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnhancementPolicy {
    Error,
}

/// A detected conflict where two extensions enhance the same entity kind with the same field.
#[derive(Debug, Clone)]
pub struct EnhancementConflict {
    pub entity_kind: String,
    pub field_name: String,
    pub first_extension: String,
    pub second_extension: String,
    pub is_grammar_level: bool,
}

/// An explicit override that resolves a conflict by picking a winning extension.
#[derive(Debug, Clone)]
pub struct EnhancementOverride {
    pub entity_kind: String,
    pub field_name: String,
    pub winning_extension: String,
}

/// Resolve enhancement conflicts according to policy and explicit overrides.
/// Grammar-level conflicts (is_grammar_level=true) always produce E018 regardless of overrides.
/// Field-level conflicts produce E017 unless an explicit override exists.
pub fn resolve_enhancement_conflicts(
    conflicts: &[EnhancementConflict],
    _policy: EnhancementPolicy,
    overrides: &[EnhancementOverride],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for conflict in conflicts {
        if conflict.is_grammar_level {
            // Grammar-level conflicts always error, overrides don't apply
            diagnostics.push(Diagnostic {
                code: "E018".to_string(),
                severity: Severity::Error,
                message: format!(
                    "grammar conflict on kind '{}': extension '{}' and '{}' both contribute grammar — cannot be overridden",
                    conflict.entity_kind, conflict.first_extension, conflict.second_extension
                ),
                span: None,
                suggestion: Some("only one extension may provide grammar for a given entity kind".to_string()),
            });
            continue;
        }

        // Check if an explicit override resolves this conflict
        let has_override = overrides
            .iter()
            .any(|o| o.entity_kind == conflict.entity_kind && o.field_name == conflict.field_name);

        if !has_override {
            diagnostics.push(Diagnostic {
                code: "E017".to_string(),
                severity: Severity::Error,
                message: format!(
                    "enhancement conflict on kind '{}' field '{}': extension '{}' and '{}' both define this field",
                    conflict.entity_kind, conflict.field_name,
                    conflict.first_extension, conflict.second_extension
                ),
                span: None,
                suggestion: Some("add an explicit override in specforge.json to resolve".to_string()),
            });
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{MockRuntime, WasmTrapInfo};
    use crate::test_helpers::default_manifest;
    use specforge_registry::{ExtensionContributions, ManifestField};

    // -- dispatch_contribution_exports --

    // B:dispatch_contribution_exports — verify unit "routes to namespaced Wasm export"
    #[test]
    fn test_routes_to_namespaced_export() {
        let runtime =
            MockRuntime::new().with_call_ok("@specforge__software_validate", b"ok".to_vec());

        let result = dispatch_contribution_exports(
            "@specforge/software",
            CallSite::Validator,
            &runtime,
            &[],
        );
        assert!(result.is_ok());
    }

    // B:dispatch_contribution_exports — verify unit "returns error diagnostic on trap"
    #[test]
    fn test_dispatch_returns_error_on_trap() {
        let runtime = MockRuntime::new().with_call_trap(
            "@specforge__software_validate",
            WasmTrapInfo {
                kind: "unreachable".to_string(),
                message: "panic".to_string(),
                export_name: "@specforge__software_validate".to_string(),
            },
        );

        let result = dispatch_contribution_exports(
            "@specforge/software",
            CallSite::Validator,
            &runtime,
            &[],
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "E028");
    }

    // B:dispatch_contribution_exports — verify unit "different call sites route to different exports"
    #[test]
    fn test_different_call_sites_route_differently() {
        let runtime = MockRuntime::new()
            .with_call_ok("ext_render", b"html".to_vec())
            .with_call_ok("ext_collect", b"json".to_vec());

        let render = dispatch_contribution_exports("ext", CallSite::Renderer, &runtime, &[]);
        assert!(render.is_ok());

        let collect = dispatch_contribution_exports("ext", CallSite::Collector, &runtime, &[]);
        assert!(collect.is_ok());
    }

    // -- register_entity_enhancements --

    // B:register_entity_enhancements — verify unit "applies fields to other kinds"
    #[test]
    fn test_registers_enhancements() {
        let mut manifest = default_manifest();
        manifest.name = "@test/coverage".to_string();
        manifest.entity_enhancements = vec![FieldEnhancement {
            verify_kinds: None,
            target_kind: "behavior".to_string(),
            source_extension: "@test/coverage".to_string(),
            edge_types: vec![],
            fields: vec![ManifestField {
                name: "coverage_threshold".to_string(),
                field_type: "string".to_string(),
                description: None,
                edge: None,
                target_kind: None,
                file_reference: false,
                required: false,
                default_value: None,
                enum_values: vec![],
                inverse_of: None,
                normative: false,
                derived_from: None,
            }],
        }];

        let mut existing = Vec::new();
        let diags = register_entity_enhancements(&manifest, &mut existing);
        assert!(diags.is_empty());
        assert_eq!(existing.len(), 1);
    }

    // B:register_entity_enhancements — verify unit "detects field name conflict"
    #[test]
    fn test_detects_enhancement_conflict() {
        let mut existing = vec![(
            "@ext/a".to_string(),
            FieldEnhancement {
                verify_kinds: None,
                target_kind: "behavior".to_string(),
                source_extension: "@ext/a".to_string(),
                edge_types: vec![],
                fields: vec![ManifestField {
                    name: "priority".to_string(),
                    field_type: "string".to_string(),
                    description: None,
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    required: false,
                    default_value: None,
                    enum_values: vec![],
                    inverse_of: None,
                    normative: false,
                    derived_from: None,
                }],
            },
        )];

        let mut manifest = default_manifest();
        manifest.name = "@ext/b".to_string();
        manifest.entity_enhancements = vec![FieldEnhancement {
            verify_kinds: None,
            target_kind: "behavior".to_string(),
            source_extension: "@ext/b".to_string(),
            edge_types: vec![],
            fields: vec![ManifestField {
                name: "priority".to_string(), // Same field name!
                field_type: "string".to_string(),
                description: None,
                edge: None,
                target_kind: None,
                file_reference: false,
                required: false,
                default_value: None,
                enum_values: vec![],
                inverse_of: None,
                normative: false,
                derived_from: None,
            }],
        }];

        let diags = register_entity_enhancements(&manifest, &mut existing);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E017");
        assert!(diags[0].message.contains("priority"));
    }

    // B:register_entity_enhancements — verify unit "no conflict when same extension re-registers"
    #[test]
    fn test_no_conflict_same_extension() {
        let mut existing = vec![(
            "@ext/a".to_string(),
            FieldEnhancement {
                verify_kinds: None,
                target_kind: "behavior".to_string(),
                source_extension: "@ext/a".to_string(),
                edge_types: vec![],
                fields: vec![ManifestField {
                    name: "priority".to_string(),
                    field_type: "string".to_string(),
                    description: None,
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    required: false,
                    default_value: None,
                    enum_values: vec![],
                    inverse_of: None,
                    normative: false,
                    derived_from: None,
                }],
            },
        )];

        let mut manifest = default_manifest();
        manifest.name = "@ext/a".to_string(); // Same extension
        manifest.entity_enhancements = vec![FieldEnhancement {
            verify_kinds: None,
            target_kind: "behavior".to_string(),
            source_extension: "@ext/a".to_string(),
            edge_types: vec![],
            fields: vec![ManifestField {
                name: "priority".to_string(),
                field_type: "string".to_string(),
                description: None,
                edge: None,
                target_kind: None,
                file_reference: false,
                required: false,
                default_value: None,
                enum_values: vec![],
                inverse_of: None,
                normative: false,
                derived_from: None,
            }],
        }];

        let diags = register_entity_enhancements(&manifest, &mut existing);
        assert!(diags.is_empty());
    }

    // -- reject_reserved_entity_kind --

    // B:reject_reserved_entity_kind — verify unit "rejects reserved keyword"
    #[test]
    fn test_rejects_reserved_keyword() {
        let mut manifest = default_manifest();
        manifest.name = "@specforge/core".to_string();
        manifest.reserved_keywords = vec!["spec".to_string(), "ref".to_string()];

        let result = reject_reserved_entity_kind("spec", &[manifest]);
        assert!(result.is_some());
        let diag = result.unwrap();
        assert_eq!(diag.code, "E035");
        assert!(diag.message.contains("spec"));
    }

    // B:reject_reserved_entity_kind — verify unit "allows non-reserved keyword"
    #[test]
    fn test_allows_non_reserved_keyword() {
        let mut manifest = default_manifest();
        manifest.name = "@specforge/core".to_string();
        manifest.reserved_keywords = vec!["scenario".to_string()];

        let result = reject_reserved_entity_kind("behavior", &[manifest]);
        assert!(result.is_none());
    }

    // B:reject_reserved_entity_kind — verify unit "rejects structural keyword 'spec'"
    #[test]
    fn test_rejects_structural_keyword_spec() {
        let result = reject_reserved_entity_kind("spec", &[]);
        assert!(result.is_some());
        let diag = result.unwrap();
        assert_eq!(diag.code, "E035");
        assert!(diag.message.contains("reserved structural keyword"));
    }

    // B:reject_reserved_entity_kind — verify unit "rejects DSL syntax word 'define'"
    #[test]
    fn test_rejects_structural_keyword_define() {
        let result = reject_reserved_entity_kind("define", &[]);
        assert!(result.is_some());
        let diag = result.unwrap();
        assert_eq!(diag.code, "E035");
        assert!(diag.message.contains("reserved structural keyword"));
    }

    // B:reject_reserved_entity_kind — verify unit "rejects literal token 'true'"
    #[test]
    fn test_rejects_literal_token_true() {
        let result = reject_reserved_entity_kind("true", &[]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().code, "E035");

        let result_false = reject_reserved_entity_kind("false", &[]);
        assert!(result_false.is_some());
        assert_eq!(result_false.unwrap().code, "E035");
    }

    // B:reject_reserved_entity_kind — verify unit "rejects invalid identifier characters"
    #[test]
    fn test_rejects_invalid_identifier_characters() {
        // Uppercase
        let result = reject_reserved_entity_kind("Behavior", &[]);
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("not a valid identifier"));

        // Starts with digit
        let result = reject_reserved_entity_kind("1bad", &[]);
        assert!(result.is_some());

        // Contains special characters
        let result = reject_reserved_entity_kind("my-kind", &[]);
        assert!(result.is_some());

        // Too short (single char)
        let result = reject_reserved_entity_kind("a", &[]);
        assert!(result.is_some());
    }

    // B:reject_reserved_entity_kind — verify unit "extension reserving 'scenario' prevents other extensions from using it"
    #[test]
    fn test_cross_extension_reserved_keywords() {
        let mut m1 = default_manifest();
        m1.name = "@ext/testing".to_string();
        m1.reserved_keywords = vec!["scenario".to_string()];

        let mut m2 = default_manifest();
        m2.name = "@ext/other".to_string();

        // Another extension trying to use "scenario" should be rejected
        let result = reject_reserved_entity_kind("scenario", &[m1, m2]);
        assert!(result.is_some());
        let diag = result.unwrap();
        assert_eq!(diag.code, "E035");
        assert!(diag.message.contains("@ext/testing"));
    }

    // -- validate_contribution_exports --

    // B:validate_contribution_exports — verify unit "all declared exports present → pass"
    #[test]
    fn test_validate_exports_all_present_pass() {
        let mut manifest = default_manifest();
        manifest.name = "@specforge/software".to_string();
        manifest.contributes = ExtensionContributions {
            validators: true,
            renderers: true,
            ..Default::default()
        };

        let available = vec![
            "specforge__software_validate".to_string(),
            "specforge__software_render".to_string(),
        ];

        let diags = validate_contribution_exports(&manifest, &available);
        assert!(
            diags.is_empty(),
            "expected no diagnostics, got: {:?}",
            diags
        );
    }

    // B:validate_contribution_exports — verify unit "missing export → E020"
    #[test]
    fn test_validate_exports_missing_produces_e020() {
        let mut manifest = default_manifest();
        manifest.name = "@specforge/software".to_string();
        manifest.contributes = ExtensionContributions {
            validators: true,
            renderers: true,
            ..Default::default()
        };

        let available = vec![
            "specforge__software_validate".to_string(),
            // Missing: specforge__software_render
        ];

        let diags = validate_contribution_exports(&manifest, &available);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E020");
        assert!(diags[0].message.contains("render"));
    }

    // B:validate_contribution_exports — verify unit "extra exports ignored"
    #[test]
    fn test_validate_exports_extra_ignored() {
        let mut manifest = default_manifest();
        manifest.name = "@specforge/software".to_string();
        manifest.contributes = ExtensionContributions {
            validators: true,
            ..Default::default()
        };

        let available = vec![
            "specforge__software_validate".to_string(),
            "specforge__software_extra_function".to_string(),
            "some_other_export".to_string(),
        ];

        let diags = validate_contribution_exports(&manifest, &available);
        assert!(diags.is_empty());
    }

    // B:validate_contribution_exports — verify contract "requires/ensures consistency"
    #[test]
    fn test_validate_exports_contract() {
        let mut manifest = default_manifest();
        manifest.name = "@specforge/software".to_string();
        manifest.contributes = ExtensionContributions {
            validators: true,
            collectors: true,
            ..Default::default()
        };

        // ensures: all present → no diagnostics
        let full = vec![
            "specforge__software_validate".to_string(),
            "collect__specforge__software".to_string(),
        ];
        assert!(validate_contribution_exports(&manifest, &full).is_empty());

        // ensures: missing → E020 with export name
        let diags = validate_contribution_exports(&manifest, &[]);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.code == "E020"));
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    // -- toggle_extension_contributions --

    // B:toggle_extension_contributions — verify unit "disabled contribution skipped"
    #[test]
    fn test_toggle_disabled_contribution_skipped() {
        let toggles = vec![ContributionToggle {
            extension_name: "@specforge/software".to_string(),
            disabled: HashSet::from(["validators".to_string()]),
        }];

        assert!(is_contribution_disabled(
            &toggles,
            "@specforge/software",
            "validators"
        ));
        assert!(!is_contribution_disabled(
            &toggles,
            "@specforge/software",
            "renderers"
        ));
    }

    // B:toggle_extension_contributions — verify unit "still loaded when some disabled"
    #[test]
    fn test_toggle_still_loaded_when_some_disabled() {
        let toggles = vec![ContributionToggle {
            extension_name: "@specforge/software".to_string(),
            disabled: HashSet::from(["validators".to_string()]),
        }];

        // Other contribution types remain active
        assert!(!is_contribution_disabled(
            &toggles,
            "@specforge/software",
            "renderers"
        ));
        assert!(!is_contribution_disabled(
            &toggles,
            "@specforge/software",
            "collectors"
        ));
        // Different extension unaffected
        assert!(!is_contribution_disabled(
            &toggles,
            "@specforge/governance",
            "validators"
        ));
    }

    // B:toggle_extension_contributions — verify unit "re-enabled resumes"
    #[test]
    fn test_toggle_reenabled_resumes() {
        // Initially disabled
        let toggles = vec![ContributionToggle {
            extension_name: "@specforge/software".to_string(),
            disabled: HashSet::from(["validators".to_string()]),
        }];
        assert!(is_contribution_disabled(
            &toggles,
            "@specforge/software",
            "validators"
        ));

        // After re-enabling (empty disabled set)
        let updated_toggles = vec![ContributionToggle {
            extension_name: "@specforge/software".to_string(),
            disabled: HashSet::new(),
        }];
        assert!(!is_contribution_disabled(
            &updated_toggles,
            "@specforge/software",
            "validators"
        ));
    }

    // -- resolve_enhancement_conflicts --

    // B:resolve_enhancement_conflicts — verify unit "error policy produces E017 for unresolved conflicts"
    #[test]
    fn test_resolve_conflicts_error_policy_produces_e017() {
        let conflicts = vec![EnhancementConflict {
            entity_kind: "behavior".to_string(),
            field_name: "priority".to_string(),
            first_extension: "@ext/a".to_string(),
            second_extension: "@ext/b".to_string(),
            is_grammar_level: false,
        }];

        let diags = resolve_enhancement_conflicts(&conflicts, EnhancementPolicy::Error, &[]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E017");
        assert!(diags[0].message.contains("priority"));
    }

    // B:resolve_enhancement_conflicts — verify unit "explicit override takes precedence over policy"
    #[test]
    fn test_resolve_conflicts_override_takes_precedence() {
        let conflicts = vec![EnhancementConflict {
            entity_kind: "behavior".to_string(),
            field_name: "priority".to_string(),
            first_extension: "@ext/a".to_string(),
            second_extension: "@ext/b".to_string(),
            is_grammar_level: false,
        }];

        let overrides = vec![EnhancementOverride {
            entity_kind: "behavior".to_string(),
            field_name: "priority".to_string(),
            winning_extension: "@ext/a".to_string(),
        }];

        let diags = resolve_enhancement_conflicts(&conflicts, EnhancementPolicy::Error, &overrides);
        assert!(diags.is_empty(), "override should suppress E017");
    }

    // B:resolve_enhancement_conflicts — verify unit "conflict record includes both extension identities"
    #[test]
    fn test_resolve_conflicts_includes_both_extension_names() {
        let conflicts = vec![EnhancementConflict {
            entity_kind: "event".to_string(),
            field_name: "channel".to_string(),
            first_extension: "@ext/alpha".to_string(),
            second_extension: "@ext/beta".to_string(),
            is_grammar_level: false,
        }];

        let diags = resolve_enhancement_conflicts(&conflicts, EnhancementPolicy::Error, &[]);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("@ext/alpha"));
        assert!(diags[0].message.contains("@ext/beta"));
    }

    // B:resolve_enhancement_conflicts — verify unit "no E017 when no conflicts exist"
    #[test]
    fn test_resolve_conflicts_no_conflicts_no_diagnostics() {
        let diags = resolve_enhancement_conflicts(&[], EnhancementPolicy::Error, &[]);
        assert!(diags.is_empty());
    }

    // B:resolve_enhancement_conflicts — verify unit "override for non-existent conflict is ignored"
    #[test]
    fn test_resolve_conflicts_override_for_nonexistent_ignored() {
        let overrides = vec![EnhancementOverride {
            entity_kind: "behavior".to_string(),
            field_name: "nonexistent".to_string(),
            winning_extension: "@ext/a".to_string(),
        }];

        let diags = resolve_enhancement_conflicts(&[], EnhancementPolicy::Error, &overrides);
        assert!(diags.is_empty());
    }

    // B:resolve_enhancement_conflicts — verify contract "requires/ensures consistency"
    #[test]
    fn test_resolve_conflicts_contract() {
        // requires: conflicts detected, policy set
        // ensures: no conflicts → empty
        assert!(resolve_enhancement_conflicts(&[], EnhancementPolicy::Error, &[]).is_empty());

        // ensures: unresolved conflict → E017
        let conflict = EnhancementConflict {
            entity_kind: "type".to_string(),
            field_name: "format".to_string(),
            first_extension: "@ext/x".to_string(),
            second_extension: "@ext/y".to_string(),
            is_grammar_level: false,
        };
        let diags = resolve_enhancement_conflicts(
            std::slice::from_ref(&conflict),
            EnhancementPolicy::Error,
            &[],
        );
        assert!(diags.iter().all(|d| d.code == "E017"));
        assert!(diags.iter().all(|d| d.severity == Severity::Error));

        // ensures: override resolves → empty
        let over = EnhancementOverride {
            entity_kind: "type".to_string(),
            field_name: "format".to_string(),
            winning_extension: "@ext/x".to_string(),
        };
        assert!(
            resolve_enhancement_conflicts(&[conflict], EnhancementPolicy::Error, &[over])
                .is_empty()
        );
    }

    // B:resolve_enhancement_conflicts — verify unit "grammar-level conflict always errors regardless of policy"
    #[test]
    fn test_resolve_conflicts_grammar_level_always_errors() {
        let conflicts = vec![EnhancementConflict {
            entity_kind: "behavior".to_string(),
            field_name: "body".to_string(),
            first_extension: "@ext/a".to_string(),
            second_extension: "@ext/b".to_string(),
            is_grammar_level: true,
        }];

        // Even with an override, grammar-level conflicts produce E018
        let overrides = vec![EnhancementOverride {
            entity_kind: "behavior".to_string(),
            field_name: "body".to_string(),
            winning_extension: "@ext/a".to_string(),
        }];

        let diags = resolve_enhancement_conflicts(&conflicts, EnhancementPolicy::Error, &overrides);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E018");
        assert!(diags[0].message.contains("cannot be overridden"));
    }
}
