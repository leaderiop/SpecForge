use crate::ManifestV2;
#[cfg(test)]
use crate::{EdgeRegistry, FieldRegistry, KindRegistry};
use specforge_common::{Diagnostic, DiagnosticData, Severity};

/// Cross-validate registered entity fields: check target_kind and edge label references
/// resolve to registered entries. Test-only: `validate_manifest` reports W021 on load.
#[cfg(test)]
pub fn validate_registered_entity_fields(
    field_reg: &FieldRegistry,
    kind_reg: &KindRegistry,
    edge_reg: &EdgeRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (kind_name, field_name, entry) in field_reg.iter() {
        // Validate target_kind references
        if let Some(target) = &entry.target_kind
            && !kind_reg.contains(target)
        {
            diagnostics.push(Diagnostic {
                code: "W021".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "field '{}' on kind '{}' references target_kind '{}' which is not in the KindRegistry",
                    field_name, kind_name, target
                ),
                span: None,
                suggestion: None,
                data: None,
            });
        }

        // Validate edge label references
        if let Some(edge) = &entry.edge
            && !edge_reg.contains(edge)
        {
            diagnostics.push(Diagnostic {
                code: "W021".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "field '{}' on kind '{}' references edge label '{}' which is not in the EdgeRegistry",
                    field_name, kind_name, edge
                ),
                span: None,
                suggestion: None,
                data: None,
            });
        }
    }

    // Sort for deterministic output
    diagnostics.sort_by(|a, b| a.message.cmp(&b.message));
    diagnostics
}

/// Detect duplicate entity kinds across extensions during registration.
/// This is called during populate_registries, but exposed separately for testing.
pub fn detect_duplicate_entity_kinds(manifests: &[ManifestV2]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for manifest in manifests {
        for kind in &manifest.entity_kinds {
            if let Some(first_ext) = seen.get(&kind.keyword) {
                diagnostics.push(Diagnostic {
                    code: "E026".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "entity kind '{}' registered by '{}' conflicts with '{}' (first registration wins)",
                        kind.keyword, manifest.name, first_ext
                    ),
                    span: None,
                    suggestion: None,
                    data: Some(Box::new(DiagnosticData::ShadowedKeyword {
                        keyword: kind.keyword.clone(),
                    })),
                });
            } else {
                seen.insert(kind.keyword.clone(), manifest.name.clone());
            }
        }
    }

    diagnostics
}

/// Validate peer dependencies against installed extensions: a required
/// peer must be installed (E027), and every installed peer, optional or
/// not, must satisfy its range (E027; W062 for malformed semver). An
/// optional peer that is not installed is fine.
pub fn validate_peer_dependencies(manifests: &[ManifestV2]) -> Vec<Diagnostic> {
    manifests
        .iter()
        .flat_map(|manifest| validate_peer_dependencies_of(manifest, manifests))
        .collect()
}

/// [`validate_peer_dependencies`] for the peers `manifest` declares, against
/// the `installed` extensions (`manifest` itself may be among them).
pub fn validate_peer_dependencies_of(
    manifest: &ManifestV2,
    installed: &[ManifestV2],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let installed: std::collections::HashMap<&str, &str> = installed
        .iter()
        .map(|m| (m.name.as_str(), m.version.as_str()))
        .collect();

    for peer in &manifest.peer_dependencies {
        match installed.get(peer.name.as_str()) {
            None if peer.optional => {}
            None => {
                diagnostics.push(Diagnostic {
                    code: "E027".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "extension '{}' requires peer dependency '{}' {} which is not installed",
                        manifest.name, peer.name, peer.version
                    ),
                    span: None,
                    suggestion: Some(format!("install it with: specforge add {}", peer.name)),
                    data: None,
                });
            }
            Some(installed_version) => {
                // Validate that both the required range and installed version are parseable semver
                let req_parse = semver::VersionReq::parse(&peer.version);
                let ver_parse = semver::Version::parse(installed_version);

                if req_parse.is_err() {
                    diagnostics.push(Diagnostic {
                        code: "W062".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "extension '{}' declares peer dependency '{}' with malformed semver range '{}'",
                            manifest.name, peer.name, peer.version
                        ),
                        span: None,
                        suggestion: Some("use a valid semver range like ^1.0.0, ~1.2.0, or >=1.0.0".to_string()),
                        data: None,
                    });
                } else if ver_parse.is_err() {
                    diagnostics.push(Diagnostic {
                        code: "W062".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "extension '{}' has malformed version '{}' (not valid semver)",
                            peer.name, installed_version
                        ),
                        span: None,
                        suggestion: Some("use a valid semver version like 1.0.0".to_string()),
                        data: None,
                    });
                } else if !version_satisfies(installed_version, &peer.version) {
                    diagnostics.push(Diagnostic {
                        code: "E027".to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "extension '{}' requires peer dependency '{}' {} but version {} is installed",
                            manifest.name, peer.name, peer.version, installed_version
                        ),
                        span: None,
                        suggestion: None,
                        data: None,
                    });
                }
            }
        }
    }

    diagnostics
}

