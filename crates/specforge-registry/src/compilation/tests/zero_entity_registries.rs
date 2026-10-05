// Integration tests for spec/behaviors/zero-entity-registries.spec
//
// Covers verify statements for:
//   - boot_empty_kind_registry (6 verifies)
//   - boot_empty_field_registry (4 verifies)
//   - boot_empty_edge_registry (3 verifies)
//   - populate_kind_registry_from_extensions (6 verifies)
//   - populate_field_registry_from_extensions (6 verifies)
//   - populate_edge_registry_from_extensions (4 verifies)
//   - register_entity_kinds_from_manifest (9 verifies)
//   - register_edge_types_from_manifest (6 verifies)
//   - validate_manifest_v2_schema (5 verifies)
//   - detect_unknown_entity_kinds (5 verifies)
//   - suggest_missing_extensions (4 verifies)
//   - validate_registered_entity_fields (0; proven in specforge-project)
//   - detect_duplicate_entity_kinds (4+1 verifies)
//   - validate_peer_dependencies (4 verifies)
//   - validate_extension_testability (5 verifies)
//   - register_validation_rules_from_manifest (6 verifies)
//   - register_extension_validation_rules (3 verifies)
//   - apply_entity_enhancements (5 verifies)

use specforge_test_macros::test as spec;

use specforge_common::{Severity, SourceSpan, Sym};
use specforge_extension_sdk::{
    CheckKind, ContributionsBuilder, EnhancementBuilder, ExtensionMeta, FieldType, PeerDependency,
    ValidationSeverity,
};
use specforge_protocol_types::{
    EdgeTypeDescriptor, EntityEnhancementDescriptor, ExtensionDeclaration,
};
use specforge_registry::compilation::EntityView;
use specforge_registry::compilation::{
    apply_entity_enhancements, register_validation_rules, validate_extension_testability,
    validate_registered_entity_fields,
};
use specforge_registry::{
    EdgeRegistry, FieldRegistry, FieldRegistryEntry, KindRegistry, ManifestFieldType, ManifestV2,
    detect_unknown_entity_fields, validate_manifest,
};

use super::support::{declare, extension, peer, product, software};
use crate::compilation::declaration::{consistency, shape};
use crate::compilation::populate::populate;
use crate::compilation::validate::{duplicate_entity_kinds, peer_dependencies};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A manifest, for the checks only a manifest has (its `manifestVersion`,
/// its `wasmPath`, its unknown camelCase keys).
fn manifest(json: &str) -> ManifestV2 {
    serde_json::from_str(json).unwrap()
}

/// The declaration of `name` at `version`, with `peers` and nothing else.
fn versioned(name: &str, version: &str, peers: Vec<PeerDependency>) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, version));
    c.meta.peer_dependencies = peers;
    c.declaration()
}

/// An optional peer dependency on `name` in `version`.
fn optional_peer(name: &str, version: &str) -> PeerDependency {
    PeerDependency {
        optional: true,
        ..peer(name, version)
    }
}

/// `@other/ext`, declaring `behavior` again.
fn other_behavior() -> ExtensionDeclaration {
    declare("@other/ext", |c| {
        c.kind("Behavior", |k| {
            k.keyword("behavior");
        });
    })
}

/// The enhancement `extension` declares on `target` (owned by itself),
/// with what `f` adds.
fn enhancement(
    extension: &str,
    target: &str,
    f: impl FnOnce(&mut EnhancementBuilder),
) -> (String, EntityEnhancementDescriptor) {
    let declaration = declare(extension, |c| {
        c.enhance(target, extension, f);
    });
    (extension.to_string(), declaration.enhancements[0].clone())
}

/// An extension declaring warning rules `(code, message, check)`.
fn rules(name: &str, declared: &[(&str, &str, CheckKind)]) -> ExtensionDeclaration {
    declare(name, |c| {
        for (code, message, check) in declared {
            c.rule(code, |r| {
                r.severity(ValidationSeverity::Warning)
                    .message_template(message)
                    .check(*check);
            });
        }
    })
}

#[allow(dead_code)]
fn span(file: &str) -> SourceSpan {
    SourceSpan {
        file: Sym::new(file),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

/// A span that outlives the test's entity views.
fn pinned(span: SourceSpan) -> &'static SourceSpan {
    Box::leak(Box::new(span))
}

// ===========================================================================
// B:boot_empty_kind_registry (6 verifies)
// ===========================================================================

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "KindRegistry::new() has zero entries"
)]
fn boot_kind_registry_zero_entries() {
    let registry = KindRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "parser recognizes spec keyword without extensions"
)]
fn boot_kind_registry_spec_keyword() {
    let registry = KindRegistry::new();
    // spec is a structural keyword — NOT in KindRegistry
    assert!(!registry.contains("spec"));
    // But the parser recognizes it (tested via specforge_parser)
    let parsed = specforge_parser::parse(
        "spec my_spec \"My Spec\" {\n  version \"1.0\"\n}\n",
        "test.spec",
    );
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].kind.raw, "spec");
}

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "parser recognizes ref keyword without extensions"
)]
fn boot_kind_registry_ref_keyword() {
    let registry = KindRegistry::new();
    assert!(!registry.contains("ref"));
    let parsed = specforge_parser::parse("ref gh.issue:42 \"Fix bug\"\n", "test.spec");
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].kind.raw, "ref");
}

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "parser recognizes use keyword without extensions"
)]
fn boot_kind_registry_use_keyword() {
    let registry = KindRegistry::new();
    assert!(!registry.contains("use"));
    let parsed = specforge_parser::parse("use \"types/core\"\n", "test.spec");
    assert_eq!(parsed.imports.len(), 1);
}

#[test]
fn boot_kind_registry_define_keyword() {
    let registry = KindRegistry::new();
    assert!(!registry.contains("define"));
    let parsed = specforge_parser::parse(
        "define user_story {\n  required [description]\n}\n",
        "test.spec",
    );
    let has_define = parsed.entities.iter().any(|e| e.kind.raw == "define");
    assert!(
        has_define || parsed.entities.is_empty(),
        "define should parse via grammar rule"
    );
}

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "Boot Empty Kind Registry: empty kind registry boot holds — compiler_initializing, kind_registry_empty, structural_keywords_ready"
)]
fn boot_kind_registry_contract() {
    let registry = KindRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.get("behavior").is_none());
    assert!(registry.get("").is_none());
    assert!(!registry.contains("behavior"));
    assert_eq!(registry.keywords().count(), 0);
    assert_eq!(registry.iter().count(), 0);
}

// ===========================================================================
// B:boot_empty_field_registry (4 verifies)
// ===========================================================================

#[spec(
    behavior = "boot_empty_field_registry",
    verify = "FieldRegistry::new() has zero entries"
)]
fn boot_field_registry_zero_entries() {
    let registry = FieldRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[spec(
    behavior = "boot_empty_field_registry",
    verify = "no field names recognized before extension loading"
)]
fn boot_field_registry_no_fields() {
    let registry = FieldRegistry::new();
    assert!(registry.get("behavior", "contract").is_none());
    assert!(!registry.contains("behavior", "contract"));
    assert!(registry.fields_for_kind("behavior").is_empty());
}

#[test]
fn boot_field_registry_title_not_a_field() {
    let mut registry = FieldRegistry::new();
    registry.register(FieldRegistryEntry {
        kind_name: "behavior".to_string(),
        field_name: "contract".to_string(),
        description: None,
        field_type: ManifestFieldType::Block,
        source_extension: "@specforge/software".to_string(),
        edge: None,
        target_kind: None,
        file_reference: false,
        required: false,
        inverse_of: None,
        normative: false,
        exempts_obligations: false,
        headline: false,
        derived_from: None,
        proof_role: None,
    });
    // title is NOT a field — it's a grammar-level construct
    assert!(registry.get("behavior", "title").is_none());
}

#[spec(
    behavior = "boot_empty_field_registry",
    verify = "Boot Empty Field Registry: empty field registry boot holds — compiler_initializing, field_registry_empty, no_fields_recognized"
)]
fn boot_field_registry_contract() {
    let registry = FieldRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.get("behavior", "contract").is_none());
    assert!(registry.fields_for_kind("behavior").is_empty());
    assert_eq!(registry.iter().count(), 0);
}

// ===========================================================================
// B:boot_empty_edge_registry (3 verifies)
// ===========================================================================

