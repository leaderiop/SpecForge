// `build_registries`: the one call that turns loaded manifests into
// everything the compiler needs from them (architecture plan 05, step R1).
// In-memory manifests, no Wasm.

use specforge_test_macros::test as spec;

use specforge_registry::{ManifestV2, build_registries};

fn manifest(json: &str) -> ManifestV2 {
    serde_json::from_str(json).unwrap()
}

/// Kinds with fields, a required field, a single reference, a body parser,
/// an edge, a rule and a CLI command.
fn software() -> ManifestV2 {
    manifest(
        r#"{
            "name": "@test/software",
            "version": "1.2.0",
            "manifestVersion": 2,
            "wasmPath": "software.wasm",
            "entityKinds": [
                {
                    "name": "Behavior",
                    "keyword": "behavior",
                    "testable": true,
                    "fields": [
                        { "name": "contract", "fieldType": "string", "required": true },
                        { "name": "owner", "fieldType": "reference", "edge": "owned_by", "targetKind": "team" },
                        { "name": "types", "fieldType": "reference_list", "edge": "uses", "targetKind": "type" }
                    ]
                },
                { "name": "Type", "keyword": "type", "hasBodyParser": true }
            ],
            "edgeTypes": [
                { "label": "uses", "sourceKind": "behavior", "targetKind": "type" }
            ],
            "validationRules": [
                {
                    "code": "W901",
                    "severity": "warning",
                    "messageTemplate": "behavior '{id}' uses nothing",
                    "check": "no_outgoing_edges",
                    "targetKind": "behavior",
                    "edgeType": "uses"
                }
            ],
            "surfaces": { "commands": [
                { "id": "hello", "title": "Hello", "description": "hello", "export": "run_hello" }
            ] }
        }"#,
    )
}

fn product() -> ManifestV2 {
    manifest(
        r#"{
            "name": "@test/product",
            "version": "0.3.0",
            "manifestVersion": 2,
            "wasmPath": "product.wasm",
            "entityKinds": [ { "name": "Feature", "keyword": "feature" } ],
            "surfaces": { "commands": [
                { "id": "hello", "title": "Hello", "description": "hello again", "export": "run_hello" }
            ] }
        }"#,
    )
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
        build.extension_info,
        vec![
            ("@test/software".to_string(), "1.2.0".to_string()),
            ("@test/product".to_string(), "0.3.0".to_string()),
        ]
    );
    assert_eq!(build.manifests.len(), 2);
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
    assert_eq!(build.manifest_surfaces.len(), 2);
}

/// Two extensions declaring one rule code: the build warns (W023), after
/// the rule-parse diagnostics, and keeps both rules.
#[spec(
    behavior = "register_extension_validation_rules",
    verify = "duplicate codes across extensions produce warning"
)]
fn a_rule_code_declared_by_two_extensions_warns() {
    let rule = |name: &str| {
        manifest(&format!(
            r#"{{"name":"{name}","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
                "validationRules":[{{"code":"W100","severity":"warning","messageTemplate":"m",
                "check":"no_incoming_edges"}}]}}"#
        ))
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

#[test]
fn no_manifests_build_empty_registries() {
    let build = build_registries(Vec::new());

    assert!(build.kinds.is_empty());
    assert!(build.rules.is_empty());
    assert!(build.single_reference_fields.is_empty());
    assert!(build.registry_diagnostics.is_empty());
    assert!(build.surface_diagnostics.is_empty());
}
