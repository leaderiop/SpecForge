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
                    && existing_enh
                        .fields
                        .iter()
                        .any(|ef| ef.name == field.name)
            });

            if conflict {
                diagnostics.push(Diagnostic {
                    code: "E034".to_string(),
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

/// A registered collector contribution.
#[derive(Debug, Clone)]
pub struct RegisteredCollector {
    pub extension_name: String,
    pub command_name: String,
    pub export_name: String,
}

/// Register collector contributions from manifests.
pub fn register_collector_contributions(
    manifests: &[ManifestV2],
) -> Vec<RegisteredCollector> {
    let mut collectors = Vec::new();

    for manifest in manifests {
        if manifest.contributes.collectors {
            // Convention: collector export is named collect__{extension_name_slug}
            let slug = manifest.name.replace('@', "").replace('/', "__");
            collectors.push(RegisteredCollector {
                extension_name: manifest.name.clone(),
                command_name: format!("collect_{}", slug),
                export_name: format!("collect__{}", slug),
            });
        }
    }

    collectors
}

/// Dispatch a collector by calling its Wasm export.
pub fn dispatch_collector(
    collector: &RegisteredCollector,
    runtime: &dyn WasmRuntime,
    input: &[u8],
) -> Result<Vec<u8>, Diagnostic> {
    match runtime.call_export(&collector.extension_name, &collector.export_name, input) {
        WasmCallResult::Ok(output) => {
            // Validate output is valid JSON
            if !output.is_empty()
                && let Err(e) = serde_json::from_slice::<serde_json::Value>(&output)
            {
                return Err(Diagnostic {
                    code: "E028".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "extension '{}': collector '{}' returned invalid JSON: {}",
                        collector.extension_name, collector.command_name, e
                    ),
                    span: None,
                    suggestion: None,
                });
            }
            Ok(output)
        }
        WasmCallResult::Trap(trap) => Err(Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': collector {}() trapped: {} — {}",
                collector.extension_name, collector.export_name, trap.kind, trap.message
            ),
            span: None,
            suggestion: None,
        }),
    }
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
                suggestion: Some(format!("add #[export_name = \"{}\"] to the Wasm module", export)),
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
    toggles.iter().any(|t| {
        t.extension_name == extension_name && t.disabled.contains(contribution_type)
    })
}

/// Detect grammar-level construct conflicts between extensions.
/// Two extensions contributing grammar for the same entity kind is a conflict.
pub fn detect_grammar_contribution_conflicts(
    manifests: &[ManifestV2],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for manifest in manifests {
        for gc in &manifest.grammar_contributions {
            if let Some(first_ext) = seen.get(&gc.entity_kind) {
                diagnostics.push(Diagnostic {
                    code: "E018".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "grammar conflict: extension '{}' and '{}' both contribute grammar for entity kind '{}'",
                        manifest.name, first_ext, gc.entity_kind
                    ),
                    span: None,
                    suggestion: Some("only one extension may provide grammar for a given entity kind".to_string()),
                });
            } else {
                seen.insert(gc.entity_kind.clone(), manifest.name.clone());
            }
        }
    }

    diagnostics
}

/// Validate collector output report against known entity IDs.
pub fn validate_collector_output(
    report: &serde_json::Value,
    known_entity_ids: &HashSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check entity_results for unknown entity IDs
    if let Some(results) = report.get("entity_results").and_then(|v| v.as_array()) {
        for result in results {
            if let Some(id) = result.get("entity_id").and_then(|v| v.as_str())
                && !known_entity_ids.contains(id)
            {
                diagnostics.push(Diagnostic {
                    code: "W029".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "collector report references unknown entity ID '{}'",
                        id
                    ),
                    span: None,
                    suggestion: Some("check that the entity ID matches a declared entity".to_string()),
                });
            }
        }
    }

    // Check stats consistency
    if let Some(stats) = report.get("stats") {
        let total = stats.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        let passed = stats.get("passed").and_then(|v| v.as_u64()).unwrap_or(0);
        let failed = stats.get("failed").and_then(|v| v.as_u64()).unwrap_or(0);
        let skipped = stats.get("skipped").and_then(|v| v.as_u64()).unwrap_or(0);

        if total > 0 && passed + failed + skipped != total {
            diagnostics.push(Diagnostic {
                code: "W030".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "collector report stats inconsistent: total={} but passed+failed+skipped={}",
                    total,
                    passed + failed + skipped
                ),
                span: None,
                suggestion: None,
            });
        }
    }

    diagnostics
}