#[spec(
    behavior = "boot_empty_edge_registry",
    verify = "edge type set starts with zero entries"
)]
fn boot_edge_registry_zero_entries() {
    let registry = EdgeRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[spec(
    behavior = "boot_empty_edge_registry",
    verify = "no edge labels recognized before extension loading"
)]
fn boot_edge_registry_no_labels() {
    let registry = EdgeRegistry::new();
    assert!(registry.get("enforces").is_none());
    assert!(!registry.contains("enforces"));
}

#[spec(
    behavior = "boot_empty_edge_registry",
    verify = "Boot Empty Edge Registry: empty edge registry boot holds — compiler_initializing, edge_registry_empty, no_edges_recognized"
)]
fn boot_edge_registry_contract() {
    let registry = EdgeRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.get("enforces").is_none());
    assert_eq!(registry.labels().count(), 0);
    assert_eq!(registry.iter().count(), 0);
}

// ===========================================================================
// B:populate_kind_registry_from_extensions (6 verifies)
// ===========================================================================

#[test]
fn populate_kind_extensions_topological_order() {
    let (kind_reg, _, _, _) = populate(&[software(), product()]);
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("invariant"));
    assert!(kind_reg.contains("feature"));
}

#[spec(
    behavior = "populate_kind_registry_from_extensions",
    verify = "all entityKinds entries registered"
)]
fn populate_kind_all_entries_registered() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    assert_eq!(kind_reg.len(), 2);
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("invariant"));
}

#[spec(
    behavior = "populate_kind_registry_from_extensions",
    verify = "registered keywords available to parser"
)]
fn populate_kind_keywords_available() {
    let (kind_reg, _, _, _) = populate(&[software(), product()]);
    let keywords: Vec<String> = kind_reg.keywords().cloned().collect();
    assert!(keywords.contains(&"behavior".to_string()));
    assert!(keywords.contains(&"invariant".to_string()));
    assert!(keywords.contains(&"feature".to_string()));
}

// Not linked to "population completes before validation": it validates
// with the test-only validate_registered_entity_fields, which no load
// runs. specforge-project/tests/registered_fields.rs proves the obligation
// through CompiledProject::compile.
#[test]
fn populate_kind_completes_before_validation() {
    // The first extension's field points at a kind only the second declares.
    let early = declare("@test/early", |c| {
        c.kind("Task", |k| {
            k.keyword("task");
            k.field("owner", |f| {
                f.field_type(FieldType::Reference)
                    .edge("owned_by")
                    .target_kind("person");
            });
        });
        c.edge("owned_by", |e| {
            e.source_kind("task").target_kind("person");
        });
    });
    let late = declare("@test/late", |c| {
        c.kind("Person", |k| {
            k.keyword("person");
        });
    });

    // Validating mid-population, with only the first extension registered,
    // reports the forward reference.
    let (k1, f1, e1, _) = populate(std::slice::from_ref(&early));
    let partial = validate_registered_entity_fields(&f1, &k1, &e1);
    assert!(
        partial
            .iter()
            .any(|d| d.code == "W021" && d.message.contains("'person'")),
        "{partial:?}"
    );

    // populate returns only once every extension is in, so the
    // validation that follows sees both kinds and reports nothing.
    let (kind_reg, field_reg, edge_reg, diags) = populate(&[early, late]);
    assert!(diags.is_empty(), "{diags:?}");
    let mut kinds: Vec<&String> = kind_reg.keywords().collect();
    kinds.sort();
    assert_eq!(kinds, ["person", "task"]);
    let post = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    assert!(post.is_empty(), "{post:?}");
    let unknown = specforge_registry::detect_unknown_entity_kinds(
        &[
            EntityView::new("task", "t1", pinned(span("main.spec"))),
            EntityView::new("person", "p1", pinned(span("main.spec"))),
        ],
        &kind_reg,
        None,
    );
    assert!(
        unknown.is_empty(),
        "no E024 for registered kinds: {unknown:?}"
    );
}

#[spec(
    behavior = "populate_kind_registry_from_extensions",
    verify = "two extensions register kinds without collision"
)]
fn populate_kind_two_extensions_no_collision() {
    let (kind_reg, _, _, diags) = populate(&[software(), product()]);
    assert!(!diags.iter().any(|d| d.code == "E026"));
    assert_eq!(kind_reg.len(), 3);
}

#[spec(
    behavior = "populate_kind_registry_from_extensions",
    verify = "Populate Kind Registry From Extensions: registry population holds for the declared obligations"
)]
fn populate_kind_registry_contract() {
    let (kind_reg, field_reg, edge_reg, diags) = populate(&[software(), product()]);
    assert!(!kind_reg.is_empty());
    assert!(!field_reg.is_empty());
    assert!(!edge_reg.is_empty());
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("feature"));
    assert!(!diags.iter().any(|d| d.code == "E026"));
    assert_eq!(kind_reg.len(), 3);
}

// ===========================================================================
// B:populate_field_registry_from_extensions (4 verifies)
// ===========================================================================

#[spec(
    behavior = "populate_field_registry_from_extensions",
    verify = "fields registered per entity kind"
)]
fn populate_field_per_entity_kind() {
    let (_, field_reg, _, _) = populate(&[software()]);
    assert!(field_reg.contains("behavior", "contract"));
    assert!(field_reg.contains("behavior", "invariants"));
    assert!(field_reg.fields_for_kind("invariant").is_empty());
}

#[spec(
    behavior = "populate_field_registry_from_extensions",
    verify = "field types validated against known types"
)]
fn populate_field_types_validated() {
    let mut manifest = declare("@test/ext", |c| {
        c.kind("Thing", |k| {
            k.keyword("thing");
            k.field("summary", |f| {
                f.field_type(FieldType::String);
            });
            k.field("tags", |f| {
                f.field_type(FieldType::StringList);
            });
            k.field("status", |f| {
                f.field_type(FieldType::Enum)
                    .enum_values(&["draft", "done"]);
            });
            k.field("body", |f| {
                f.field_type(FieldType::Block);
            });
            k.field("parts", |f| {
                f.field_type(FieldType::ReferenceList);
            });
            k.field("notes", |f| {
                f.field_type(FieldType::String);
            });
        });
    });
    // "text" is no field type the vocabulary has.
    manifest.entities[0].fields[5].field_type = "text".into();
    let (_, field_reg, _, diags) = populate(&[manifest]);

    // Each known type name maps to its variant.
    let ty = |name: &str| field_reg.get("thing", name).unwrap().field_type.clone();
    assert_eq!(ty("summary"), ManifestFieldType::String);
    assert_eq!(ty("tags"), ManifestFieldType::StringList);
    assert_eq!(
        ty("status"),
        ManifestFieldType::Enum(vec!["draft".to_string(), "done".to_string()])
    );
    assert_eq!(ty("body"), ManifestFieldType::Block);
    assert_eq!(ty("parts"), ManifestFieldType::ReferenceList);

    // "text" is no known type: the field is rejected with one warning.
    assert!(!field_reg.contains("thing", "notes"));
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W019");
    assert_eq!(diags[0].severity, Severity::Warning);
    assert!(diags[0].message.contains("'text'"), "{}", diags[0].message);
}

#[spec(
    behavior = "populate_field_registry_from_extensions",
    verify = "invalid field type produces warning"
)]
fn populate_field_invalid_type_warning() {
    let mut manifest = declare("@test/ext", |c| {
        c.kind("Thing", |k| {
            k.keyword("thing");
            k.field("data", |f| {
                f.field_type(FieldType::String);
            });
        });
    });
    manifest.entities[0].fields[0].field_type = "unknown_type_xyz".into();
    let (_, field_reg, _, diags) = populate(&[manifest]);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W019" && d.message.contains("unknown_type_xyz"))
    );
    assert!(!field_reg.contains("thing", "data"));
}

#[spec(
    behavior = "populate_field_registry_from_extensions",
    verify = "a field's normative flag reaches its registry entry"
)]
fn populate_field_keeps_normative_flag() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Rule", |k| {
            k.keyword("rule");
            k.field("guarantee", |f| {
                f.field_type(FieldType::String).normative();
            });
            k.field("description", |f| {
                f.field_type(FieldType::String);
            });
        });
    });
    let (_, field_reg, _, _) = populate(&[manifest]);
    assert!(field_reg.get("rule", "guarantee").unwrap().normative);
    assert!(!field_reg.get("rule", "description").unwrap().normative);
}