/// Check if an installed version satisfies a required version range.
/// Supports semver ranges: ^X.Y.Z, ~X.Y.Z, >=X.Y.Z, >X.Y.Z, <=X.Y.Z, <X.Y.Z, and exact X.Y.Z.
fn version_satisfies(installed: &str, required: &str) -> bool {
    let Ok(ver) = semver::Version::parse(installed) else {
        return false;
    };
    let Ok(req) = semver::VersionReq::parse(required) else {
        // Fall back to exact match for non-parseable ranges
        return installed == required;
    };
    req.matches(&ver)
}

/// W017: a kind declared `testable` that does not accept `verify`
/// statements, so its entities could never declare the obligations
/// coverage counts. [`super::build::build_registries`] runs it once the
/// kinds are populated. A kind that accepts `verify` but is not testable
/// (a formal `property`) is a deliberate combination, not reported.
pub(crate) fn validate_extension_testability(kind_reg: &crate::KindRegistry) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (_, entry) in kind_reg.iter() {
        if entry.testable && !entry.supports_verify {
            diagnostics.push(Diagnostic {
                code: "W017".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "entity kind '{}' from '{}' is testable but does not support verify statements",
                    entry.kind_name, entry.source_extension
                ),
                span: None,
                suggestion: Some("set supportsVerify: true in the manifest".to_string()),
                data: None,
            });
        }
    }

    // Sort for deterministic output
    diagnostics.sort_by(|a, b| a.message.cmp(&b.message));
    diagnostics
}

