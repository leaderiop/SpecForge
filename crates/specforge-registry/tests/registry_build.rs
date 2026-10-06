// `build_registries`: the one call that turns loaded declarations into
// everything the compiler needs from them (architecture plan 05, step R1).
// In-memory declarations, no Wasm.

use specforge_test_macros::test as spec;

use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::build_registries;

/// A builder for the extension `name` in `version`.
fn extension(name: &str, version: &str) -> ContributionsBuilder {
    ContributionsBuilder::new(ExtensionMeta::new(name, version))
}

/// The command `hello`, described as `description`.
fn hello(c: &mut ContributionsBuilder, description: &str) {
    c.command("hello", |cmd| {
        cmd.title("Hello")
            .description(description)
            .handler(|_| CommandOutput::ok("hello"));
    });
}

/// Kinds with fields, a required field, a single reference, a body parser,
/// an edge, a rule and a CLI command.
fn software() -> ExtensionDeclaration {
    let mut c = extension("@test/software", "1.2.0");
    c.kind("Behavior", |k| {
        k.keyword("behavior").testable(true).supports_verify(true);
        k.field("contract", |f| {
            f.field_type(FieldType::String).required();
        });
        k.field("owner", |f| {
            f.field_type(FieldType::Reference)
                .edge("owned_by")
                .target_kind("team");
        });
        k.field("types", |f| {
            f.field_type(FieldType::ReferenceList)
                .edge("uses")
                .target_kind("type");
        });
    });
    c.kind("Type", |k| {
        k.keyword("type").has_body_parser();
    });
    c.edge("uses", |e| {
        e.source_kind("behavior").target_kind("type");
    });
    c.rule("W901", |r| {
        r.severity(ValidationSeverity::Warning)
            .message_template("behavior '{id}' uses nothing")
            .check(CheckKind::NoOutgoingEdges)
            .target_kind("behavior")
            .edge_type("uses");
    });
    hello(&mut c, "hello");
    c.declaration()
}

fn product() -> ExtensionDeclaration {
    let mut c = extension("@test/product", "0.3.0");
    c.kind("Feature", |k| {
        k.keyword("feature");
    });
    hello(&mut c, "hello again");
    c.declaration()
}

#[test]
fn kinds_fields_and_edges_come_from_the_descriptors() {
    let build = build_registries(vec![software(), product()]);

    let kinds: std::collections::BTreeSet<&str> =
        build.kinds.keywords().map(String::as_str).collect();
    assert_eq!(kinds, ["behavior", "feature", "type"].into_iter().collect());
    assert!(build.fields.contains("behavior", "contract"));
    assert!(build.fields.contains("behavior", "owner"));
    assert!(build.edges.contains("uses"));
    assert_eq!(
        build.extension_info().collect::<Vec<_>>(),
        vec![("@test/software", "1.2.0"), ("@test/product", "0.3.0")]
    );
    assert_eq!(build.declarations().len(), 2);
}

#[spec(
    behavior = "load_extension_manifests",
    verify = "two extensions loaded and registries populated without collision"
)]
fn two_extensions_populate_without_collision() {
    let build = build_registries(vec![software(), product()]);

    assert!(build.kinds.contains("behavior"));
    assert!(build.kinds.contains("feature"));
    assert_eq!(
        build.kinds.get("feature").unwrap().source_extension,
        "@test/product"
    );
    assert!(
        build.registry_diagnostics.is_empty(),
        "no collision, no diagnostic: {:?}",
        build.registry_diagnostics
    );
}

#[test]
fn rules_include_the_manifest_rules_and_the_required_field_rules() {
    let build = build_registries(vec![software()]);

    let codes: Vec<(&str, &str)> = build
        .rules
        .iter()
        .map(|(p, origin)| (p.code.as_str(), origin.as_str()))
        .collect();
    // The extension's rule keeps its origin (for custom-rule dispatch);
    // the host-generated E006 rule for `contract` has none.
    assert!(codes.contains(&("W901", "@test/software")), "{codes:?}");
    assert!(codes.contains(&("E006", "")), "{codes:?}");
}