/// An extension whose `rule` kind declares `limit` (kind field) and whose
/// shared `goal` field reaches every kind, with the roles given.
fn roles_extension(limit_role: &str, goal_role: &str) -> ExtensionDeclaration {
    declare("@test/ext", |c| {
        c.shared_field("goal", |f| {
            f.field_type(FieldType::String).proof_role(goal_role);
        });
        c.kind("Rule", |k| {
            k.keyword("rule");
            k.field("limit", |f| {
                f.field_type(FieldType::String).proof_role(limit_role);
            });
            k.field("description", |f| {
                f.field_type(FieldType::String);
            });
        });
    })
}

#[spec(
    behavior = "populate_field_registry_from_extensions",
    verify = "a field's proof_role reaches the field registry"
)]
fn populate_field_keeps_proof_role() {
    use specforge_registry::ProofRole;
    let (mut kind_reg, mut field_reg, _, diags) = populate(&[roles_extension("bound", "claim")]);
    assert!(diags.is_empty(), "{diags:?}");
    let role =
        |reg: &FieldRegistry, kind: &str, field: &str| reg.get(kind, field).unwrap().proof_role;
    assert_eq!(role(&field_reg, "rule", "limit"), Some(ProofRole::Bound));
    assert_eq!(role(&field_reg, "rule", "goal"), Some(ProofRole::Claim));
    assert_eq!(role(&field_reg, "rule", "description"), None);

    // An enhancement's field carries its role onto the target kind.
    let enhancements = vec![enhancement("@test/other", "rule", |e| {
        e.field("expression", |f| {
            f.field_type(FieldType::String).proof_role("claim");
        });
    })];
    let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(
        role(&field_reg, "rule", "expression"),
        Some(ProofRole::Claim)
    );
}

#[spec(
    behavior = "populate_field_registry_from_extensions",
    verify = "a proof_role other than bound or claim is refused"
)]
fn populate_field_refuses_unknown_proof_role() {
    let (_, field_reg, _, diags) = populate(&[roles_extension("assumed", "claim")]);
    let refused: Vec<_> = diags
        .iter()
        .filter(|d| d.code == "W021" && d.message.contains("proof_role 'assumed'"))
        .collect();
    assert_eq!(refused.len(), 1, "{diags:?}");
    assert_eq!(refused[0].severity, Severity::Warning);
    // The field is registered; it has no role.
    assert_eq!(field_reg.get("rule", "limit").unwrap().proof_role, None);
}

#[spec(
    behavior = "populate_field_registry_from_extensions",
    verify = "Populate Field Registry From Extensions: field registry population holds — extension_manifests_loaded_fired, kind_registry_populated, fields_registered, field_types_validated, fields_populated"
)]
fn populate_field_registry_contract() {
    let (_, field_reg, _, diags) = populate(&[software()]);
    assert!(field_reg.contains("behavior", "contract"));
    assert!(field_reg.contains("behavior", "invariants"));
    let contract = field_reg.get("behavior", "contract").unwrap();
    assert_eq!(contract.field_type, ManifestFieldType::Block);
    assert!(!diags.iter().any(|d| d.code == "W019"));
}

// ===========================================================================
// B:populate_edge_registry_from_extensions (4 verifies)
// ===========================================================================

#[spec(
    behavior = "populate_edge_registry_from_extensions",
    verify = "explicit edgeTypes merged into edge set"
)]
fn populate_edge_explicit_merged() {
    let (_, _, edge_reg, _) = populate(&[software()]);
    assert!(edge_reg.contains("enforces"));
}

#[spec(
    behavior = "populate_edge_registry_from_extensions",
    verify = "implicit edges from field mappings merged"
)]
fn populate_edge_implicit_from_fields() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Task", |k| {
            k.keyword("task");
            k.field("assignee", |f| {
                f.field_type(FieldType::Reference)
                    .edge("assigned_to")
                    .target_kind("person");
            });
        });
    });
    let (_, _, edge_reg, _) = populate(&[manifest]);
    assert!(edge_reg.contains("assigned_to"));
    let edge = edge_reg.get("assigned_to").unwrap();
    assert_eq!(edge.source_kind.as_deref(), Some("task"));
    assert_eq!(edge.target_kind.as_deref(), Some("person"));
}

#[spec(
    behavior = "populate_edge_registry_from_extensions",
    verify = "duplicate edge labels produce warning"
)]
fn populate_edge_duplicate_warning() {
    let mut m1 = software();
    let mut m2 = product();
    m1.edges.push(EdgeTypeDescriptor {
        label: "links_to".to_string(),
        ..Default::default()
    });
    m2.edges.push(EdgeTypeDescriptor {
        label: "links_to".to_string(),
        ..Default::default()
    });
    let (_, _, _, diags) = populate(&[m1, m2]);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W018" && d.message.contains("links_to"))
    );
}

#[spec(
    behavior = "populate_edge_registry_from_extensions",
    verify = "Populate Edge Registry From Extensions: edge registry population holds — extension_manifests_loaded_fired, edge_set_complete, duplicates_warned, edges_populated"
)]
fn populate_edge_registry_contract() {
    let (_, _, edge_reg, diags) = populate(&[software(), product()]);
    assert!(edge_reg.contains("enforces"));
    assert!(edge_reg.contains("composes"));
    assert!(!diags.iter().any(|d| d.code == "W018"));
}

// ===========================================================================
// B:register_entity_kinds_from_manifest (8 verifies)
// ===========================================================================

#[spec(
    behavior = "register_entity_kinds_from_manifest",
    verify = "entity kind registered with testable flag"
)]
fn register_kind_testable_flag() {
    let (kind_reg, _, _, diags) = populate(&[software()]);
    assert!(diags.is_empty());
    let behavior = kind_reg.get("behavior").unwrap();
    assert!(behavior.testable);
    let invariant = kind_reg.get("invariant").unwrap();
    assert!(invariant.testable);
}

#[spec(
    behavior = "register_entity_kinds_from_manifest",
    verify = "entity kind registered with singleton flag"
)]
fn register_kind_singleton_flag() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Project", |k| {
            k.keyword("project").singleton(true);
        });
        c.kind("Task", |k| {
            k.keyword("task");
        });
    });
    let (kind_reg, _, _, _) = populate(&[manifest]);
    assert!(kind_reg.get("project").unwrap().singleton);
    assert!(
        !kind_reg.get("task").unwrap().singleton,
        "defaults to false"
    );
}

#[spec(
    behavior = "register_entity_kinds_from_manifest",
    verify = "entity kind registered with LSP metadata"
)]
fn register_kind_lsp_metadata() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let behavior = kind_reg.get("behavior").unwrap();
    assert_eq!(behavior.semantic_token.as_deref(), Some("function"));
    assert_eq!(behavior.lsp_icon.as_deref(), Some("Method"));
}

#[spec(
    behavior = "register_entity_kinds_from_manifest",
    verify = "source extension recorded in registry entry"
)]
fn register_kind_source_extension() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let behavior = kind_reg.get("behavior").unwrap();
    assert_eq!(behavior.source_extension, "@specforge/software");
}

#[test]
fn register_kind_testable_participates_in_coverage() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let testable_kinds: Vec<_> = kind_reg
        .iter()
        .filter(|(_, e)| e.testable)
        .map(|(k, _)| k.clone())
        .collect();
    assert!(testable_kinds.contains(&"behavior".to_string()));
    assert!(testable_kinds.contains(&"invariant".to_string()));
}

#[test]
fn register_kind_testable_false_excluded() {
    let (kind_reg, _, _, _) = populate(&[product()]);
    let feature = kind_reg.get("feature").unwrap();
    assert!(!feature.testable);
}

#[spec(
    behavior = "register_entity_kinds_from_manifest",
    verify = "no default testability assumed by core"
)]
fn register_kind_no_default_testability() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Thing", |k| {
            k.keyword("thing");
        });
    });
    let (kind_reg, _, _, _) = populate(&[manifest]);
    let thing = kind_reg.get("thing").unwrap();
    assert!(!thing.testable, "default testability should be false");
}

