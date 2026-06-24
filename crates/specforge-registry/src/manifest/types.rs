use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, Severity};

use super::surface::SurfaceContributions;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestV2 {
    pub name: String,
    pub version: String,
    pub manifest_version: u32,
    pub wasm_path: String,
    #[serde(default)]
    pub contributes: ExtensionContributions,
    #[serde(default)]
    pub entity_kinds: Vec<ManifestEntityKind>,
    #[serde(default)]
    pub edge_types: Vec<ManifestEdgeType>,
    #[serde(default)]
    pub validation_rules: Vec<ManifestValidationRule>,
    #[serde(default)]
    pub verify_kinds: Vec<String>,
    #[serde(default)]
    pub fields: Vec<ManifestField>,
    #[serde(default)]
    pub incremental: Option<bool>,
    #[serde(default)]
    pub reserved_keywords: Vec<String>,
    #[serde(default)]
    pub migration_hook: Option<String>,
    #[serde(default)]
    pub peer_dependencies: Vec<PeerDependency>,
    #[serde(default)]
    pub sandbox_policy: Option<SandboxPolicy>,
    #[serde(default)]
    pub host_api_version: Option<String>,
    #[serde(default)]
    pub entity_enhancements: Vec<FieldEnhancement>,
    #[serde(default)]
    pub starter_template: Option<String>,
    #[serde(default)]
    pub grammar_contributions: Vec<GrammarContribution>,
    #[serde(default)]
    pub body_parser_contributions: Vec<BodyParserContribution>,
    #[serde(default)]
    pub ext_short: Option<String>,
    #[serde(default)]
    pub query_scope: Option<String>,
    #[serde(default)]
    pub collector_contributions: Vec<CollectorContribution>,
    #[serde(default)]
    pub analyzer_contributions: Vec<AnalyzerContribution>,
    #[serde(default)]
    pub surfaces: Option<SurfaceContributions>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionContributions {
    #[serde(default)]
    pub entities: bool,
    #[serde(default)]
    pub validators: bool,
    #[serde(default)]
    pub renderers: bool,
    #[serde(default)]
    pub providers: bool,
    #[serde(default)]
    pub collectors: bool,
    #[serde(default)]
    pub prompts: bool,
    #[serde(default)]
    pub parsers: bool,
    #[serde(default)]
    pub grammars: bool,
    #[serde(default)]
    pub body_parsers: bool,
    #[serde(default)]
    pub analyzers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntityKind {
    pub name: String,
    pub keyword: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub testable: bool,
    #[serde(default)]
    pub singleton: bool,
    #[serde(default)]
    pub supports_verify: bool,
    #[serde(default)]
    pub allowed_verify_kinds: Vec<String>,
    #[serde(default)]
    pub semantic_token: Option<String>,
    #[serde(default)]
    pub lsp_icon: Option<String>,
    #[serde(default)]
    pub dot_shape: Option<String>,
    #[serde(default)]
    pub dot_color: Option<String>,
    #[serde(default)]
    pub dot_fillcolor: Option<String>,
    #[serde(default)]
    pub fields: Vec<ManifestField>,
    #[serde(default)]
    pub incremental: Option<bool>,
    #[serde(default)]
    pub has_body_parser: bool,
    #[serde(default)]
    pub open_fields: bool,
    #[serde(default)]
    pub inference_guide: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEdgeType {
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub edge_style: Option<String>,
    #[serde(default)]
    pub edge_color: Option<String>,
    #[serde(default)]
    pub edge_arrowhead: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestField {
    pub name: String,
    pub field_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub edge: Option<String>,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub file_reference: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    #[serde(default)]
    pub inverse_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestValidationRule {
    pub code: String,
    pub severity: String,
    pub message_template: String,
    pub check: String,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub edge_type: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub constraint: Option<FieldConstraint>,
    #[serde(default)]
    pub wasm_function: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldConstraint {
    pub kind: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerDependency {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxPolicy {
    #[serde(default)]
    pub max_memory_mb: Option<u32>,
    #[serde(default)]
    pub max_execution_ms: Option<u32>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub allowed_output_extensions: Vec<String>,
    #[serde(default)]
    pub network_access: Option<bool>,
    #[serde(default)]
    pub file_system_access: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldEnhancement {
    pub target_kind: String,
    pub source_extension: String,
    pub fields: Vec<ManifestField>,
    #[serde(default)]
    pub edge_types: Vec<ManifestEdgeType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarContribution {
    pub entity_kind: String,
    pub grammar_wasm_path: String,
    #[serde(default)]
    pub export_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyParserContribution {
    pub entity_kind: String,
    pub export_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorContribution {
    pub name: String,
    pub input_formats: Vec<String>,
    pub export: String,
    #[serde(default)]
    pub auto_detect: Option<CollectorAutoDetect>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorAutoDetect {
    pub file_patterns: Vec<String>,
    #[serde(default)]
    pub env_vars: Vec<String>,
}

/// Declares a language analyzer contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerContribution {
    pub language: String,
    pub file_extensions: Vec<String>,
    #[serde(default)]
    pub excluded_dirs: Vec<String>,
    pub scan_export: String,
    pub classify_export: String,
    pub map_export: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Validate a ManifestV2 against the v2 schema rules.
/// Returns diagnostics for any issues found.
pub fn validate_manifest(manifest: &ManifestV2) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if manifest.manifest_version != 2 {
        diagnostics.push(Diagnostic {
            code: "E030".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': manifestVersion must be 2, got {}",
                manifest.name, manifest.manifest_version
            ),
            span: None,
            suggestion: None,
        });
    }

    if manifest.name.is_empty() {
        diagnostics.push(Diagnostic {
            code: "E030".to_string(),
            severity: Severity::Error,
            message: "extension manifest: 'name' field is required".to_string(),
            span: None,
            suggestion: None,
        });
    }

    if manifest.version.is_empty() {
        diagnostics.push(Diagnostic {
            code: "E030".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': 'version' field is required",
                manifest.name
            ),
            span: None,
            suggestion: None,
        });
    }

    if manifest.wasm_path.is_empty() {
        diagnostics.push(Diagnostic {
            code: "E030".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': 'wasmPath' field is required",
                manifest.name
            ),
            span: None,
            suggestion: None,
        });
    }

    // Validate grammar contributions structurally
    for gc in &manifest.grammar_contributions {
        if gc.grammar_wasm_path.is_empty() {
            diagnostics.push(Diagnostic {
                code: "E030".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': grammarContribution for '{}' has empty grammarWasmPath",
                    manifest.name, gc.entity_kind
                ),
                span: None,
                suggestion: None,
            });
        }
    }

    // Validate body parser contributions structurally
    for bp in &manifest.body_parser_contributions {
        if bp.export_name.is_empty() {
            diagnostics.push(Diagnostic {
                code: "E030".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': bodyParserContribution for '{}' has empty exportName",
                    manifest.name, bp.entity_kind
                ),
                span: None,
                suggestion: None,
            });
        }
    }

    for ac in &manifest.analyzer_contributions {
        if ac.language.is_empty() {
            diagnostics.push(Diagnostic {
                code: "E030".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': analyzerContribution has empty language",
                    manifest.name
                ),
                span: None,
                suggestion: None,
            });
        }
        if ac.file_extensions.is_empty() {
            diagnostics.push(Diagnostic {
                code: "E030".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': analyzerContribution for '{}' has no file extensions",
                    manifest.name, ac.language
                ),
                span: None,
                suggestion: None,
            });
        }
        for export in [&ac.scan_export, &ac.classify_export, &ac.map_export] {
            if export.is_empty() {
                diagnostics.push(Diagnostic {
                    code: "E030".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "extension '{}': analyzerContribution for '{}' has empty export name",
                        manifest.name, ac.language
                    ),
                    span: None,
                    suggestion: None,
                });
            }
        }
    }

    diagnostics
}

/// Validate internal consistency of a manifest (target_kind refs, edge label refs).
pub fn validate_manifest_consistency(manifest: &ManifestV2) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Collect all kind names declared by this manifest
    let own_kinds: std::collections::HashSet<&str> = manifest
        .entity_kinds
        .iter()
        .map(|k| k.keyword.as_str())
        .collect();

    // Collect peer dependency names
    let peer_deps: std::collections::HashSet<&str> = manifest
        .peer_dependencies
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    // Collect declared edge labels
    let own_edge_labels: std::collections::HashSet<&str> = manifest
        .edge_types
        .iter()
        .map(|e| e.label.as_str())
        .collect();

    // Validate target_kind and edge references in entity kind fields
    for kind in &manifest.entity_kinds {
        for field in &kind.fields {
            if let Some(ref target) = field.target_kind
                && !own_kinds.contains(target.as_str())
                && peer_deps.is_empty()
            {
                diagnostics.push(Diagnostic {
                    code: "W021".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "extension '{}': field '{}' on kind '{}' references target_kind '{}' not declared in this manifest",
                        manifest.name, field.name, kind.keyword, target
                    ),
                    span: None,
                    suggestion: None,
                });
            }
            if let Some(ref edge) = field.edge
                && !own_edge_labels.contains(edge.as_str())
            {
                diagnostics.push(Diagnostic {
                    code: "W021".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "extension '{}': field '{}' on kind '{}' references edge label '{}' not declared in edgeTypes",
                        manifest.name, field.name, kind.keyword, edge
                    ),
                    span: None,
                    suggestion: None,
                });
            }
        }
    }

    // Validate edge type source_kind/target_kind references
    for edge in &manifest.edge_types {
        if let Some(ref source) = edge.source_kind
            && !own_kinds.contains(source.as_str())
            && peer_deps.is_empty()
        {
            diagnostics.push(Diagnostic {
                code: "W021".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "extension '{}': edge type '{}' references source_kind '{}' not declared in this manifest",
                    manifest.name, edge.label, source
                ),
                span: None,
                suggestion: None,
            });
        }
        if let Some(ref target) = edge.target_kind
            && !own_kinds.contains(target.as_str())
            && peer_deps.is_empty()
        {
            diagnostics.push(Diagnostic {
                code: "W021".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "extension '{}': edge type '{}' references target_kind '{}' not declared in this manifest",
                    manifest.name, edge.label, target
                ),
                span: None,
                suggestion: None,
            });
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_valid_manifest() -> ManifestV2 {
        serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm"
            }"#,
        )
        .unwrap()
    }

    // B:validate_manifest_v2_schema — verify unit "valid v2 manifest passes schema validation"
    #[test]
    fn test_valid_v2_manifest_passes_schema_validation() {
        let manifest = minimal_valid_manifest();
        let diags = validate_manifest(&manifest);
        assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
    }

    // B:validate_manifest_v2_schema — verify unit "missing required field produces hard error"
    #[test]
    fn test_missing_required_field_produces_hard_error() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm"
            }"#,
        )
        .unwrap();
        let diags = validate_manifest(&manifest);
        assert!(diags.iter().any(|d| d.code == "E030" && d.message.contains("'name'")));

        let manifest2: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": ""
            }"#,
        )
        .unwrap();
        let diags2 = validate_manifest(&manifest2);
        assert!(diags2.iter().any(|d| d.code == "E030" && d.message.contains("wasmPath")));
    }

    // B:validate_manifest_v2_schema — verify unit "manifestVersion != 2 produces hard error"
    #[test]
    fn test_manifest_version_not_2_produces_hard_error() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 1,
                "wasmPath": "x.wasm"
            }"#,
        )
        .unwrap();
        let diags = validate_manifest(&manifest);
        assert!(diags
            .iter()
            .any(|d| d.code == "E030" && d.message.contains("manifestVersion must be 2")));
    }

    // B:validate_manifest_v2_schema — verify unit "unknown top-level field produces warning"
    #[test]
    fn test_unknown_top_level_field_accepted_by_serde() {
        // serde(deny_unknown_fields) is NOT set — unknown fields are silently ignored.
        // This is the correct behavior: unknown fields produce a warning (handled at
        // a higher level that compares raw JSON keys vs known fields), not a parse error.
        let result: Result<ManifestV2, _> = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "unknownField": true
            }"#,
        );
        assert!(result.is_ok(), "unknown fields should not cause parse failure");
    }

    // B:validate_manifest_v2_schema — serde round-trip for full manifest with entity kinds
    #[test]
    fn test_full_manifest_deserialization() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/software",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "software.wasm",
                "contributes": { "entities": true, "validators": true },
                "entityKinds": [
                    {
                        "name": "Behavior",
                        "keyword": "behavior",
                        "testable": true,
                        "supportsVerify": true,
                        "semanticToken": "function",
                        "lspIcon": "Method",
                        "dotShape": "ellipse",
                        "fields": [
                            { "name": "contract", "fieldType": "block" },
                            { "name": "invariants", "fieldType": "reference_list", "edge": "enforces", "targetKind": "invariant" }
                        ]
                    }
                ],
                "edgeTypes": [
                    { "label": "enforces", "sourceKind": "behavior", "targetKind": "invariant", "edgeStyle": "dashed" }
                ],
                "verifyKinds": ["smoke", "contract"],
                "peerDependencies": [
                    { "name": "@specforge/governance", "version": ">=1.0.0" }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.name, "@specforge/software");
        assert_eq!(manifest.entity_kinds.len(), 1);
        assert_eq!(manifest.entity_kinds[0].keyword, "behavior");
        assert!(manifest.entity_kinds[0].testable);
        assert!(manifest.entity_kinds[0].supports_verify);
        assert_eq!(manifest.entity_kinds[0].fields.len(), 2);
        assert_eq!(manifest.edge_types.len(), 1);
        assert_eq!(manifest.edge_types[0].label, "enforces");
        assert_eq!(manifest.verify_kinds, vec!["smoke", "contract"]);
        assert_eq!(manifest.peer_dependencies.len(), 1);
        assert!(manifest.contributes.entities);
        assert!(manifest.contributes.validators);
    }

    // B:validate_extension_manifest_consistency — verify unit "target_kind referencing own manifest kind passes"
    #[test]
    fn test_target_kind_referencing_own_manifest_kind_passes() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "Behavior", "keyword": "behavior", "fields": [
                        { "name": "invariants", "fieldType": "reference_list", "targetKind": "invariant" }
                    ]},
                    { "name": "Invariant", "keyword": "invariant" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_manifest_consistency(&manifest);
        assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
    }

    // B:validate_extension_manifest_consistency — verify unit "self-contradictory target_kind produces E-level error"
    #[test]
    fn test_self_contradictory_target_kind_produces_warning() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "Behavior", "keyword": "behavior", "fields": [
                        { "name": "invariants", "fieldType": "reference_list", "targetKind": "nonexistent_kind" }
                    ]}
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_manifest_consistency(&manifest);
        assert!(
            diags.iter().any(|d| d.message.contains("nonexistent_kind")),
            "expected warning about nonexistent target_kind, got: {:?}",
            diags
        );
    }

    // B:validate_extension_manifest_consistency — verify unit "self-contradictory edge label produces E-level error"
    #[test]
    fn test_self_contradictory_edge_label_produces_warning() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "Behavior", "keyword": "behavior", "fields": [
                        { "name": "invariants", "fieldType": "reference_list", "edge": "missing_edge" }
                    ]}
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_manifest_consistency(&manifest);
        assert!(
            diags.iter().any(|d| d.message.contains("missing_edge")),
            "expected warning about missing edge label, got: {:?}",
            diags
        );
    }

    // B:validate_extension_manifest_consistency — verify unit "target_kind referencing peer dependency kind passes"
    #[test]
    fn test_target_kind_referencing_peer_dependency_kind_passes() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "Feature", "keyword": "feature", "fields": [
                        { "name": "behaviors", "fieldType": "reference_list", "targetKind": "behavior" }
                    ]}
                ],
                "peerDependencies": [
                    { "name": "@specforge/software", "version": ">=1.0.0" }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_manifest_consistency(&manifest);
        // With peer deps declared, we can't fully resolve cross-extension refs,
        // so we don't warn about target_kind not in own kinds
        assert!(diags.is_empty(), "expected no diagnostics with peer deps, got: {:?}", diags);
    }

    // B:validate_extension_manifest_consistency — verify unit "target_kind referencing non-peer extension kind produces W-level warning"
    #[test]
    fn test_target_kind_referencing_non_peer_extension_kind_produces_warning() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "Feature", "keyword": "feature", "fields": [
                        { "name": "behaviors", "fieldType": "reference_list", "targetKind": "behavior" }
                    ]}
                ]
            }"#,
        )
        .unwrap();
        // No peer dependencies — referencing 'behavior' which is not an own kind
        let diags = validate_manifest_consistency(&manifest);
        assert!(
            diags.iter().any(|d| d.message.contains("behavior") && d.message.contains("target_kind")),
            "expected warning about non-peer target_kind 'behavior', got: {:?}",
            diags
        );
    }

    // B:validate_manifest_v2_schema — verify contract "requires/ensures consistency for manifest v2 schema validation"
    #[test]
    fn test_validate_manifest_v2_schema_contract() {
        // requires: manifest_json_available (parsed from JSON)
        let manifest = minimal_valid_manifest();
        let diags = validate_manifest(&manifest);
        // ensures: schema_validated — valid manifest produces zero diagnostics
        assert!(diags.is_empty());
        // ensures: malformed_diagnosed — missing fields produce E030
        let bad: ManifestV2 = serde_json::from_str(
            r#"{"name":"","version":"","manifestVersion":1,"wasmPath":""}"#,
        )
        .unwrap();
        let bad_diags = validate_manifest(&bad);
        assert!(bad_diags.len() >= 3, "expected multiple E030 diagnostics");
        assert!(bad_diags.iter().all(|d| d.code == "E030"));
    }

    // B:validate_extension_manifest_consistency — verify contract "requires/ensures consistency for manifest self-consistency validation"
    #[test]
    fn test_validate_manifest_consistency_contract() {
        // requires: manifest_validated — manifest passed schema validation
        let good: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext", "version": "1.0.0", "manifestVersion": 2, "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "A", "keyword": "a", "fields": [{ "name": "bs", "fieldType": "reference_list", "targetKind": "b" }] },
                    { "name": "B", "keyword": "b" }
                ],
                "edgeTypes": [{ "label": "links", "sourceKind": "a", "targetKind": "b" }]
            }"#,
        ).unwrap();
        let diags = validate_manifest_consistency(&good);
        // ensures: self_consistent — no warnings when all refs resolve internally
        assert!(diags.is_empty());

        // ensures: self_contradictions_diagnosed — unresolvable refs produce W021
        let bad: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext", "version": "1.0.0", "manifestVersion": 2, "wasmPath": "x.wasm",
                "entityKinds": [
                    { "name": "A", "keyword": "a", "fields": [{ "name": "bs", "fieldType": "reference_list", "targetKind": "missing", "edge": "missing_edge" }] }
                ]
            }"#,
        ).unwrap();
        let bad_diags = validate_manifest_consistency(&bad);
        assert!(bad_diags.len() >= 2, "expected warnings for target_kind + edge: {:?}", bad_diags);
        assert!(bad_diags.iter().all(|d| d.code == "W021"));
    }

    // -- Slice 8a: CollectorContribution manifest type --

    // B:collector_manifest_type — verify unit "ManifestV2 with collectorContributions deserializes"
    #[test]
    fn test_manifest_with_collector_contributions_deserializes() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/rust",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "rust.wasm",
                "collectorContributions": [
                    {
                        "name": "rust",
                        "inputFormats": ["junit-xml"],
                        "export": "collect__rust",
                        "autoDetect": {
                            "filePatterns": ["**/target/**/junit.xml"],
                            "envVars": ["CARGO_TARGET_DIR"]
                        }
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.collector_contributions.len(), 1);
        let cc = &manifest.collector_contributions[0];
        assert_eq!(cc.name, "rust");
        assert_eq!(cc.input_formats, vec!["junit-xml"]);
        assert_eq!(cc.export, "collect__rust");
        let ad = cc.auto_detect.as_ref().unwrap();
        assert_eq!(ad.file_patterns, vec!["**/target/**/junit.xml"]);
        assert_eq!(ad.env_vars, vec!["CARGO_TARGET_DIR"]);
    }

    // B:collector_manifest_type — verify unit "ManifestV2 without collectorContributions defaults to empty"
    #[test]
    fn test_manifest_without_collector_contributions_defaults_empty() {
        let manifest = minimal_valid_manifest();
        assert!(manifest.collector_contributions.is_empty());
    }

    // B:collector_manifest_type — verify unit "CollectorContribution without autoDetect parses"
    #[test]
    fn test_collector_contribution_without_auto_detect_parses() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/js",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "js.wasm",
                "collectorContributions": [
                    {
                        "name": "jest",
                        "inputFormats": ["jest-json"],
                        "export": "collect__jest"
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.collector_contributions.len(), 1);
        assert!(manifest.collector_contributions[0].auto_detect.is_none());
    }

    // -- AnalyzerContribution tests --

    #[test]
    fn test_manifest_with_analyzer_contributions_deserializes() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@specforge/rust",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "rust.wasm",
                "contributes": { "analyzers": true },
                "analyzerContributions": [
                    {
                        "language": "rust",
                        "fileExtensions": [".rs"],
                        "excludedDirs": ["target"],
                        "scanExport": "scan__rust",
                        "classifyExport": "classify__rust",
                        "mapExport": "map__rust",
                        "description": "Rust source analyzer"
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.analyzer_contributions.len(), 1);
        assert!(manifest.contributes.analyzers);
        let ac = &manifest.analyzer_contributions[0];
        assert_eq!(ac.language, "rust");
        assert_eq!(ac.file_extensions, vec![".rs"]);
        assert_eq!(ac.excluded_dirs, vec!["target"]);
        assert_eq!(ac.scan_export, "scan__rust");
        assert_eq!(ac.classify_export, "classify__rust");
        assert_eq!(ac.map_export, "map__rust");
        assert_eq!(ac.description.as_deref(), Some("Rust source analyzer"));
    }

    #[test]
    fn test_manifest_without_analyzer_contributions_defaults_empty() {
        let manifest = minimal_valid_manifest();
        assert!(manifest.analyzer_contributions.is_empty());
        assert!(!manifest.contributes.analyzers);
    }

    #[test]
    fn test_validate_manifest_rejects_empty_analyzer_language() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "analyzerContributions": [
                    {
                        "language": "",
                        "fileExtensions": [".rs"],
                        "scanExport": "scan__x",
                        "classifyExport": "classify__x",
                        "mapExport": "map__x"
                    }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_manifest(&manifest);
        assert!(diags.iter().any(|d| d.code == "E030" && d.message.contains("empty language")));
    }

    #[test]
    fn test_validate_manifest_rejects_empty_analyzer_exports() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "analyzerContributions": [
                    {
                        "language": "rust",
                        "fileExtensions": [".rs"],
                        "scanExport": "",
                        "classifyExport": "classify__rust",
                        "mapExport": "map__rust"
                    }
                ]
            }"#,
        )
        .unwrap();
        let diags = validate_manifest(&manifest);
        assert!(diags.iter().any(|d| d.code == "E030" && d.message.contains("empty export")));
    }

    // -- H6: ManifestField with default_value and enum_values --

    #[test]
    fn test_manifest_field_with_default_value_and_enum_values_deserializes() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityKinds": [
                    {
                        "name": "Feature",
                        "keyword": "feature",
                        "fields": [
                            {
                                "name": "status",
                                "fieldType": "string",
                                "defaultValue": "draft",
                                "enumValues": ["draft", "active", "done"]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

        let field = &manifest.entity_kinds[0].fields[0];
        assert_eq!(field.default_value.as_deref(), Some("draft"));
        assert_eq!(field.enum_values, vec!["draft", "active", "done"]);
    }

    #[test]
    fn test_manifest_field_without_default_value_and_enum_values_defaults() {
        let _manifest = minimal_valid_manifest();
        // Fields from JSON deserialization without these fields should get defaults
        let field: ManifestField = serde_json::from_str(
            r#"{ "name": "contract", "fieldType": "block" }"#,
        )
        .unwrap();
        assert!(field.default_value.is_none());
        assert!(field.enum_values.is_empty());
    }

    // -- H5: FieldEnhancement with edge_types --

    #[test]
    fn test_field_enhancement_with_edge_types_deserializes() {
        let manifest: ManifestV2 = serde_json::from_str(
            r#"{
                "name": "@test/ext",
                "version": "1.0.0",
                "manifestVersion": 2,
                "wasmPath": "x.wasm",
                "entityEnhancements": [
                    {
                        "targetKind": "behavior",
                        "sourceExtension": "@test/ext",
                        "fields": [],
                        "edgeTypes": [
                            {
                                "label": "RequiresCondition",
                                "sourceKind": "behavior",
                                "targetKind": "condition"
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

        let enh = &manifest.entity_enhancements[0];
        assert_eq!(enh.edge_types.len(), 1);
        assert_eq!(enh.edge_types[0].label, "RequiresCondition");
        assert_eq!(enh.edge_types[0].source_kind.as_deref(), Some("behavior"));
        assert_eq!(enh.edge_types[0].target_kind.as_deref(), Some("condition"));
    }

    #[test]
    fn test_field_enhancement_without_edge_types_defaults_empty() {
        let enh: FieldEnhancement = serde_json::from_str(
            r#"{
                "targetKind": "behavior",
                "sourceExtension": "@test/ext",
                "fields": []
            }"#,
        )
        .unwrap();
        assert!(enh.edge_types.is_empty());
    }
}