#[test]
fn derived_graph_inputs_match_the_manifests() {
    let build = build_registries(vec![software(), product()]);

    assert_eq!(
        build.body_parser_kinds,
        ["type".to_string()].into_iter().collect()
    );
    assert!(
        build
            .single_reference_fields
            .contains(&("behavior".to_string(), "owner".to_string()))
    );
    assert!(
        !build
            .single_reference_fields
            .contains(&("behavior".to_string(), "types".to_string())),
        "a reference list is not a single reference"
    );
    // `team` is declared by no loaded extension.
    assert_eq!(
        build
            .absent_reference_targets
            .get(&("behavior".to_string(), "owner".to_string()))
            .map(String::as_str),
        Some("team")
    );
}

#[test]
fn surface_conflicts_land_in_their_own_bucket() {
    let build = build_registries(vec![software(), product()]);

    let surface: Vec<&str> = build
        .surface_diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert_eq!(surface, ["E039"]);
    assert!(
        build.registry_diagnostics.iter().all(|d| d.code != "E039"),
        "E039 is reported last, apart from the registry diagnostics"
    );
    assert_eq!(build.surfaces.len(), 1, "first registration wins");
    assert_eq!(
        build
            .declarations()
            .iter()
            .map(|d| d.surfaces.commands.len())
            .sum::<usize>(),
        2,
        "each declaration keeps its own surfaces"
    );
}

/// Two extensions declaring one rule code: the build warns (W023), after
/// the rule-parse diagnostics, and keeps both rules.
#[spec(
    behavior = "registry_build_rules",
    verify = "a rule code two extensions declare is W023 and both rules are kept"
)]
fn a_rule_code_declared_by_two_extensions_warns() {
    let rule = |name: &str| {
        let mut c = extension(name, "1.0.0");
        c.rule("W100", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("m")
                .check(CheckKind::NoIncomingEdges);
        });
        c.declaration()
    };

    let build = build_registries(vec![rule("@ext/a"), rule("@ext/b")]);

    let codes: Vec<&str> = build
        .registry_diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert_eq!(codes, ["W023"]);
    let message = &build.registry_diagnostics[0].message;
    for part in ["'W100'", "'@ext/a'", "'@ext/b'"] {
        assert!(message.contains(part), "{message}");
    }
    assert_eq!(
        build.rules.iter().filter(|(r, _)| r.code == "W100").count(),
        2
    );
}

/// With no extension loaded the host knows no kind, field or edge: every
/// registry starts empty and stays so.
#[spec(
    behavior = "build_registries_from_declarations",
    verify = "a build of no declarations has empty registries, no rules and no diagnostics"
)]
fn no_declarations_build_empty_registries() {
    let build = build_registries(Vec::new());

    assert!(build.kinds.is_empty());
    assert_eq!(build.kinds.len(), 0);
    assert_eq!(build.kinds.keywords().count(), 0);
    assert!(build.kinds.get("behavior").is_none());
    assert!(build.fields.is_empty());
    assert!(build.fields.get("behavior", "contract").is_none());
    assert!(build.fields.fields_for_kind("behavior").is_empty());
    assert!(build.edges.is_empty());
    assert!(build.edges.get("enforces").is_none());
    assert_eq!(build.edges.labels().count(), 0);
    assert!(build.rules.is_empty());
    assert!(build.surfaces.is_empty());
    assert!(build.passes.is_empty());
    assert!(build.body_parser_kinds.is_empty());
    assert!(build.single_reference_fields.is_empty());
    assert!(build.bidirectional_pairs.is_empty());
    assert!(build.absent_reference_targets.is_empty());
    assert!(build.declarations().is_empty());
    assert!(build.declaration_diagnostics.is_empty());
    assert!(build.registry_diagnostics.is_empty());
    assert!(build.surface_diagnostics.is_empty());
}