#[spec(
    behavior = "register_entity_kinds_from_manifest",
    verify = "a kind's lifecycle_field must name a field it declares"
)]
fn register_kind_lifecycle_field() {
    let manifest = declare("@test/ext", |c| {
        c.shared_field("phase", |f| {
            f.field_type(FieldType::String);
        });
        c.kind("Task", |k| {
            k.keyword("task").lifecycle_field("stage");
            k.field("stage", |f| {
                f.field_type(FieldType::String);
            });
        });
        c.kind("Epic", |k| {
            k.keyword("epic").lifecycle_field("phase");
        });
        c.kind("Note", |k| {
            k.keyword("note").lifecycle_field("status");
        });
        c.kind("Idea", |k| {
            k.keyword("idea");
        });
    });
    let (kind_reg, _, _, diags) = populate(&[manifest]);
    let lifecycle = |kind: &str| kind_reg.get(kind).unwrap().lifecycle_field.clone();
    assert_eq!(lifecycle("task").as_deref(), Some("stage"));
    // An extension-level shared field is one of the kind's fields.
    assert_eq!(lifecycle("epic").as_deref(), Some("phase"));
    assert_eq!(lifecycle("idea"), None);
    // `note` declares no `status`: refused.
    assert_eq!(lifecycle("note"), None);
    let refused: Vec<_> = diags.iter().filter(|d| d.code == "W021").collect();
    assert_eq!(refused.len(), 1, "{diags:?}");
    assert!(refused[0].message.contains("lifecycle_field 'status'"));
}

#[spec(
    behavior = "register_entity_kinds_from_manifest",
    verify = "Register Entity Kinds From Manifest: entity kind registration holds — extension_manifests_loaded_fired, kinds_registered, source_extension_recorded"
)]
fn register_kind_contract() {
    let manifest = software();
    let (kind_reg, _, _, diags) = populate(&[manifest]);
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("invariant"));
    assert_eq!(
        kind_reg.get("behavior").unwrap().source_extension,
        "@specforge/software"
    );
    assert!(kind_reg.get("behavior").unwrap().testable);
    assert!(!diags.iter().any(|d| d.severity == Severity::Error));
}

// ===========================================================================
// B:register_edge_types_from_manifest (6 verifies)
// ===========================================================================

#[spec(
    behavior = "register_edge_types_from_manifest",
    verify = "edge type registered with label and description"
)]
fn register_edge_label_and_description() {
    let manifest = declare("@test/ext", |c| {
        c.kind("A", |k| {
            k.keyword("a");
        });
        c.kind("B", |k| {
            k.keyword("b");
        });
        c.edge("guards", |e| {
            e.description("A guards the B it names")
                .source_kind("a")
                .target_kind("b");
        });
        c.edge("touches", |e| {
            e.source_kind("a").target_kind("b");
        });
    });
    let (_, _, edge_reg, _) = populate(&[manifest]);
    let guards = edge_reg.get("guards").unwrap();
    assert_eq!(guards.label, "guards");
    assert_eq!(
        guards.description.as_deref(),
        Some("A guards the B it names")
    );
    let touches = edge_reg.get("touches").unwrap();
    assert_eq!(touches.label, "touches");
    assert_eq!(touches.description, None);
}

#[spec(
    behavior = "register_edge_types_from_manifest",
    verify = "source/target kind constraints recorded"
)]
fn register_edge_source_target_constraints() {
    let (_, _, edge_reg, _) = populate(&[software()]);
    let enforces = edge_reg.get("enforces").unwrap();
    assert_eq!(enforces.source_kind.as_deref(), Some("behavior"));
    assert_eq!(enforces.target_kind.as_deref(), Some("invariant"));
    assert_eq!(enforces.edge_style.as_deref(), Some("dashed"));
}

#[spec(
    behavior = "register_edge_types_from_manifest",
    verify = "duplicate edge label across extensions produces W-level warning"
)]
fn register_edge_duplicate_warning() {
    let m1 = software();
    let mut m2 = product();
    m2.edges.push(EdgeTypeDescriptor {
        label: "enforces".to_string(),
        source_kind: Some("feature".to_string()),
        target_kind: Some("behavior".to_string()),
        ..Default::default()
    });
    let (_, _, _, diags) = populate(&[m1, m2]);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W018" && d.message.contains("enforces"))
    );
}

#[spec(
    behavior = "register_edge_types_from_manifest",
    verify = "first-registered edge type wins on collision (topological order)"
)]
fn register_edge_first_wins() {
    let m1 = software();
    let mut m2 = product();
    m2.edges.push(EdgeTypeDescriptor {
        label: "enforces".to_string(),
        source_kind: Some("feature".to_string()),
        target_kind: Some("behavior".to_string()),
        edge_style: Some("dotted".to_string()),
        ..Default::default()
    });
    let (_, _, edge_reg, _) = populate(&[m1, m2]);
    let enforces = edge_reg.get("enforces").unwrap();
    assert_eq!(enforces.source_extension, "@specforge/software");
    assert_eq!(enforces.edge_style.as_deref(), Some("dashed"));
}

#[test]
fn register_edge_field_mapping() {
    let (_, _, edge_reg, _) = populate(&[product()]);
    assert!(edge_reg.contains("composes"));
}

#[spec(
    behavior = "register_edge_types_from_manifest",
    verify = "Register Edge Types From Manifest: edge type registration holds — extension_manifests_loaded_fired, edge_types_registered, constraints_recorded, duplicates_warned"
)]
fn register_edge_contract() {
    let manifest = software();
    let (_, _, edge_reg, diags) = populate(&[manifest]);
    assert!(edge_reg.contains("enforces"));
    let enforces = edge_reg.get("enforces").unwrap();
    assert_eq!(enforces.source_kind.as_deref(), Some("behavior"));
    assert_eq!(enforces.target_kind.as_deref(), Some("invariant"));
    assert!(!diags.iter().any(|d| d.severity == Severity::Error));
}

// ===========================================================================
// B:validate_manifest_v2_schema (5 verifies)
// ===========================================================================

#[spec(
    behavior = "validate_manifest_v2_schema",
    verify = "valid v2 manifest passes schema validation"
)]
fn manifest_v2_valid_passes() {
    let diags = shape(&declare("@specforge/software", |_| {}));
    assert!(diags.is_empty());
}

#[spec(
    behavior = "validate_manifest_v2_schema",
    verify = "missing required field produces hard error"
)]
fn manifest_v2_missing_required_field() {
    // The wasm path is the manifest's alone: this stays on the manifest.
    let manifest =
        manifest(r#"{"name": "", "version": "1.0.0", "manifestVersion": 2, "wasmPath": "x.wasm"}"#);
    let diags = validate_manifest(&manifest);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "E030" && d.message.contains("'name'"))
    );

    let manifest2 = self::manifest(
        r#"{"name": "@test/ext", "version": "1.0.0", "manifestVersion": 2, "wasmPath": ""}"#,
    );
    let diags2 = validate_manifest(&manifest2);
    assert!(
        diags2
            .iter()
            .any(|d| d.code == "E030" && d.message.contains("wasmPath"))
    );
}

#[spec(
    behavior = "validate_manifest_v2_schema",
    verify = "manifestVersion != 2 produces hard error"
)]
fn manifest_v2_wrong_version() {
    // A manifest version is the manifest's alone: this stays on the manifest.
    let manifest = manifest(
        r#"{"name": "@test/ext", "version": "1.0.0", "manifestVersion": 1, "wasmPath": "x.wasm"}"#,
    );
    let diags = validate_manifest(&manifest);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "E030" && d.message.contains("manifestVersion must be 2"))
    );
}

#[spec(
    behavior = "validate_manifest_v2_schema",
    verify = "unknown top-level field produces warning"
)]
fn manifest_v2_unknown_field() {
    // Unknown camelCase keys are the manifest's alone: this stays on the
    // manifest.
    let raw: serde_json::Value = serde_json::from_str(
        r#"{"name": "@test/ext", "version": "1.0.0", "manifestVersion": 2, "wasmPath": "x.wasm",
            "entityKnds": [], "entityKinds": [], "surfaces": null}"#,
    )
    .unwrap();

    let diags = specforge_registry::unknown_manifest_fields(&raw);

    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W138");
    assert_eq!(diags[0].severity, Severity::Warning);
    assert!(
        diags[0].message.contains("'entityKnds'"),
        "{}",
        diags[0].message
    );
    // A known field set to null or empty is not unknown, and the manifest
    // still parses.
    assert!(serde_json::from_value::<ManifestV2>(raw).is_ok());
}

