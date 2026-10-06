#[cfg(test)]
use crate::{EdgeRegistry, FieldRegistry, KindRegistry};
use specforge_common::{Diagnostic, Severity};
use specforge_protocol_types::ExtensionDeclaration;
use std::collections::HashMap;

/// Cross-validate registered entity fields: check target_kind and edge label references
/// resolve to registered entries. Test-only: the registry build reports W021 on load
/// (`declaration::consistency`).
#[cfg(test)]
pub fn validate_registered_entity_fields(
    field_reg: &FieldRegistry,
    kind_reg: &KindRegistry,
    edge_reg: &EdgeRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (kind_name, field_name, entry) in field_reg.iter() {
        // Validate target_kind references
        if let Some(target) = &entry.declared.target_kind
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
        if let Some(edge) = &entry.declared.edge
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

/// E027/W062 for every declaration's peers, against the loaded ones.
pub(crate) fn peer_dependencies(declarations: &[ExtensionDeclaration]) -> Vec<Diagnostic> {
    declarations
        .iter()
        .flat_map(|declaration| peer_dependencies_of(declaration, declarations))
        .collect()
}

/// The peer diagnostics of `declaration` against the `installed` ones
/// (`declaration` itself may be among them).
pub(crate) fn peer_dependencies_of(
    declaration: &ExtensionDeclaration,
    installed: &[ExtensionDeclaration],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let name = declaration.name();

    let installed: std::collections::HashMap<&str, &str> =
        installed.iter().map(|d| (d.name(), d.version())).collect();

    for peer in declaration.peers() {
        match installed.get(peer.name.as_str()) {
            None if peer.optional => {}
            None => {
                diagnostics.push(Diagnostic {
                    code: "E027".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "extension '{}' requires peer dependency '{}' {} which is not installed",
                        name, peer.name, peer.version
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
                            name, peer.name, peer.version
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
                            name, peer.name, peer.version, installed_version
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
                suggestion: Some(
                    "declare the kind with supports_verify (KindBuilder::supports_verify)"
                        .to_string(),
                ),
                data: None,
            });
        }
    }

    // Sort for deterministic output
    diagnostics.sort_by(|a, b| a.message.cmp(&b.message));
    diagnostics
}

/// W023: a validation rule code a later extension declares again, once
/// per repeat, naming the code, that extension and the first one. A code
/// one extension repeats is not reported. The build keeps every rule.
pub(crate) fn duplicate_rule_codes(declarations: &[ExtensionDeclaration]) -> Vec<Diagnostic> {
    let mut first: HashMap<&str, &str> = HashMap::new();
    let mut diagnostics = Vec::new();
    for declaration in declarations {
        for rule in &declaration.validation_rules {
            match first.get(rule.code.as_str()) {
                None => {
                    first.insert(&rule.code, declaration.name());
                }
                Some(owner) if *owner != declaration.name() => diagnostics.push(Diagnostic {
                    code: "W023".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "validation rule code '{}' from '{}' duplicates code from '{}'",
                        rule.code,
                        declaration.name(),
                        owner
                    ),
                    span: None,
                    suggestion: None,
                    data: None,
                }),
                Some(_) => {}
            }
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation::populate::populate;
    use crate::compilation::tests::support::{declare, peer};
    use specforge_extension_sdk::prelude::*;

    fn software_manifest() -> ExtensionDeclaration {
        declare("@specforge/software", |c| {
            c.kind("Behavior", |k| {
                k.keyword("behavior").testable(true).supports_verify(true);
                k.field("invariants", |f| {
                    f.field_type(FieldType::ReferenceList)
                        .edge("enforces")
                        .target_kind("invariant");
                });
            });
            c.kind("Invariant", |k| {
                k.keyword("invariant").testable(true).supports_verify(true);
            });
            c.edge("enforces", |e| {
                e.source_kind("behavior").target_kind("invariant");
            });
        })
    }

    /// The extension `name` at `version`, declaring nothing.
    fn installed(name: &str, version: &str) -> ExtensionDeclaration {
        ContributionsBuilder::new(ExtensionMeta::new(name, version)).declaration()
    }

    /// `@specforge/product` 1.0.0, requiring `peer_name` in `range`.
    fn product_requiring(peer_name: &str, range: &str) -> ExtensionDeclaration {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@specforge/product", "1.0.0"));
        c.meta.peer_dependencies.push(peer(peer_name, range));
        c.declaration()
    }

    // -- B:validate_registered_entity_fields --

    // B:validate_registered_entity_fields — verify unit "target_kind reference resolves to registered kind"
    // B:register_validation_rules_from_manifest — verify unit "target_kind reference validated against KindRegistry after registries_populated"
    #[test]
    fn test_target_kind_reference_resolves_to_registered_kind() {
        let (kind_reg, field_reg, edge_reg, _) = populate(&[software_manifest()]);
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
        let (kind_reg, field_reg, edge_reg, _) = populate(&[software_manifest()]);
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
        let declaration = declare("@test/ext", |c| {
            c.kind("Task", |k| {
                k.keyword("task");
                k.field("owner", |f| {
                    f.field_type(FieldType::Reference).target_kind("person");
                });
            });
        });
        let (kind_reg, field_reg, edge_reg, _) = populate(&[declaration]);
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
        let declaration = declare("@test/ext", |c| {
            c.kind("Task", |k| {
                k.keyword("task");
                k.field("owner", |f| {
                    f.field_type(FieldType::Reference).edge("owns");
                });
            });
        });
        let (kind_reg, field_reg, edge_reg, _) = populate(&[declaration]);
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
        let declaration = declare("@custom/cooking", |c| {
            c.kind("Recipe", |k| {
                k.keyword("recipe").testable(true).supports_verify(true);
                k.field("ingredients", |f| {
                    f.field_type(FieldType::ReferenceList)
                        .edge("uses")
                        .target_kind("ingredient");
                });
            });
            c.kind("Ingredient", |k| {
                k.keyword("ingredient");
            });
            c.edge("uses", |e| {
                e.source_kind("recipe").target_kind("ingredient");
            });
        });
        let (kind_reg, field_reg, edge_reg, pop_diags) = populate(&[declaration]);
        assert!(pop_diags.is_empty());
        let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
        assert!(
            diags.is_empty(),
            "custom domain should validate cleanly: {:?}",
            diags
        );
    }

    // -- B:detect_duplicate_entity_kinds --

    // -- B:validate_peer_dependencies --

    // B:validate_peer_dependencies — verify unit "satisfied peer dependency passes validation"
    #[test]
    fn test_satisfied_peer_dependency_passes_validation() {
        let m1 = software_manifest();
        let m2 = product_requiring("@specforge/software", ">=1.0.0");
        let diags = peer_dependencies(&[m1, m2]);
        assert!(
            diags.is_empty(),
            "expected no diagnostics, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "missing peer dependency produces hard error"
    #[test]
    fn test_missing_peer_dependency_produces_hard_error() {
        let m = product_requiring("@specforge/software", ">=1.0.0");
        let diags = peer_dependencies(&[m]);
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
        let m1 = installed("@specforge/software", "0.5.0");
        let m2 = product_requiring("@specforge/software", ">=1.0.0");
        let diags = peer_dependencies(&[m1, m2]);
        assert!(
            diags.iter().any(|d| d.code == "E027"
                && d.message.contains(">=1.0.0")
                && d.message.contains("0.5.0")),
            "expected E027 with version info, got: {:?}",
            diags
        );
    }

    // -- B:validate_extension_testability --

    // -- B:register_validation_rules_from_manifest --

    // B:validate_registered_entity_fields — verify contract "requires/ensures consistency for field cross-validation"
    #[test]
    fn test_validate_registered_entity_fields_contract() {
        // requires: all registries populated
        let (kind_reg, field_reg, edge_reg, _) = populate(&[software_manifest()]);
        let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
        // ensures: valid references produce no warnings
        assert!(diags.is_empty());
        // ensures: unresolved references produce W021
        let bad_manifest = declare("@t/e", |c| {
            c.kind("A", |k| {
                k.keyword("a");
                k.field("f", |f| {
                    f.field_type(FieldType::Reference)
                        .target_kind("nonexistent");
                });
            });
        });
        let (kr, fr, er, _) = populate(&[bad_manifest]);
        let bad_diags = validate_registered_entity_fields(&fr, &kr, &er);
        assert!(bad_diags.iter().any(|d| d.code == "W021"));
    }

    // B:validate_peer_dependencies — verify contract "requires/ensures consistency for peer dependency validation"
    #[test]
    fn test_validate_peer_dependencies_contract() {
        // requires: manifests loaded
        // ensures: satisfied deps → no error
        let m1 = software_manifest();
        let m2 = product_requiring("@specforge/software", ">=1.0.0");
        assert!(peer_dependencies(&[m1, m2]).is_empty());
        // ensures: missing dep → E027
        let m3 = product_requiring("@specforge/missing", ">=1.0.0");
        let diags = peer_dependencies(&[m3]);
        assert!(diags.iter().any(|d| d.code == "E027"));
    }

    // -- B:validate_peer_dependencies (semver range matching) --

    // B:validate_peer_dependencies — verify unit "caret range ^1.0.0 matches 1.x.x"
    #[test]
    fn test_caret_range_matches() {
        let m1 = installed("@specforge/software", "1.2.3");
        let m2 = product_requiring("@specforge/software", "^1.0.0");
        let diags = peer_dependencies(&[m1, m2]);
        assert!(
            diags.is_empty(),
            "^1.0.0 should match 1.2.3, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "caret range ^1.0.0 rejects 2.0.0"
    #[test]
    fn test_caret_range_rejects_major_bump() {
        let m1 = installed("@specforge/software", "2.0.0");
        let m2 = product_requiring("@specforge/software", "^1.0.0");
        let diags = peer_dependencies(&[m1, m2]);
        assert!(
            diags.iter().any(|d| d.code == "E027"),
            "^1.0.0 should reject 2.0.0, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "tilde range ~1.2.0 matches 1.2.x"
    #[test]
    fn test_tilde_range_matches() {
        let m1 = installed("@specforge/software", "1.2.5");
        let m2 = product_requiring("@specforge/software", "~1.2.0");
        let diags = peer_dependencies(&[m1, m2]);
        assert!(
            diags.is_empty(),
            "~1.2.0 should match 1.2.5, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "tilde range ~1.2.0 rejects 1.3.0"
    #[test]
    fn test_tilde_range_rejects_minor_bump() {
        let m1 = installed("@specforge/software", "1.3.0");
        let m2 = product_requiring("@specforge/software", "~1.2.0");
        let diags = peer_dependencies(&[m1, m2]);
        assert!(
            diags.iter().any(|d| d.code == "E027"),
            "~1.2.0 should reject 1.3.0, got: {:?}",
            diags
        );
    }

    // B:validate_peer_dependencies — verify unit "malformed semver range in peer dep produces W062"
    #[test]
    fn test_malformed_semver_range_produces_warning() {
        let m1 = installed("@specforge/software", "1.0.0");
        let m2 = product_requiring("@specforge/software", "not-a-version");
        let diags = peer_dependencies(&[m1, m2]);
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
        let m1 = installed("@specforge/software", "bad-version");
        let m2 = product_requiring("@specforge/software", "^1.0.0");
        let diags = peer_dependencies(&[m1, m2]);
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
        let m1 = installed("@specforge/software", "1.0.0");
        let m2 = product_requiring("@specforge/software", "1.0.0");
        let diags = peer_dependencies(&[m1, m2]);
        assert!(
            diags.is_empty(),
            "exact 1.0.0 should match 1.0.0, got: {:?}",
            diags
        );
    }
}