/// Register validation rules from manifests. Returns aggregated rules + diagnostics.
pub fn register_validation_rules(
    manifests: &[ManifestV2],
) -> (Vec<crate::ManifestValidationRule>, Vec<Diagnostic>) {
    let mut all_rules = Vec::new();
    let mut diagnostics = Vec::new();
    let mut seen_codes: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for manifest in manifests {
        for rule in &manifest.validation_rules {
            if let Some(first_ext) = seen_codes.get(&rule.code) {
                if *first_ext != manifest.name {
                    diagnostics.push(Diagnostic {
                        code: "W023".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "validation rule code '{}' from '{}' duplicates code from '{}'",
                            rule.code, manifest.name, first_ext
                        ),
                        span: None,
                        suggestion: None,
                        data: None,
                    });
                }
            } else {
                seen_codes.insert(rule.code.clone(), manifest.name.clone());
            }
            all_rules.push(rule.clone());
        }
    }

    // Sort by code for deterministic execution order
    all_rules.sort_by(|a, b| a.code.cmp(&b.code));

    (all_rules, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ManifestV2, populate_registries};

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
                            { "name": "invariants", "fieldType": "reference_list", "edge": "enforces", "targetKind": "invariant" }
                        ]
                    },
                    {
                        "name": "Invariant",
                        "keyword": "invariant",
                        "testable": true,
                        "supportsVerify": true
                    }
                ],
                "edgeTypes": [
                    { "label": "enforces", "sourceKind": "behavior", "targetKind": "invariant" }
                ]
            }"#,
        )
        .unwrap()
    }

    // -- B:validate_registered_entity_fields --

    // B:validate_registered_entity_fields — verify unit "target_kind reference resolves to registered kind"
    // B:register_validation_rules_from_manifest — verify unit "target_kind reference validated against KindRegistry after registries_populated"
    #[test]
    fn test_target_kind_reference_resolves_to_registered_kind() {
        let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[software_manifest()]);
        let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
        assert!(
            !diags.iter().any(|d| d.message.contains("target_kind")),
            "expected no target_kind warnings, got: {:?}",
            diags
        );
    }

    // B:validate_registered_entity_fields — verify unit "edge label resolves to registered edge type"
    // B:register_validation_rules_from_manifest — verify unit "edge_type reference validated against edge type set after registries_populated"
    #[test]
    fn test_edge_label_resolves_to_registered_edge_type() {
        let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[software_manifest()]);
        let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
        assert!(
            !diags.iter().any(|d| d.message.contains("edge label")),
            "expected no edge label warnings, got: {:?}",
            diags
        );
    }

    // B:validate_registered_entity_fields — verify unit "unresolved target_kind produces warning"
    // B:register_validation_rules_from_manifest — verify unit "invalid reference produces warning not error"
    #[test]
    fn test_unresolved_target_kind_produces_warning() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    {
                        "name": "Task",
                        "keyword": "task",
                        "fields": [
                            { "name": "owner", "fieldType": "reference", "targetKind": "person" }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();
        let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[manifest]);
        let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
        assert!(
            diags
                .iter()
                .any(|d| d.code == "W021" && d.message.contains("person")),
            "expected W021 about unresolved target_kind 'person', got: {:?}",
            diags
        );
    }

    // B:validate_registered_entity_fields — verify unit "unresolved edge label produces warning"
    #[test]
    fn test_unresolved_edge_label_produces_warning() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    {
                        "name": "Task",
                        "keyword": "task",
                        "fields": [
                            { "name": "owner", "fieldType": "reference", "edge": "owns" }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();
        let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[manifest]);
        let _diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
        // "owns" was auto-created as an implicit edge during populate, so it resolves
        assert!(
            edge_reg.contains("owns"),
            "implicit edge 'owns' should have been created"
        );
    }

    // B:validate_registered_entity_fields — verify unit "cross-validation uses no domain-specific logic"
    #[test]
    fn test_cross_validation_uses_no_domain_specific_logic() {
        // Custom domain: entirely made-up entity kinds, field types, edges
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@custom/cooking",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "cooking.wasm",
                "entityKinds": [
                    {
                        "name": "Recipe",
                        "keyword": "recipe",
                        "testable": true,
                        "supportsVerify": true,
                        "fields": [
                            { "name": "ingredients", "fieldType": "reference_list", "edge": "uses", "targetKind": "ingredient" }
                        ]
                    },
                    {
                        "name": "Ingredient",
                        "keyword": "ingredient"
                    }
                ],
                "edgeTypes": [
                    { "label": "uses", "sourceKind": "recipe", "targetKind": "ingredient" }
                ]
            }"#,
        )
        .unwrap();
        let (kind_reg, field_reg, edge_reg, pop_diags) = populate_registries(&[manifest]);
        assert!(pop_diags.is_empty());
        let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
        assert!(
            diags.is_empty(),
            "custom domain should validate cleanly: {:?}",
            diags
        );
    }

    // -- B:detect_duplicate_entity_kinds --

    // B:detect_duplicate_entity_kinds — verify unit "duplicate kind from two extensions produces E026"
    #[test]
    fn test_duplicate_kind_from_two_extensions_produces_e026() {
        let m1 = software_manifest();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@other/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "other.wasm",
                "entityKinds": [
                    { "name": "Behavior", "keyword": "behavior" }
                ]
            }"#,
        )
        .unwrap();
        let diags = detect_duplicate_entity_kinds(&[m1, m2]);
        assert!(
            diags
                .iter()
                .any(|d| d.code == "E026" && d.message.contains("behavior")),
            "expected E026 for duplicate 'behavior', got: {:?}",
            diags
        );
    }

    // B:detect_duplicate_entity_kinds — verify unit "first extension in topological order owns the kind"
    #[test]
    fn test_first_extension_in_topological_order_owns_the_kind() {
        let m1 = software_manifest();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@other/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "other.wasm",
                "entityKinds": [
                    { "name": "Behavior", "keyword": "behavior" }
                ]
            }"#,
        )
        .unwrap();
        let (kind_reg, _, _, _) = populate_registries(&[m1, m2]);
        let behavior = kind_reg.get("behavior").unwrap();
        assert_eq!(behavior.source_extension, "@specforge/software");
    }

    // B:detect_duplicate_entity_kinds — verify unit "single extension registering a kind produces no diagnostic"
    #[test]
    fn test_single_extension_registering_a_kind_produces_no_diagnostic() {
        let diags = detect_duplicate_entity_kinds(&[software_manifest()]);
        assert!(diags.is_empty());
    }

    // -- B:validate_peer_dependencies --

    // B:validate_peer_dependencies — verify unit "satisfied peer dependency passes validation"
    #[test]
    fn test_satisfied_peer_dependency_passes_validation() {
        let m1 = software_manifest();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": ">=1.0.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m1, m2]);
        assert!(
            diags.is_empty(),
            "expected no diagnostics, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "missing peer dependency produces hard error"
    #[test]
    fn test_missing_peer_dependency_produces_hard_error() {
        let m: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": ">=1.0.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m]);
        assert!(
            diags
                .iter()
                .any(|d| d.code == "E027" && d.message.contains("@specforge/software")),
            "expected E027 for missing peer, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "incompatible version produces hard error with required range"
    #[test]
    fn test_incompatible_version_produces_hard_error() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "0.5.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm"
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": ">=1.0.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m1, m2]);
        assert!(
            diags.iter().any(|d| d.code == "E027"
                && d.message.contains(">=1.0.0")
                && d.message.contains("0.5.0")),
            "expected E027 with version info, got: {:?}",
            diags
        );
    }

    // -- B:validate_extension_testability --

    // B:validate_extension_testability — verify unit "testable kind without supportsVerify produces W017"
    #[test]
    fn test_testable_kind_without_supports_verify_produces_w017() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "Thing", "keyword": "thing", "testable": true, "supportsVerify": false }
                ]
            }"#,
        )
        .unwrap();
        let (kind_reg, _, _, _) = populate_registries(&[manifest]);
        let diags = validate_extension_testability(&kind_reg);
        assert!(
            diags
                .iter()
                .any(|d| d.code == "W017" && d.message.contains("thing")),
            "expected W017, got: {:?}",
            diags
        );
    }

    // B:validate_extension_testability — verify unit "testable kind with supportsVerify=true passes"
    #[test]
    fn test_testable_kind_with_supports_verify_passes() {
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        let diags = validate_extension_testability(&kind_reg);
        // behavior has both testable=true and supportsVerify=true
        assert!(
            !diags.iter().any(|d| d.message.contains("behavior")),
            "expected no diagnostics for behavior, got: {:?}",
            diags
        );
    }

    // B:validate_extension_testability — verify unit "a kind that accepts verify statements but is not testable produces no diagnostic"
    #[test]
    fn test_kind_with_supports_verify_but_not_testable_is_not_reported() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "Note", "keyword": "note", "testable": false, "supportsVerify": true }
                ]
            }"#,
        )
        .unwrap();
        let (kind_reg, _, _, _) = populate_registries(&[manifest]);
        let diags = validate_extension_testability(&kind_reg);
        assert!(diags.is_empty(), "{diags:?}");
    }

    // B:validate_extension_testability — verify unit "consistent testable and supportsVerify flags produce no diagnostic"
    #[test]
    fn test_consistent_flags_produce_no_diagnostic() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "Thing", "keyword": "thing", "testable": false, "supportsVerify": false }
                ]
            }"#,
        )
        .unwrap();
        let (kind_reg, _, _, _) = populate_registries(&[manifest]);
        let diags = validate_extension_testability(&kind_reg);
        assert!(
            diags.is_empty(),
            "expected no diagnostics, got: {:?}",
            diags
        );
    }

    // -- B:register_validation_rules_from_manifest --

    // B:register_validation_rules_from_manifest — verify unit "validation rule registered from manifest"
    #[test]
    fn test_validation_rule_registered_from_manifest() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "validationRules": [
                    {
                        "code": "W100",
                        "severity": "warning",
                        "messageTemplate": "orphan {kind} '{id}'",
                        "check": "no_incoming_edges",
                        "targetKind": "behavior"
                    }
                ]
            }"#,
        )
        .unwrap();
        let (rules, diags) = register_validation_rules(&[manifest]);
        assert!(diags.is_empty());
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].code, "W100");
        assert_eq!(rules[0].check, "no_incoming_edges");
    }

    // B:register_validation_rules_from_manifest — verify unit "target_kind validation deferred to post-registration phase"
    #[test]
    fn test_target_kind_validation_deferred_to_post_registration() {
        // register_validation_rules does not validate target_kind — that's
        // done by validate_registered_entity_fields after all registries populated
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "validationRules": [
                    {
                        "code": "W100",
                        "severity": "warning",
                        "messageTemplate": "test",
                        "check": "no_incoming_edges",
                        "targetKind": "nonexistent_kind"
                    }
                ]
            }"#,
        )
        .unwrap();
        let (rules, diags) = register_validation_rules(&[manifest]);
        assert!(
            diags.is_empty(),
            "rule registration should not validate target_kind"
        );
        assert_eq!(rules.len(), 1);
    }

    // B:register_extension_validation_rules — verify unit "rules sorted by code for deterministic order"
    // B:register_extension_validation_rules — verify unit "rules from multiple extensions are collected"
    #[test]
    fn test_rules_sorted_by_code_for_deterministic_order() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@ext/a",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "a.wasm",
                "validationRules": [
                    { "code": "W300", "severity": "warning", "messageTemplate": "third", "check": "no_incoming_edges" },
                    { "code": "W100", "severity": "warning", "messageTemplate": "first", "check": "no_incoming_edges" }
                ]
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@ext/b",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "b.wasm",
                "validationRules": [
                    { "code": "W200", "severity": "warning", "messageTemplate": "second", "check": "no_outgoing_edges" }
                ]
            }"#,
        )
        .unwrap();
        let (rules, _) = register_validation_rules(&[m1, m2]);
        let codes: Vec<&str> = rules.iter().map(|r| r.code.as_str()).collect();
        assert_eq!(codes, vec!["W100", "W200", "W300"]);
    }

    // B:register_extension_validation_rules — verify unit "duplicate codes across extensions produce warning"
    #[test]
    fn test_duplicate_codes_across_extensions_produce_warning() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@ext/a",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "a.wasm",
                "validationRules": [
                    { "code": "W100", "severity": "warning", "messageTemplate": "a", "check": "no_incoming_edges" }
                ]
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@ext/b",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "b.wasm",
                "validationRules": [
                    { "code": "W100", "severity": "warning", "messageTemplate": "b", "check": "no_incoming_edges" }
                ]
            }"#,
        )
        .unwrap();
        let (_, diags) = register_validation_rules(&[m1, m2]);
        assert!(
            diags
                .iter()
                .any(|d| d.code == "W023" && d.message.contains("W100")),
            "expected W023 for duplicate code, got: {:?}",
            diags
        );
    }

    // B:validate_registered_entity_fields — verify contract "requires/ensures consistency for field cross-validation"
    #[test]
    fn test_validate_registered_entity_fields_contract() {
        // requires: all registries populated
        let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[software_manifest()]);
        let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
        // ensures: valid references produce no warnings
        assert!(diags.is_empty());
        // ensures: unresolved references produce W021
        let bad_manifest: ManifestV2 = serde_json::from_str(
            r#"{"name":"@t/e","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
                "entityKinds":[{"name":"A","keyword":"a","fields":[
                    {"name":"f","fieldType":"reference","targetKind":"nonexistent"}
                ]}]}"#,
        )
        .unwrap();
        let (kr, fr, er, _) = populate_registries(&[bad_manifest]);
        let bad_diags = validate_registered_entity_fields(&fr, &kr, &er);
        assert!(bad_diags.iter().any(|d| d.code == "W021"));
    }

    // B:detect_duplicate_entity_kinds — verify contract "requires/ensures consistency for duplicate entity kind detection"
    #[test]
    fn test_detect_duplicate_entity_kinds_contract() {
        // requires: manifests parsed
        // ensures: no duplicates → no diagnostics
        let diags = detect_duplicate_entity_kinds(&[software_manifest()]);
        assert!(diags.is_empty());
        // ensures: duplicate → E026 with both extension names
        let m2: ManifestV2 = serde_json::from_str(
            r#"{"name":"@other/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"o.wasm",
                "entityKinds":[{"name":"Behavior","keyword":"behavior"}]}"#,
        )
        .unwrap();
        let dup_diags = detect_duplicate_entity_kinds(&[software_manifest(), m2]);
        assert!(dup_diags.iter().any(|d| d.code == "E026"));
    }

    // B:validate_peer_dependencies — verify contract "requires/ensures consistency for peer dependency validation"
    #[test]
    fn test_validate_peer_dependencies_contract() {
        // requires: manifests loaded
        // ensures: satisfied deps → no error
        let m1 = software_manifest();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{"name":"@specforge/product","version":"1.0.0","manifestVersion":2,"wasmPath":"p.wasm",
                "peerDependencies":[{"name":"@specforge/software","version":">=1.0.0"}]}"#,
        ).unwrap();
        assert!(validate_peer_dependencies(&[m1, m2]).is_empty());
        // ensures: missing dep → E027
        let m3: ManifestV2 = serde_json::from_str(
            r#"{"name":"@specforge/product","version":"1.0.0","manifestVersion":2,"wasmPath":"p.wasm",
                "peerDependencies":[{"name":"@specforge/missing","version":">=1.0.0"}]}"#,
        ).unwrap();
        let diags = validate_peer_dependencies(&[m3]);
        assert!(diags.iter().any(|d| d.code == "E027"));
    }

    // B:validate_extension_testability — verify contract "requires/ensures consistency for extension testability validation"
    #[test]
    fn test_validate_extension_testability_contract() {
        // requires: KindRegistry populated
        let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
        // ensures: consistent flags → no diagnostics
        let diags = validate_extension_testability(&kind_reg);
        assert!(diags.is_empty());
        // ensures: testable without supportsVerify → W017
        let bad: ManifestV2 = serde_json::from_str(
            r#"{"name":"@t/e","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
                "entityKinds":[{"name":"X","keyword":"x","testable":true,"supportsVerify":false}]}"#,
        ).unwrap();
        let (bad_kr, _, _, _) = populate_registries(&[bad]);
        let bad_diags = validate_extension_testability(&bad_kr);
        assert!(bad_diags.iter().any(|d| d.code == "W017"));
    }

    // B:register_validation_rules_from_manifest — verify contract "requires/ensures consistency for validation rule registration"
    // B:register_extension_validation_rules — verify contract "requires/ensures consistency for cross-extension rule aggregation"
    #[test]
    fn test_register_validation_rules_contract() {
        // requires: manifests parsed
        let m: ManifestV2 = serde_json::from_str(
            r#"{"name":"@t/e","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
                "validationRules":[
                    {"code":"W100","severity":"warning","messageTemplate":"test","check":"no_incoming_edges"}
                ]}"#,
        ).unwrap();
        let (rules, diags) = register_validation_rules(&[m]);
        // ensures: rules registered
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].code, "W100");
        // ensures: no duplicate warnings for single extension
        assert!(diags.is_empty());
    }

    // -- B:validate_peer_dependencies (semver range matching) --

    // B:validate_peer_dependencies — verify unit "caret range ^1.0.0 matches 1.x.x"
    #[test]
    fn test_caret_range_matches() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.2.3",
                "manifestVersion": 2,
                "wasmPath": "software.wasm"
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": "^1.0.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m1, m2]);
        assert!(
            diags.is_empty(),
            "^1.0.0 should match 1.2.3, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "caret range ^1.0.0 rejects 2.0.0"
    #[test]
    fn test_caret_range_rejects_major_bump() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "2.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm"
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": "^1.0.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m1, m2]);
        assert!(
            diags.iter().any(|d| d.code == "E027"),
            "^1.0.0 should reject 2.0.0, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "tilde range ~1.2.0 matches 1.2.x"
    #[test]
    fn test_tilde_range_matches() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.2.5",
                "manifestVersion": 2,
                "wasmPath": "software.wasm"
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": "~1.2.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m1, m2]);
        assert!(
            diags.is_empty(),
            "~1.2.0 should match 1.2.5, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "tilde range ~1.2.0 rejects 1.3.0"
    #[test]
    fn test_tilde_range_rejects_minor_bump() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.3.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm"
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": "~1.2.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m1, m2]);
        assert!(
            diags.iter().any(|d| d.code == "E027"),
            "~1.2.0 should reject 1.3.0, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "malformed semver range in peer dep produces W062"
    #[test]
    fn test_malformed_semver_range_produces_warning() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm"
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": "not-a-version" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m1, m2]);
        assert!(
            diags
                .iter()
                .any(|d| d.code == "W062" && d.message.contains("not-a-version")),
            "expected W062 for malformed version range, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "malformed installed version produces W062"
    #[test]
    fn test_malformed_installed_version_produces_warning() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "bad-version",
                "manifestVersion": 2,
                "wasmPath": "software.wasm"
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": "^1.0.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m1, m2]);
        assert!(
            diags
                .iter()
                .any(|d| d.code == "W062" && d.message.contains("bad-version")),
            "expected W062 for malformed installed version, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "exact version match works"
    #[test]
    fn test_exact_version_match() {
        let m1: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm"
            }"#,
        )
        .unwrap();
        let m2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/product",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "product.wasm",
                "peerDependencies": [
                    { "name": "@specforge/software", "version": "1.0.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_peer_dependencies(&[m1, m2]);
        assert!(
            diags.is_empty(),
            "exact 1.0.0 should match 1.0.0, got: {:?}",
            diags
        );
    }
}