#[spec(
    behavior = "validate_manifest_v2_schema",
    verify = "Validate Manifest V2 Schema: manifest v2 schema validation holds — manifest_json_available, schema_validated, malformed_diagnosed"
)]
fn manifest_v2_schema_contract() {
    let good_diags = shape(&declare("@specforge/software", |_| {}));
    assert!(good_diags.is_empty());

    // No name, no version, and a short name that names no CLI subcommand.
    let mut bad = ContributionsBuilder::new(ExtensionMeta::new("", ""));
    bad.meta.short = Some("Not Kebab".to_string());
    let bad_diags = shape(&bad.declaration());
    assert!(bad_diags.len() >= 3);
    assert!(bad_diags.iter().all(|d| d.code == "E030"));
}

// ===========================================================================
// B:detect_unknown_entity_kinds (5 verifies)
// ===========================================================================

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "unregistered keyword produces E024"
)]
fn detect_unknown_kinds_e024() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let entities = vec![EntityView::new(
        "unknown_thing",
        "u1",
        pinned(span("test.spec")),
    )];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E024");
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "E024 includes keyword name and source span"
)]
fn detect_unknown_kinds_e024_includes_info() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let s = SourceSpan {
        file: Sym::new("my/file.spec"),
        start_line: 42,
        start_col: 0,
        end_line: 42,
        end_col: 10,
    };
    let entities = vec![EntityView::new("unknown_thing", "u1", &s)];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags[0].message.contains("unknown_thing"));
    assert!(diags[0].message.contains("my/file.spec"));
    assert_eq!(diags[0].span.as_ref().unwrap().start_line, 42);
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "registered keyword does not produce E024"
)]
fn detect_unknown_kinds_registered_no_e024() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let entities = vec![EntityView::new("behavior", "b1", pinned(span("test.spec")))];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "define-block keywords not checked against KindRegistry"
)]
fn detect_unknown_kinds_define_not_checked() {
    let kind_reg = KindRegistry::new();
    let entities = vec![EntityView::new(
        "define",
        "my_define",
        pinned(span("test.spec")),
    )];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "Detect Unknown Entity Kinds: unknown entity kind detection holds — registries_populated_fired, structural_parse_ready, unknown_kinds_diagnosed, registered_kinds_accepted"
)]
fn detect_unknown_kinds_contract() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let unknown = vec![EntityView::new("xyzzy", "x1", pinned(span("t.spec")))];
    let d1 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&unknown, &kind_reg, None);
    assert!(d1.iter().any(|d| d.code == "E024"));
    let known = vec![EntityView::new("behavior", "b1", pinned(span("t.spec")))];
    let d2 = specforge_registry::compilation::detect_unknown_entity_kinds(&known, &kind_reg, None);
    assert!(d2.is_empty());
}

// ===========================================================================
// B:suggest_missing_extensions (4 verifies)
// ===========================================================================

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "E024 for keyword in index suggests the providing extension"
)]
fn suggest_missing_ext_known_keyword() {
    let kind_reg = KindRegistry::new();
    let mut entries = std::collections::HashMap::new();
    entries.insert("behavior".to_string(), "@specforge/software".to_string());
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_entries(entries);
    let entities = vec![EntityView::new("behavior", "b1", pinned(span("test.spec")))];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(
        &entities,
        &kind_reg,
        Some(&index),
    );
    assert!(
        diags[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("specforge add @specforge/software")
    );
}

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "E024 for keyword not in index suggests specforge search"
)]
fn suggest_missing_ext_unknown_keyword() {
    let kind_reg = KindRegistry::new();
    let index = specforge_registry::compilation::KeywordExtensionIndex::new();
    let entities = vec![EntityView::new("xyzzy", "x1", pinned(span("test.spec")))];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(
        &entities,
        &kind_reg,
        Some(&index),
    );
    assert!(
        diags[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("specforge search")
    );
}

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "keyword-to-extension index is loaded from bundled data file"
)]
fn bundled_keyword_index_maps_every_builtin_keyword() {
    // The bundled file must say what the builtins' own declarations say
    // (their pinned wire answers).
    let declarations = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../specforge-component/tests/declarations");
    let mut expected = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(&declarations).unwrap() {
        let src = entry.unwrap().path();
        let Ok(entities) = std::fs::read_to_string(src.join("describe_entities.json")) else {
            continue;
        };
        let handshake: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(src.join("handshake.json")).unwrap())
                .unwrap();
        let name = handshake["name"].as_str().unwrap().to_string();
        // The SDK greet fixture is pinned beside the builtins.
        if !name.starts_with("@specforge/") {
            continue;
        }
        let entities: serde_json::Value = serde_json::from_str(&entities).unwrap();
        for item in entities["items"].as_array().unwrap() {
            expected.insert(item["keyword"].as_str().unwrap().to_string(), name.clone());
        }
    }
    assert!(!expected.is_empty());

    let bundled = specforge_registry::compilation::KeywordExtensionIndex::bundled();
    for (keyword, extension) in &expected {
        assert_eq!(
            bundled.lookup(keyword),
            Some(extension.as_str()),
            "{keyword}"
        );
    }
    assert_eq!(bundled.lookup("xyzzy"), None);

    // Without an index argument, E024 uses the bundled one.
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(
        &[EntityView::new("feature", "f1", pinned(span("test.spec")))],
        &KindRegistry::new(),
        None,
    );
    assert!(
        diags[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("specforge add @specforge/product"),
        "{diags:?}"
    );
}

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "an undeclared field a builtin enhancement adds suggests its extension"
)]
fn bundled_field_index_maps_every_builtin_enhancement_field() {
    // The bundled file must say what the builtins' enhancements say (their
    // pinned wire answers).
    let declarations = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../specforge-component/tests/declarations");
    let mut expected = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(&declarations).unwrap() {
        let src = entry.unwrap().path();
        let Ok(enhancements) = std::fs::read_to_string(src.join("describe_enhancements.json"))
        else {
            continue;
        };
        let handshake: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(src.join("handshake.json")).unwrap())
                .unwrap();
        let name = handshake["name"].as_str().unwrap().to_string();
        // The SDK greet fixture is pinned beside the builtins.
        if !name.starts_with("@specforge/") {
            continue;
        }
        let enhancements: serde_json::Value = serde_json::from_str(&enhancements).unwrap();
        for item in enhancements["items"].as_array().unwrap() {
            let kind = item["target_kind"].as_str().unwrap();
            for field in item["fields"].as_array().into_iter().flatten() {
                let field = field["name"].as_str().unwrap();
                expected.insert(format!("{kind}.{field}"), name.clone());
            }
        }
    }
    assert!(expected.contains_key("invariant.expression"));
    let bundled_json: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!("../../../data/field-index.json")).unwrap();
    assert_eq!(bundled_json, expected);

    // W020 for such a field names the extension, as E024 does for a kind.
    let (kind_reg, field_reg, _, _) = populate(&[software()]);
    let diags = detect_unknown_entity_fields(
        &[
            EntityView::new("invariant", "i1", pinned(span("test.spec")))
                .with_fields(&["expression", "bogus"]),
        ],
        &kind_reg,
        &field_reg,
    );
    assert_eq!(diags.len(), 2, "{diags:?}");
    assert!(
        diags[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("specforge add @specforge/formal"),
        "{diags:?}"
    );
    assert_eq!(diags[1].suggestion, None, "{diags:?}");
}

#[test]
fn malformed_keyword_index_falls_back_to_search() {
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_json("{not json");
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(
        &[EntityView::new("feature", "f1", pinned(span("test.spec")))],
        &KindRegistry::new(),
        Some(&index),
    );
    assert!(
        diags[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("specforge search feature"),
        "{diags:?}"
    );
}

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "Suggest Missing Extensions: missing extension suggestions holds — e024_diagnostic_emitted, suggestion_provided, lazy_loading_enforced"
)]
fn suggest_missing_ext_contract() {
    let kind_reg = KindRegistry::new();
    let mut entries = std::collections::HashMap::new();
    entries.insert("behavior".to_string(), "@specforge/software".to_string());
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_entries(entries);
    let e1 = vec![EntityView::new("behavior", "b1", pinned(span("test.spec")))];
    let d1 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&e1, &kind_reg, Some(&index));
    assert!(
        d1[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("@specforge/software")
    );
    let e2 = vec![EntityView::new("xyzzy", "x1", pinned(span("test.spec")))];
    let d2 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&e2, &kind_reg, Some(&index));
    assert!(
        d2[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("specforge search")
    );
}