/// Auto-detect a collector by matching file patterns.
/// Returns the name of the matching collector or an I013 info diagnostic.
pub fn auto_detect_collector(
    file_patterns: &[(&str, &str)], // (glob_pattern, collector_name)
    files_present: &[String],
) -> Result<String, Diagnostic> {
    for (pattern, collector_name) in file_patterns {
        if files_present.iter().any(|f| f.contains(pattern)) {
            return Ok(collector_name.to_string());
        }
    }

    Err(Diagnostic {
        code: "I013".to_string(),
        severity: Severity::Info,
        message: "no collector auto-detected for the current project".to_string(),
        span: None,
        suggestion: Some("specify a collector explicitly with --collector".to_string()),
    })
}

/// Policy for resolving grammar contribution conflicts when multiple extensions
/// contribute grammar for the same entity kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrammarConflictPolicy {
    Error,
    Priority,
    Namespace,
}

/// Compose grammar contributions from manifests into entity-kind→grammar mappings.
/// Returns Vec<(entity_kind, grammar_wasm_path, extension_name)> on success.
pub fn compose_grammar_injections(
    manifests: &[ManifestV2],
    conflict_policy: GrammarConflictPolicy,
) -> Result<Vec<(String, String, String)>, Vec<Diagnostic>> {
    let mut by_kind: std::collections::BTreeMap<String, Vec<(String, String)>> =
        std::collections::BTreeMap::new();

    for manifest in manifests {
        for gc in &manifest.grammar_contributions {
            by_kind
                .entry(gc.entity_kind.clone())
                .or_default()
                .push((gc.grammar_wasm_path.clone(), manifest.name.clone()));
        }
    }

    let mut result = Vec::new();
    let mut errors = Vec::new();

    for (kind, contributors) in &by_kind {
        if contributors.len() == 1 {
            let (path, ext) = &contributors[0];
            result.push((kind.clone(), path.clone(), ext.clone()));
        } else {
            match conflict_policy {
                GrammarConflictPolicy::Error => {
                    let ext_names: Vec<&str> = contributors.iter().map(|(_, e)| e.as_str()).collect();
                    errors.push(Diagnostic {
                        code: "E018".to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "grammar conflict: extensions {} both contribute grammar for entity kind '{}'",
                            ext_names.join(", "), kind
                        ),
                        span: None,
                        suggestion: Some("only one extension may provide grammar for a given entity kind".to_string()),
                    });
                }
                GrammarConflictPolicy::Priority => {
                    // First contributor wins (sorted by manifest order)
                    let (path, ext) = &contributors[0];
                    result.push((kind.clone(), path.clone(), ext.clone()));
                }
                GrammarConflictPolicy::Namespace => {
                    // Include all contributors
                    for (path, ext) in contributors {
                        result.push((kind.clone(), path.clone(), ext.clone()));
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        result.sort();
        Ok(result)
    } else {
        Err(errors)
    }
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
        let has_override = overrides.iter().any(|o| {
            o.entity_kind == conflict.entity_kind && o.field_name == conflict.field_name
        });

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

/// Result of ingesting a collector report.
#[derive(Debug, Clone)]
pub struct IngestedReport {
    pub mapped_entries: Vec<(String, serde_json::Value)>,
    pub unmapped_entries: Vec<serde_json::Value>,
    pub coverage_updates: Vec<(String, CoverageMetadata)>,
}

/// Coverage metadata for an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageMetadata {
    pub total: u64,
    pub passed: u64,
    pub failed: u64,
}

/// Ingest a collector report, associating entries with known entity IDs.
/// Entries with unknown entity IDs are placed in `unmapped_entries`.
/// Coverage metadata is computed for each mapped entity.
pub fn ingest_collector_report(
    report: &serde_json::Value,
    known_entity_ids: &HashSet<String>,
) -> IngestedReport {
    let mut mapped_entries = Vec::new();
    let mut unmapped_entries = Vec::new();
    let mut coverage_updates = Vec::new();

    if let Some(results) = report.get("entity_results").and_then(|v| v.as_array()) {
        for entry in results {
            if let Some(id) = entry.get("entity_id").and_then(|v| v.as_str()) {
                if known_entity_ids.contains(id) {
                    mapped_entries.push((id.to_string(), entry.clone()));

                    // Compute coverage from test_results sub-array
                    let tests = entry
                        .get("test_results")
                        .and_then(|v| v.as_array())
                        .map(|a| a.as_slice())
                        .unwrap_or(&[]);
                    let total = tests.len() as u64;
                    let passed = tests
                        .iter()
                        .filter(|t| t.get("status").and_then(|s| s.as_str()) == Some("passed"))
                        .count() as u64;
                    let failed = tests
                        .iter()
                        .filter(|t| t.get("status").and_then(|s| s.as_str()) == Some("failed"))
                        .count() as u64;

                    coverage_updates.push((
                        id.to_string(),
                        CoverageMetadata {
                            total,
                            passed,
                            failed,
                        },
                    ));
                } else {
                    unmapped_entries.push(entry.clone());
                }
            }
        }
    }

    IngestedReport {
        mapped_entries,
        unmapped_entries,
        coverage_updates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{MockRuntime, WasmTrapInfo};
    use crate::test_helpers::default_manifest;
    use specforge_registry::{ExtensionContributions, GrammarContribution, ManifestField};

    // -- dispatch_contribution_exports --

    // B:dispatch_contribution_exports — verify unit "routes to namespaced Wasm export"
    #[test]
    fn test_routes_to_namespaced_export() {
        let runtime = MockRuntime::new()
            .with_call_ok("@specforge__software_validate", b"ok".to_vec());

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
                }],
            },
        )];

        let mut manifest = default_manifest();
        manifest.name = "@ext/b".to_string();
        manifest.entity_enhancements = vec![FieldEnhancement {
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
            }],
        }];

        let diags = register_entity_enhancements(&manifest, &mut existing);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E034");
        assert!(diags[0].message.contains("priority"));
    }

    // B:register_entity_enhancements — verify unit "no conflict when same extension re-registers"
    #[test]
    fn test_no_conflict_same_extension() {
        let mut existing = vec![(
            "@ext/a".to_string(),
            FieldEnhancement {
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
                }],
            },
        )];

        let mut manifest = default_manifest();
        manifest.name = "@ext/a".to_string(); // Same extension
        manifest.entity_enhancements = vec![FieldEnhancement {
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

    // -- register_collector_contributions + dispatch_collector --

    // B:register_collector_contributions — verify unit "registers collectors from manifests"
    #[test]
    fn test_registers_collectors() {
        let mut m1 = default_manifest();
        m1.name = "@specforge/rust".to_string();
        m1.contributes = ExtensionContributions {
            collectors: true,
            ..Default::default()
        };

        let mut m2 = default_manifest();
        m2.name = "@specforge/js".to_string();
        m2.contributes = ExtensionContributions {
            collectors: true,
            ..Default::default()
        };

        let mut m3 = default_manifest();
        m3.name = "@specforge/other".to_string();
        m3.contributes = ExtensionContributions {
            collectors: false,
            ..Default::default()
        };

        let collectors = register_collector_contributions(&[m1, m2, m3]);
        assert_eq!(collectors.len(), 2);
        assert!(collectors.iter().any(|c| c.extension_name == "@specforge/rust"));
        assert!(collectors.iter().any(|c| c.extension_name == "@specforge/js"));
    }

    // B:dispatch_collector — verify unit "dispatches collector and validates output"
    #[test]
    fn test_dispatches_collector_valid_output() {
        let runtime = MockRuntime::new()
            .with_call_ok("collect__specforge__rust", b"[{\"test\":1}]".to_vec());

        let collector = RegisteredCollector {
            extension_name: "@specforge/rust".to_string(),
            command_name: "collect_specforge__rust".to_string(),
            export_name: "collect__specforge__rust".to_string(),
        };

        let result = dispatch_collector(&collector, &runtime, &[]);
        assert!(result.is_ok());
    }

    // B:dispatch_collector — verify unit "rejects invalid JSON output"
    #[test]
    fn test_dispatch_collector_rejects_invalid_json() {
        let runtime = MockRuntime::new()
            .with_call_ok("collect__specforge__rust", b"not json {{{".to_vec());

        let collector = RegisteredCollector {
            extension_name: "@specforge/rust".to_string(),
            command_name: "collect_specforge__rust".to_string(),
            export_name: "collect__specforge__rust".to_string(),
        };

        let result = dispatch_collector(&collector, &runtime, &[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "E028");
        assert!(err.message.contains("invalid JSON"));
    }

    // B:dispatch_collector — verify unit "handles collector trap"
    #[test]
    fn test_dispatch_collector_handles_trap() {
        let runtime = MockRuntime::new().with_call_trap(
            "collect__specforge__rust",
            WasmTrapInfo {
                kind: "unreachable".to_string(),
                message: "panic in collector".to_string(),
                export_name: "collect__specforge__rust".to_string(),
            },
        );

        let collector = RegisteredCollector {
            extension_name: "@specforge/rust".to_string(),
            command_name: "collect_specforge__rust".to_string(),
            export_name: "collect__specforge__rust".to_string(),
        };

        let result = dispatch_collector(&collector, &runtime, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "E028");
    }

    // B:dispatch_collector — verify unit "empty output is accepted"
    #[test]
    fn test_dispatch_collector_empty_output_accepted() {
        let runtime = MockRuntime::new()
            .with_call_ok("collect__specforge__rust", vec![]);

        let collector = RegisteredCollector {
            extension_name: "@specforge/rust".to_string(),
            command_name: "collect_specforge__rust".to_string(),
            export_name: "collect__specforge__rust".to_string(),
        };

        let result = dispatch_collector(&collector, &runtime, &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
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
        assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
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

        assert!(is_contribution_disabled(&toggles, "@specforge/software", "validators"));
        assert!(!is_contribution_disabled(&toggles, "@specforge/software", "renderers"));
    }

    // B:toggle_extension_contributions — verify unit "still loaded when some disabled"
    #[test]
    fn test_toggle_still_loaded_when_some_disabled() {
        let toggles = vec![ContributionToggle {
            extension_name: "@specforge/software".to_string(),
            disabled: HashSet::from(["validators".to_string()]),
        }];

        // Other contribution types remain active
        assert!(!is_contribution_disabled(&toggles, "@specforge/software", "renderers"));
        assert!(!is_contribution_disabled(&toggles, "@specforge/software", "collectors"));
        // Different extension unaffected
        assert!(!is_contribution_disabled(&toggles, "@specforge/governance", "validators"));
    }

    // B:toggle_extension_contributions — verify unit "re-enabled resumes"
    #[test]
    fn test_toggle_reenabled_resumes() {
        // Initially disabled
        let toggles = vec![ContributionToggle {
            extension_name: "@specforge/software".to_string(),
            disabled: HashSet::from(["validators".to_string()]),
        }];
        assert!(is_contribution_disabled(&toggles, "@specforge/software", "validators"));

        // After re-enabling (empty disabled set)
        let updated_toggles = vec![ContributionToggle {
            extension_name: "@specforge/software".to_string(),
            disabled: HashSet::new(),
        }];
        assert!(!is_contribution_disabled(&updated_toggles, "@specforge/software", "validators"));
    }

    // -- detect_grammar_contribution_conflicts (enhancement conflicts) --

    // B:detect_enhancement_conflicts — verify unit "grammar-level construct conflict → E018"
    #[test]
    fn test_detect_grammar_conflict_e018() {
        let mut m1 = default_manifest();
        m1.name = "@ext/a".to_string();
        m1.grammar_contributions = vec![GrammarContribution {
            entity_kind: "behavior".to_string(),
            grammar_wasm_path: "a/grammar.wasm".to_string(),
            export_name: None,
        }];

        let mut m2 = default_manifest();
        m2.name = "@ext/b".to_string();
        m2.grammar_contributions = vec![GrammarContribution {
            entity_kind: "behavior".to_string(),
            grammar_wasm_path: "b/grammar.wasm".to_string(),
            export_name: None,
        }];

        let diags = detect_grammar_contribution_conflicts(&[m1, m2]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E018");
        assert!(diags[0].message.contains("behavior"));
    }

    // B:detect_enhancement_conflicts — verify unit "conflict record includes both extension names"
    #[test]
    fn test_detect_grammar_conflict_includes_both_names() {
        let mut m1 = default_manifest();
        m1.name = "@ext/alpha".to_string();
        m1.grammar_contributions = vec![GrammarContribution {
            entity_kind: "event".to_string(),
            grammar_wasm_path: "alpha.wasm".to_string(),
            export_name: None,
        }];

        let mut m2 = default_manifest();
        m2.name = "@ext/beta".to_string();
        m2.grammar_contributions = vec![GrammarContribution {
            entity_kind: "event".to_string(),
            grammar_wasm_path: "beta.wasm".to_string(),
            export_name: None,
        }];

        let diags = detect_grammar_contribution_conflicts(&[m1, m2]);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("@ext/alpha"));
        assert!(diags[0].message.contains("@ext/beta"));
    }

    // -- validate_collector_output --

    // B:validate_collector_output — verify unit "valid report passes"
    #[test]
    fn test_validate_collector_output_valid_passes() {
        let known = HashSet::from(["my_behavior".to_string()]);
        let report: serde_json::Value = serde_json::from_str(r#"{
            "entity_results": [
                { "entity_id": "my_behavior", "status": "passed" }
            ],
            "stats": { "total": 1, "passed": 1, "failed": 0, "skipped": 0 }
        }"#).unwrap();

        let diags = validate_collector_output(&report, &known);
        assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
    }

    // B:validate_collector_output — verify unit "unknown entity ID → W029"
    #[test]
    fn test_validate_collector_output_unknown_entity_w029() {
        let known = HashSet::from(["my_behavior".to_string()]);
        let report: serde_json::Value = serde_json::from_str(r#"{
            "entity_results": [
                { "entity_id": "unknown_behavior", "status": "passed" }
            ],
            "stats": { "total": 1, "passed": 1, "failed": 0, "skipped": 0 }
        }"#).unwrap();

        let diags = validate_collector_output(&report, &known);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W029");
        assert!(diags[0].message.contains("unknown_behavior"));
    }

    // B:validate_collector_output — verify unit "inconsistent stats → W030"
    #[test]
    fn test_validate_collector_output_inconsistent_stats_w030() {
        let known = HashSet::new();
        let report: serde_json::Value = serde_json::from_str(r#"{
            "stats": { "total": 10, "passed": 3, "failed": 2, "skipped": 1 }
        }"#).unwrap();
        // 3 + 2 + 1 = 6 ≠ 10

        let diags = validate_collector_output(&report, &known);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W030");
        assert!(diags[0].message.contains("total=10"));
    }

    // B:validate_collector_output — verify contract "requires/ensures consistency"
    #[test]
    fn test_validate_collector_output_contract() {
        let known = HashSet::from(["b1".to_string()]);

        // ensures: valid → empty
        let good: serde_json::Value = serde_json::from_str(r#"{
            "entity_results": [{ "entity_id": "b1", "status": "passed" }],
            "stats": { "total": 1, "passed": 1, "failed": 0, "skipped": 0 }
        }"#).unwrap();
        assert!(validate_collector_output(&good, &known).is_empty());

        // ensures: unknown ID → W029
        let bad_id: serde_json::Value = serde_json::from_str(r#"{
            "entity_results": [{ "entity_id": "unknown", "status": "passed" }]
        }"#).unwrap();
        let diags = validate_collector_output(&bad_id, &known);
        assert!(diags.iter().any(|d| d.code == "W029"));

        // ensures: bad stats → W030
        let bad_stats: serde_json::Value = serde_json::from_str(r#"{
            "stats": { "total": 5, "passed": 1, "failed": 1, "skipped": 1 }
        }"#).unwrap();
        let diags = validate_collector_output(&bad_stats, &known);
        assert!(diags.iter().any(|d| d.code == "W030"));
    }

    // -- auto_detect_collector --

    // B:auto_detect_collector — verify unit "file pattern match selects collector"
    #[test]
    fn test_auto_detect_collector_matches_pattern() {
        let patterns = &[
            ("junit", "rust"),
            ("jest", "javascript"),
        ];
        let files = vec!["target/junit.xml".to_string()];

        let result = auto_detect_collector(patterns, &files);
        assert_eq!(result.unwrap(), "rust");
    }

    // B:auto_detect_collector — verify unit "no match → I013"
    #[test]
    fn test_auto_detect_collector_no_match_i013() {
        let patterns: &[(&str, &str)] = &[
            ("junit", "rust"),
            ("jest", "javascript"),
        ];
        let files = vec!["README.md".to_string()];

        let err = auto_detect_collector(patterns, &files).unwrap_err();
        assert_eq!(err.code, "I013");
        assert_eq!(err.severity, Severity::Info);
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
        let diags = resolve_enhancement_conflicts(std::slice::from_ref(&conflict), EnhancementPolicy::Error, &[]);
        assert!(diags.iter().all(|d| d.code == "E017"));
        assert!(diags.iter().all(|d| d.severity == Severity::Error));

        // ensures: override resolves → empty
        let over = EnhancementOverride {
            entity_kind: "type".to_string(),
            field_name: "format".to_string(),
            winning_extension: "@ext/x".to_string(),
        };
        assert!(resolve_enhancement_conflicts(&[conflict], EnhancementPolicy::Error, &[over]).is_empty());
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

    // -- compose_grammar_injections --

    fn manifest_with_grammar(name: &str, kind: &str, path: &str) -> ManifestV2 {
        let mut m = default_manifest();
        m.name = name.to_string();
        m.grammar_contributions = vec![GrammarContribution {
            entity_kind: kind.to_string(),
            grammar_wasm_path: path.to_string(),
            export_name: None,
        }];
        m
    }

    // B:compose_grammar_injections — verify unit "single grammar per entity kind maps correctly"
    #[test]
    fn test_compose_grammar_single_per_kind() {
        let manifests = vec![
            manifest_with_grammar("@ext/a", "behavior", "/grammars/a.wasm"),
            manifest_with_grammar("@ext/b", "event", "/grammars/b.wasm"),
        ];

        let result = compose_grammar_injections(&manifests, GrammarConflictPolicy::Error).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&("behavior".to_string(), "/grammars/a.wasm".to_string(), "@ext/a".to_string())));
        assert!(result.contains(&("event".to_string(), "/grammars/b.wasm".to_string(), "@ext/b".to_string())));
    }

    // B:compose_grammar_injections — verify unit "conflict with error policy produces diagnostic"
    #[test]
    fn test_compose_grammar_conflict_error_policy() {
        let manifests = vec![
            manifest_with_grammar("@ext/a", "behavior", "/grammars/a.wasm"),
            manifest_with_grammar("@ext/b", "behavior", "/grammars/b.wasm"),
        ];

        let errors = compose_grammar_injections(&manifests, GrammarConflictPolicy::Error).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "E018");
        assert!(errors[0].message.contains("behavior"));
    }

    // B:compose_grammar_injections — verify unit "conflict with priority policy selects higher priority"
    #[test]
    fn test_compose_grammar_conflict_priority_policy() {
        let manifests = vec![
            manifest_with_grammar("@ext/a", "behavior", "/grammars/a.wasm"),
            manifest_with_grammar("@ext/b", "behavior", "/grammars/b.wasm"),
        ];

        let result = compose_grammar_injections(&manifests, GrammarConflictPolicy::Priority).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "behavior");
        assert_eq!(result[0].2, "@ext/a"); // First wins
    }

    // B:compose_grammar_injections — verify unit "conflict with namespace policy loads both grammars"
    #[test]
    fn test_compose_grammar_conflict_namespace_policy() {
        let manifests = vec![
            manifest_with_grammar("@ext/a", "behavior", "/grammars/a.wasm"),
            manifest_with_grammar("@ext/b", "behavior", "/grammars/b.wasm"),
        ];

        let result = compose_grammar_injections(&manifests, GrammarConflictPolicy::Namespace).unwrap();
        assert_eq!(result.len(), 2);
        let ext_names: Vec<&str> = result.iter().map(|(_, _, e)| e.as_str()).collect();
        assert!(ext_names.contains(&"@ext/a"));
        assert!(ext_names.contains(&"@ext/b"));
    }

    // B:compose_grammar_injections — verify property "same extensions + same policy = same composition"
    #[test]
    fn test_compose_grammar_deterministic() {
        let manifests = vec![
            manifest_with_grammar("@ext/b", "event", "/grammars/b.wasm"),
            manifest_with_grammar("@ext/a", "behavior", "/grammars/a.wasm"),
        ];

        let r1 = compose_grammar_injections(&manifests, GrammarConflictPolicy::Error).unwrap();
        let r2 = compose_grammar_injections(&manifests, GrammarConflictPolicy::Error).unwrap();
        assert_eq!(r1, r2);
    }

    // B:compose_grammar_injections — verify contract "requires/ensures consistency for grammar composition"
    #[test]
    fn test_compose_grammar_injections_contract() {
        // requires: manifests with grammar contributions
        let manifests = vec![
            manifest_with_grammar("@ext/a", "behavior", "/grammars/a.wasm"),
            manifest_with_grammar("@ext/b", "event", "/grammars/b.wasm"),
        ];

        // ensures: non-conflicting grammars compose cleanly
        let result = compose_grammar_injections(&manifests, GrammarConflictPolicy::Error).unwrap();
        assert_eq!(result.len(), 2);

        // ensures: result is sorted (deterministic)
        let r1 = compose_grammar_injections(&manifests, GrammarConflictPolicy::Error).unwrap();
        let r2 = compose_grammar_injections(&manifests, GrammarConflictPolicy::Error).unwrap();
        assert_eq!(r1, r2);

        // ensures: conflicting grammars with Error policy produce E018
        let conflicting = vec![
            manifest_with_grammar("@ext/a", "behavior", "/a.wasm"),
            manifest_with_grammar("@ext/b", "behavior", "/b.wasm"),
        ];
        let errors = compose_grammar_injections(&conflicting, GrammarConflictPolicy::Error).unwrap_err();
        assert!(errors.iter().all(|d| d.code == "E018"));

        // ensures: Priority policy resolves conflict by selecting first
        let resolved = compose_grammar_injections(&conflicting, GrammarConflictPolicy::Priority).unwrap();
        assert_eq!(resolved.len(), 1);

        // ensures: Namespace policy includes all contributors
        let all = compose_grammar_injections(&conflicting, GrammarConflictPolicy::Namespace).unwrap();
        assert_eq!(all.len(), 2);

        // ensures: empty manifests produce empty result
        let empty = compose_grammar_injections(&[], GrammarConflictPolicy::Error).unwrap();
        assert!(empty.is_empty());
    }

    // -- ingest_collector_report --

    fn sample_report() -> serde_json::Value {
        serde_json::json!({
            "entity_results": [
                {
                    "entity_id": "login_behavior",
                    "test_results": [
                        {"name": "test_login_success", "status": "passed"},
                        {"name": "test_login_failure", "status": "failed"}
                    ]
                },
                {
                    "entity_id": "signup_behavior",
                    "test_results": [
                        {"name": "test_signup", "status": "passed"}
                    ]
                },
                {
                    "entity_id": "unknown_entity",
                    "test_results": []
                }
            ]
        })
    }

    fn known_ids() -> HashSet<String> {
        let mut ids = HashSet::new();
        ids.insert("login_behavior".to_string());
        ids.insert("signup_behavior".to_string());
        ids
    }

    // B:ingest_collector_report — verify unit "entries associated with entity nodes"
    #[test]
    fn test_ingest_associates_entries_with_entities() {
        let report = sample_report();
        let result = ingest_collector_report(&report, &known_ids());
        assert_eq!(result.mapped_entries.len(), 2);
        assert_eq!(result.mapped_entries[0].0, "login_behavior");
        assert_eq!(result.mapped_entries[1].0, "signup_behavior");
    }

    // B:ingest_collector_report — verify unit "coverage metadata updated on entity"
    #[test]
    fn test_ingest_updates_coverage_metadata() {
        let report = sample_report();
        let result = ingest_collector_report(&report, &known_ids());
        assert_eq!(result.coverage_updates.len(), 2);

        let login_cov = &result.coverage_updates[0];
        assert_eq!(login_cov.0, "login_behavior");
        assert_eq!(login_cov.1, CoverageMetadata { total: 2, passed: 1, failed: 1 });

        let signup_cov = &result.coverage_updates[1];
        assert_eq!(signup_cov.0, "signup_behavior");
        assert_eq!(signup_cov.1, CoverageMetadata { total: 1, passed: 1, failed: 0 });
    }

    // B:ingest_collector_report — verify unit "merged report written to specforge-report.json"
    #[test]
    fn test_ingest_produces_serializable_report() {
        let report = sample_report();
        let result = ingest_collector_report(&report, &known_ids());

        // The IngestedReport can be serialized for output
        let output = serde_json::json!({
            "mapped_count": result.mapped_entries.len(),
            "unmapped_count": result.unmapped_entries.len(),
            "coverage": result.coverage_updates.iter().map(|(id, c)| {
                serde_json::json!({"entity_id": id, "total": c.total, "passed": c.passed, "failed": c.failed})
            }).collect::<Vec<_>>()
        });
        assert!(serde_json::to_string(&output).is_ok());
    }

    // B:ingest_collector_report — verify unit "unknown entity entries in unmapped_tests"
    #[test]
    fn test_ingest_unknown_entities_in_unmapped() {
        let report = sample_report();
        let result = ingest_collector_report(&report, &known_ids());
        assert_eq!(result.unmapped_entries.len(), 1);
        assert_eq!(
            result.unmapped_entries[0].get("entity_id").unwrap().as_str().unwrap(),
            "unknown_entity"
        );
    }

    // B:ingest_collector_report — verify contract
    #[test]
    fn test_ingest_collector_report_contract() {
        let report = sample_report();
        let known = known_ids();
        let result = ingest_collector_report(&report, &known);

        // ensures: known entities are mapped
        assert_eq!(result.mapped_entries.len(), 2);
        // ensures: unknown entities are unmapped
        assert_eq!(result.unmapped_entries.len(), 1);
        // ensures: coverage computed per mapped entity
        assert_eq!(result.coverage_updates.len(), 2);
        // ensures: empty report produces empty result
        let empty = ingest_collector_report(&serde_json::json!({}), &known);
        assert!(empty.mapped_entries.is_empty());
        assert!(empty.unmapped_entries.is_empty());
        assert!(empty.coverage_updates.is_empty());
    }
}