// ===========================================================================
// B:validate_registered_entity_fields: the load reports W021 for these
// (crates/specforge-project/tests/registered_fields.rs proves all 6
// verifies through Environment::load). An edge label a field maps to is
// still registered, as an implicit edge:
// ===========================================================================

#[test]
fn a_field_edge_label_is_registered_as_an_implicit_edge() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Task", |k| {
            k.keyword("task");
            k.field("owner", |f| {
                f.field_type(FieldType::Reference).edge("owns");
            });
        });
    });
    let (_kind_reg, _field_reg, edge_reg, _) = populate(&[manifest]);
    // "owns" should be auto-created as implicit edge during populate
    assert!(
        edge_reg.contains("owns"),
        "implicit edge 'owns' should exist"
    );
}

// ===========================================================================
// B:detect_duplicate_entity_kinds (4 verifies)
// ===========================================================================

#[spec(
    behavior = "detect_duplicate_entity_kinds",
    verify = "duplicate kind from two extensions produces E026"
)]
fn detect_dup_kinds_e026() {
    let m1 = software();
    let m2 = other_behavior();
    let diags = duplicate_entity_kinds(&[m1, m2]);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "E026" && d.message.contains("behavior"))
    );
}

#[spec(
    behavior = "detect_duplicate_entity_kinds",
    verify = "first extension in topological order owns the kind"
)]
fn detect_dup_kinds_first_wins() {
    let m1 = software();
    let m2 = other_behavior();
    let (kind_reg, _, _, _) = populate(&[m1, m2]);
    let behavior = kind_reg.get("behavior").unwrap();
    assert_eq!(behavior.source_extension, "@specforge/software");
}

#[spec(
    behavior = "detect_duplicate_entity_kinds",
    verify = "single extension registering a kind produces no diagnostic"
)]
fn detect_dup_kinds_single_ext_no_diag() {
    let diags = duplicate_entity_kinds(&[software()]);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "detect_duplicate_entity_kinds",
    verify = "Detect Duplicate Entity Kinds: duplicate entity kind detection holds — manifests_loading, collisions_detected, first_wins_enforced"
)]
fn detect_dup_kinds_contract() {
    let diags = duplicate_entity_kinds(&[software()]);
    assert!(diags.is_empty());
    let m2 = other_behavior();
    let dup_diags = duplicate_entity_kinds(&[software(), m2]);
    assert!(dup_diags.iter().any(|d| d.code == "E026"));
}

// ===========================================================================
// B:validate_peer_dependencies (4 verifies)
// ===========================================================================

#[spec(
    behavior = "validate_peer_dependencies",
    verify = "satisfied peer dependency passes validation"
)]
fn peer_deps_satisfied() {
    let m1 = software();
    let m2 = versioned(
        "@specforge/product",
        "1.0.0",
        vec![peer("@specforge/software", ">=1.0.0")],
    );
    let diags = peer_dependencies(&[m1, m2]);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "validate_peer_dependencies",
    verify = "missing peer dependency produces hard error"
)]
fn peer_deps_missing() {
    let m = versioned(
        "@specforge/product",
        "1.0.0",
        vec![peer("@specforge/software", ">=1.0.0")],
    );
    let diags = peer_dependencies(&[m]);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "E027" && d.message.contains("@specforge/software"))
    );
}

#[spec(
    behavior = "validate_peer_dependencies",
    verify = "incompatible version produces hard error with required range"
)]
fn peer_deps_incompatible_version() {
    let m1 = versioned("@specforge/software", "0.5.0", vec![]);
    let m2 = versioned(
        "@specforge/product",
        "1.0.0",
        vec![peer("@specforge/software", ">=1.0.0")],
    );
    let diags = peer_dependencies(&[m1, m2]);
    assert!(
        diags.iter().any(|d| d.code == "E027"
            && d.message.contains(">=1.0.0")
            && d.message.contains("0.5.0"))
    );
}

#[spec(
    behavior = "validate_peer_dependencies",
    verify = "missing optional peer dependency passes validation"
)]
fn peer_deps_missing_optional_peer_passes() {
    let m = versioned(
        "@specforge/software",
        "1.0.0",
        vec![optional_peer("@specforge/product", "^1.0")],
    );
    let diags = peer_dependencies(&[m]);
    assert!(diags.is_empty(), "{diags:?}");
}

/// Governance works without software (only ConstrainsBehavior targets a
/// software kind), so its peer on software is optional; with the peer
/// check on compile, a required one would fail governance-only projects.
#[spec(
    behavior = "ge_declare_manifest",
    verify = "peer_dependencies includes optional @specforge/software"
)]
fn governance_peer_on_software_is_optional() {
    let handshake = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../specforge-component/tests/declarations/governance/handshake.json");
    let handshake: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(handshake).unwrap()).unwrap();
    let software = handshake["peer_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "@specforge/software")
        .expect("governance declares a peer on software");
    assert_eq!(software["version"], "^1.0");
    assert_eq!(software["optional"], true);
}

#[spec(
    behavior = "validate_peer_dependencies",
    verify = "installed optional peer outside its range produces hard error"
)]
fn peer_deps_installed_optional_peer_is_range_checked() {
    let product = versioned("@specforge/product", "2.0.0", vec![]);
    let software = versioned(
        "@specforge/software",
        "1.0.0",
        vec![optional_peer("@specforge/product", "^1.0")],
    );
    let diags = peer_dependencies(&[product, software]);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "E027");
    assert!(diags[0].message.contains("^1.0") && diags[0].message.contains("2.0.0"));
}

#[spec(
    behavior = "validate_peer_dependencies",
    verify = "Validate Peer Dependencies: peer dependency validation holds — manifests_available, dependencies_validated, unsatisfied_blocked, loading_failed_emitted"
)]
fn peer_deps_contract() {
    let m1 = software();
    let m2 = versioned(
        "@specforge/product",
        "1.0.0",
        vec![peer("@specforge/software", ">=1.0.0")],
    );
    assert!(peer_dependencies(&[m1, m2]).is_empty());
    let m3 = versioned(
        "@specforge/product",
        "1.0.0",
        vec![peer("@specforge/missing", ">=1.0.0")],
    );
    let diags = peer_dependencies(&[m3]);
    assert!(diags.iter().any(|d| d.code == "E027"));
}

// ===========================================================================
// B:validate_extension_testability (5 verifies)
// ===========================================================================

#[spec(
    behavior = "validate_extension_testability",
    verify = "the registry build reports W017 for a testable kind without supportsVerify"
)]
fn the_registry_build_reports_w017() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Thing", |k| {
            k.keyword("thing").testable(true).supports_verify(false);
        });
        c.kind("Note", |k| {
            k.keyword("note").testable(false).supports_verify(true);
        });
    });
    let build = crate::build_registries(vec![manifest]);
    let codes: Vec<(&str, &str)> = build
        .registry_diagnostics
        .iter()
        .filter(|d| d.code == "W017" || d.code == "I006")
        .map(|d| (d.code.as_str(), d.message.as_str()))
        .collect();
    assert_eq!(codes.len(), 1, "{codes:?}");
    assert_eq!(codes[0].0, "W017");
    assert!(codes[0].1.contains("thing"), "{codes:?}");
}

#[spec(
    behavior = "validate_extension_testability",
    verify = "testable kind without supportsVerify produces W017"
)]
fn testability_w017() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Thing", |k| {
            k.keyword("thing").testable(true).supports_verify(false);
        });
    });
    let (kind_reg, _, _, _) = populate(&[manifest]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W017" && d.message.contains("thing"))
    );
}

#[spec(
    behavior = "validate_extension_testability",
    verify = "testable kind with supportsVerify=true passes"
)]
fn testability_passes() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(!diags.iter().any(|d| d.message.contains("behavior")));
}

#[spec(
    behavior = "validate_extension_testability",
    verify = "a kind that accepts verify statements but is not testable produces no diagnostic"
)]
fn testability_verify_without_testable_is_not_reported() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Note", |k| {
            k.keyword("note").testable(false).supports_verify(true);
        });
    });
    let (kind_reg, _, _, _) = populate(&[manifest]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(diags.is_empty(), "{diags:?}");
}

#[spec(
    behavior = "validate_extension_testability",
    verify = "consistent testable and supportsVerify flags produce no diagnostic"
)]
fn testability_consistent_no_diag() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Thing", |k| {
            k.keyword("thing").testable(false).supports_verify(false);
        });
    });
    let (kind_reg, _, _, _) = populate(&[manifest]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "validate_extension_testability",
    verify = "Validate Extension Testability: extension testability validation holds — registries_populated_fired, flag_consistency_checked, advisory_diagnostics_emitted"
)]
fn testability_contract() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(diags.is_empty());
    let bad = declare("@t/e", |c| {
        c.kind("X", |k| {
            k.keyword("x").testable(true).supports_verify(false);
        });
    });
    let (bad_kr, _, _, _) = populate(&[bad]);
    let bad_diags = validate_extension_testability(&bad_kr);
    assert!(bad_diags.iter().any(|d| d.code == "W017"));
}

// ===========================================================================
// B:register_validation_rules_from_manifest (6 verifies)
// ===========================================================================

#[spec(
    behavior = "register_validation_rules_from_manifest",
    verify = "validation rule registered from manifest"
)]
fn validation_rule_registered() {
    let manifest = declare("@test/ext", |c| {
        c.rule("W100", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("orphan {kind} '{id}'")
                .check(CheckKind::NoIncomingEdges)
                .target_kind("behavior");
        });
    });
    let (rules, diags) = register_validation_rules(&[manifest]);
    assert!(diags.is_empty());
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].code, "W100");
    assert_eq!(rules[0].check, "no_incoming_edges");
}

#[spec(
    behavior = "register_validation_rules_from_manifest",
    verify = "target_kind validation deferred to post-registration phase"
)]
fn validation_rule_target_kind_deferred() {
    let manifest = declare("@test/ext", |c| {
        c.rule("W100", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("test")
                .check(CheckKind::NoIncomingEdges)
                .target_kind("nonexistent_kind");
        });
    });
    let (rules, diags) = register_validation_rules(&[manifest]);
    assert!(
        diags.is_empty(),
        "rule registration should not validate target_kind"
    );
    assert_eq!(rules.len(), 1);
}

#[test]
fn validation_rule_target_kind_validated_post_registration() {
    let (kind_reg, field_reg, edge_reg, _) = populate(&[software()]);
    // Post-registration cross-validation catches unresolved refs
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    // Software manifest has valid references — should be clean
    assert!(!diags.iter().any(|d| d.message.contains("target_kind")));
}

#[test]
fn validation_rule_edge_type_validated_post_registration() {
    let (kind_reg, field_reg, edge_reg, _) = populate(&[software()]);
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    assert!(!diags.iter().any(|d| d.message.contains("edge label")));
}

#[test]
fn validation_rule_invalid_ref_warning() {
    let manifest = declare("@t/e", |c| {
        c.kind("Task", |k| {
            k.keyword("task");
            k.field("owner", |f| {
                f.field_type(FieldType::Reference).target_kind("person");
            });
        });
    });
    let (kind_reg, field_reg, edge_reg, _) = populate(&[manifest]);
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    // Should be a warning (W021), not error
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W021" && d.severity == Severity::Warning)
    );
    assert!(!diags.iter().any(|d| d.severity == Severity::Error));
}

// ===========================================================================
// B:register_extension_validation_rules (3 verifies — from spec)
// ===========================================================================

// Unlinked: the compile runs rules in manifest order; only this helper sorts.
#[test]
fn ext_validation_rules_sorted() {
    let m1 = rules(
        "@ext/a",
        &[
            ("W300", "third", CheckKind::NoIncomingEdges),
            ("W100", "first", CheckKind::NoIncomingEdges),
        ],
    );
    let m2 = rules("@ext/b", &[("W200", "second", CheckKind::NoOutgoingEdges)]);
    let (rules, _) = register_validation_rules(&[m1, m2]);
    let codes: Vec<&str> = rules.iter().map(|r| r.code.as_str()).collect();
    assert_eq!(codes, vec!["W100", "W200", "W300"]);
}

#[spec(
    behavior = "register_extension_validation_rules",
    verify = "rules from multiple extensions are collected"
)]
fn ext_validation_rules_multiple_extensions() {
    let m1 = rules("@ext/a", &[("W100", "a rule", CheckKind::NoIncomingEdges)]);
    let m2 = rules("@ext/b", &[("W200", "b rule", CheckKind::NoOutgoingEdges)]);
    let (rules, diags) = register_validation_rules(&[m1, m2]);
    assert_eq!(rules.len(), 2);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "register_extension_validation_rules",
    verify = "duplicate codes across extensions produce warning"
)]
fn ext_validation_rules_duplicate_codes() {
    let m1 = rules("@ext/a", &[("W100", "a", CheckKind::NoIncomingEdges)]);
    let m2 = rules("@ext/b", &[("W100", "b", CheckKind::NoIncomingEdges)]);
    let (_, diags) = register_validation_rules(&[m1, m2]);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W023" && d.message.contains("W100"))
    );
}

// ===========================================================================
// B:register_entity_enhancements (5 verifies)
// ===========================================================================

#[spec(
    behavior = "register_entity_enhancements",
    verify = "enhancement fields registered in FieldRegistry"
)]
fn enhancements_merge_fields() {
    let (mut kind_reg, mut field_reg, _, _) = populate(&[software()]);
    let enhancements = vec![enhancement("@test/coverage", "behavior", |e| {
        e.field("coverage_threshold", |f| {
            f.field_type(FieldType::String);
        });
    })];
    let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
    assert!(diags.is_empty());
    assert!(field_reg.contains("behavior", "coverage_threshold"));
}

#[spec(
    behavior = "register_entity_enhancements",
    verify = "unknown target kind produces I004 info diagnostic"
)]
fn enhancements_unknown_kind_i004() {
    let (mut kind_reg, mut field_reg, _, _) = populate(&[software()]);
    let enhancements = vec![enhancement("@test/ext", "nonexistent_kind", |e| {
        e.field("extra", |f| {
            f.field_type(FieldType::String);
        });
    })];
    let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "I004");
    assert!(diags[0].message.contains("nonexistent_kind"));
    assert!(!field_reg.contains("nonexistent_kind", "extra"));
}

#[spec(
    behavior = "register_entity_enhancements",
    verify = "enhancement field does NOT overwrite existing kind-level field"
)]
fn enhancements_no_overwrite() {
    let (mut kind_reg, mut field_reg, _, _) = populate(&[software()]);
    let enhancements = vec![enhancement("@test/ext", "behavior", |e| {
        e.field("contract", |f| {
            f.field_type(FieldType::String); // different type!
        });
    })];
    let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
    assert!(diags.is_empty());
    let contract = field_reg.get("behavior", "contract").unwrap();
    assert_eq!(contract.field_type, ManifestFieldType::Block);
    assert_eq!(contract.source_extension, "@specforge/software");
}

#[spec(
    behavior = "register_entity_enhancements",
    verify = "enhancement fields registered in FieldRegistry"
)]
fn enhancements_two_non_conflicting() {
    let (mut kind_reg, mut field_reg, _, _) = populate(&[software()]);
    let enhancements = vec![
        enhancement("@ext/a", "behavior", |e| {
            e.field("priority", |f| {
                f.field_type(FieldType::String);
            });
        }),
        enhancement("@ext/b", "behavior", |e| {
            e.field("category", |f| {
                f.field_type(FieldType::String);
            });
        }),
    ];
    let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
    assert!(diags.is_empty());
    assert!(field_reg.contains("behavior", "priority"));
    assert!(field_reg.contains("behavior", "category"));
}

#[spec(
    behavior = "register_entity_enhancements",
    verify = "Register Entity Enhancements: entity enhancement registration holds — manifests_validated, enhancement_registered_emitted, registration_before_resolve, registration_order_deterministic"
)]
fn enhancements_contract() {
    // Two extensions each add an `owner` field to behavior, with different types.
    let enhancer = |name: &str, field_type: FieldType| -> ExtensionDeclaration {
        declare(name, |c| {
            c.enhance("behavior", name, |e| {
                e.field("owner", |f| {
                    f.field_type(field_type);
                });
                e.field(&format!("{}_note", field_type.as_str()), |f| {
                    f.field_type(FieldType::String);
                });
            });
        })
    };
    let a = enhancer("@test/a", FieldType::String);
    let b = enhancer("@test/b", FieldType::Reference);

    // requires manifests_validated: every declaration passes its shape check.
    for d in [&software(), &a, &b] {
        assert!(shape(d).is_empty(), "{}", d.name());
    }

    let (kind_reg, field_reg, _, diags) = populate(&[software(), a.clone(), b.clone()]);
    assert!(diags.is_empty(), "{diags:?}");

    // enhancement_registered_emitted: each registered field records the
    // extension, target kind, field and type the event carries.
    let owner = field_reg.get("behavior", "owner").unwrap();
    assert_eq!(owner.kind_name, "behavior");
    assert_eq!(owner.source_extension, "@test/a");
    assert_eq!(owner.field_type, ManifestFieldType::String);
    let note = field_reg.get("behavior", "reference_note").unwrap();
    assert_eq!(note.source_extension, "@test/b");

    // registration_before_resolve: the registries populate hands
    // on already accept an enhanced field on a parsed entity.
    let unknown = detect_unknown_entity_fields(
        &[EntityView::new("behavior", "b1", pinned(span("main.spec")))
            .with_fields(&["owner", "string_note"])],
        &kind_reg,
        &field_reg,
    );
    assert!(unknown.is_empty(), "{unknown:?}");

    // registration_order_deterministic: the extensions array order decides
    // which `owner` wins, and the same order always gives the same result.
    let (_, swapped, _, _) = populate(&[software(), b.clone(), a.clone()]);
    let owner = swapped.get("behavior", "owner").unwrap();
    assert_eq!(owner.source_extension, "@test/b");
    assert_eq!(owner.field_type, ManifestFieldType::Reference);
    for _ in 0..3 {
        let (_, again, _, _) = populate(&[software(), a.clone(), b.clone()]);
        assert_eq!(
            again.get("behavior", "owner").unwrap().source_extension,
            "@test/a"
        );
    }
}

// ===========================================================================
// B:validate_extension_manifest_consistency (6 verifies)
// ===========================================================================

#[spec(
    behavior = "validate_extension_manifest_consistency",
    verify = "target_kind referencing own manifest kind passes"
)]
fn manifest_consistency_own_kind_passes() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Behavior", |k| {
            k.keyword("behavior");
            k.field("invariants", |f| {
                f.field_type(FieldType::ReferenceList)
                    .target_kind("invariant");
            });
        });
        c.kind("Invariant", |k| {
            k.keyword("invariant");
        });
    });
    let diags = consistency(&manifest, &[]);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "validate_extension_manifest_consistency",
    verify = "target_kind referencing peer dependency kind passes"
)]
fn manifest_consistency_peer_dep_passes() {
    let mut c = extension("@test/ext");
    c.meta
        .peer_dependencies
        .push(peer("@specforge/software", ">=1.0.0"));
    c.kind("Feature", |k| {
        k.keyword("feature");
        k.field("behaviors", |f| {
            f.field_type(FieldType::ReferenceList)
                .target_kind("behavior");
        });
    });
    let manifest = c.declaration();
    let diags = consistency(&manifest, &[]);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "validate_extension_manifest_consistency",
    verify = "self-contradictory target_kind produces a W021 warning"
)]
fn manifest_consistency_self_contradictory_target() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Behavior", |k| {
            k.keyword("behavior");
            k.field("invariants", |f| {
                f.field_type(FieldType::ReferenceList)
                    .target_kind("nonexistent_kind");
            });
        });
    });
    let diags = consistency(&manifest, &[]);
    let w021 = diags
        .iter()
        .find(|d| d.message.contains("nonexistent_kind"))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert_eq!(w021.code, "W021");
    assert_eq!(w021.severity, Severity::Warning);
}

#[spec(
    behavior = "validate_extension_manifest_consistency",
    verify = "target_kind referencing non-peer extension kind produces W-level warning"
)]
fn manifest_consistency_non_peer_warning() {
    // Peer of @specforge/software only, yet it points at product's `feature`.
    let mut c = extension("@test/ext");
    c.meta
        .peer_dependencies
        .push(peer("@specforge/software", ">=1.0.0"));
    c.kind("Task", |k| {
        k.keyword("task");
        k.field("behaviors", |f| {
            f.field_type(FieldType::ReferenceList)
                .target_kind("behavior");
        });
        k.field("features", |f| {
            f.field_type(FieldType::ReferenceList)
                .target_kind("feature");
        });
    });
    let manifest = c.declaration();
    let loaded = [software(), product(), manifest.clone()];
    let diags = consistency(&manifest, &loaded);

    // `behavior` comes from the peer and passes; `feature` exists, but only
    // in an extension this one does not depend on.
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W021");
    assert_eq!(diags[0].severity, Severity::Warning);
    assert!(
        diags[0].message.contains("'feature'") && diags[0].message.contains("'@specforge/product'"),
        "{}",
        diags[0].message
    );
}

#[spec(
    behavior = "validate_extension_manifest_consistency",
    verify = "self-contradictory edge label produces a W021 warning"
)]
fn manifest_consistency_self_contradictory_edge() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Behavior", |k| {
            k.keyword("behavior");
            k.field("invariants", |f| {
                f.field_type(FieldType::ReferenceList).edge("missing_edge");
            });
        });
    });
    let diags = consistency(&manifest, &[]);
    let w021 = diags
        .iter()
        .find(|d| d.message.contains("missing_edge"))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert_eq!(w021.code, "W021");
    assert_eq!(w021.severity, Severity::Warning);
}

#[spec(
    behavior = "validate_extension_manifest_consistency",
    verify = "a derived_from the host can't apply produces a W021 warning"
)]
fn manifest_consistency_unusable_derived_from() {
    let manifest = declare("@test/ext", |c| {
        c.kind("Shape", |k| {
            k.keyword("shape");
            k.field("parts", |f| {
                f.field_type(FieldType::ReferenceList)
                    .target_kind("shape")
                    .derived_from("type_expressions");
            });
            k.field("uses", |f| {
                f.field_type(FieldType::ReferenceList)
                    .target_kind("shape")
                    .derived_from("method_signatures");
            });
            k.field("guessed", |f| {
                f.field_type(FieldType::ReferenceList)
                    .target_kind("shape")
                    .derived_from("comments");
            });
            k.field("untargeted", |f| {
                f.field_type(FieldType::ReferenceList)
                    .derived_from("type_expressions");
            });
            k.field("note", |f| {
                f.field_type(FieldType::String)
                    .target_kind("shape")
                    .derived_from("type_expressions");
            });
        });
    });
    let diags = consistency(&manifest, &[]);

    let named: Vec<&str> = ["'parts'", "'uses'", "'guessed'", "'untargeted'", "'note'"]
        .into_iter()
        .filter(|field| diags.iter().any(|d| d.message.contains(field)))
        .collect();
    assert_eq!(named, ["'guessed'", "'untargeted'", "'note'"], "{diags:?}");
    assert!(
        diags
            .iter()
            .all(|d| d.code == "W021" && d.severity == Severity::Warning),
        "{diags:?}"
    );
}

#[spec(
    behavior = "validate_extension_manifest_consistency",
    verify = "Validate Extension Manifest Consistency: manifest self-consistency validation holds — manifest_parsed, peer_dependencies_known, self_consistency_validated, authoring_errors_diagnosed"
)]
fn manifest_consistency_contract() {
    let good = declare("@test/ext", |c| {
        c.kind("A", |k| {
            k.keyword("a");
            k.field("bs", |f| {
                f.field_type(FieldType::ReferenceList).target_kind("b");
            });
        });
        c.kind("B", |k| {
            k.keyword("b");
        });
        c.edge("links", |e| {
            e.source_kind("a").target_kind("b");
        });
    });
    let diags = consistency(&good, &[]);
    assert!(diags.is_empty());

    let bad = declare("@test/ext", |c| {
        c.kind("A", |k| {
            k.keyword("a");
            k.field("bs", |f| {
                f.field_type(FieldType::ReferenceList)
                    .target_kind("missing")
                    .edge("missing_edge");
            });
        });
    });
    let bad_diags = consistency(&bad, &[]);
    assert!(bad_diags.len() >= 2);
    assert!(bad_diags.iter().all(|d| d.code == "W021"));
}
