//! Golden round-trip: the SDK's wire output must match, byte-for-byte in
//! content (order-insensitive objects), what the host accepts from the
//! hand-written builtin extensions. This is the pin behind "typed descriptors
//! shared wire-exact with the host" (wayfinder map #1, ticket #2).

use specforge_extension_sdk::{
    ContributionsBuilder, ExtensionMeta, PeerDependency, SandboxPolicy, prelude::*,
};

fn software_builder() -> ContributionsBuilder {
    let mut meta = ExtensionMeta::new("@specforge/software", "1.0.0");
    meta.peer_dependencies = vec![PeerDependency {
        name: "@specforge/product".to_string(),
        version: "^1.0".to_string(),
        optional: false,
    }];
    meta.sandbox_policy = Some(SandboxPolicy {
        max_memory_mb: Some(256),
        max_execution_ms: Some(5000),
        allowed_domains: vec![],
        allowed_paths: vec![],
        allowed_output_extensions: vec![],
        network_access: Some(false),
        file_system_access: Some(false),
    });

    let mut b = ContributionsBuilder::new(meta);
    b.kind("Behavior", |k| {
        k.description("A testable unit of system functionality with a defined contract")
            .testable(true)
            .supports_verify(true)
            .dot_shape("box")
            .dot_color("#1565C0")
            .dot_fillcolor("#E3F2FD")
            .field("contract", |f| {
                f.field_type(FieldType::String)
                    .required()
                    .description("The behavioral contract this behavior guarantees");
            })
            .field("invariants", |f| {
                f.field_type(FieldType::ReferenceList)
                    .description("Invariants this behavior enforces")
                    .edge("BehaviorEnforcesInvariant")
                    .target_kind("invariant")
                    .inverse_of("enforced_by");
            })
            .field("types", |f| {
                f.field_type(FieldType::ReferenceList)
                    .description("Type definitions used by this behavior")
                    .edge("BehaviorReferencesType")
                    .target_kind("type");
            })
            .field("ports", |f| {
                f.field_type(FieldType::ReferenceList)
                    .description("Port interfaces this behavior interacts with")
                    .edge("BehaviorUsesPort")
                    .target_kind("port");
            });
    });
    b.rule("W001", |r| {
        r.check(CheckKind::NoOutgoingEdges)
            .target_kind("behavior")
            .edge_type("BehaviorImplementsFeature")
            .severity(ValidationSeverity::Warning)
            .message_template("behavior '{id}' does not implement any feature");
    });
    b
}

#[test]
fn handshake_matches_builtin_wire_format() {
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../../extensions/software/src/handshake.json"
    ))
    .unwrap();
    let built: serde_json::Value =
        serde_json::from_str(&software_builder().handshake_json()).unwrap();
    assert_eq!(
        built, golden,
        "SDK handshake diverged from the builtin wire format"
    );
}

#[test]
fn entities_describe_matches_builtin_wire_format() {
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../../extensions/software/src/describe_entities.json"
    ))
    .unwrap();

    // The SDK slice contributes only the `behavior` kind; the builtin also
    // ships 5 more kinds. Compare the first item of the builtin against the
    // SDK's full items array.
    let built: serde_json::Value = serde_json::from_str(
        &software_builder()
            .describe_response_json("entities")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(built["category"], "entities");
    // The builtin behavior kind carries 13 fields; the SDK slice models the
    // first 4. Pin kind identity plus the fully-mirrored contract/invariants
    // fields.
    assert_eq!(built["items"][0]["name"], golden["items"][0]["name"]);
    assert_eq!(
        built["items"][0]["description"],
        golden["items"][0]["description"]
    );
    assert_eq!(
        built["items"][0]["dot_shape"],
        golden["items"][0]["dot_shape"]
    );
    assert_eq!(
        built["items"][0]["fields"][0], golden["items"][0]["fields"][0],
        "contract field drifted"
    );
    assert_eq!(
        built["items"][0]["fields"][1], golden["items"][0]["fields"][1],
        "invariants field drifted"
    );
}

#[test]
fn validation_rules_describe_matches_builtin_wire_format() {
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../../extensions/software/src/describe_validation_rules.json"
    ))
    .unwrap();
    let built: serde_json::Value = serde_json::from_str(
        &software_builder()
            .describe_response_json("validation_rules")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(built["category"], "validation_rules");
    assert_eq!(built["items"][0], golden["items"][0], "W001 rule drifted");
}

#[test]
fn flags_derive_from_contributions() {
    let b = software_builder();
    let f = serde_json::from_str::<serde_json::Value>(&b.handshake_json()).unwrap()["contribution_flags"]
        .clone();
    assert_eq!(f["entities"], true);
    assert_eq!(f["validators"], true);
    assert_eq!(f["renderers"], false);
    assert_eq!(f["prompts"], false);
}

#[test]
fn mock_host_pins_wire_output() {
    use specforge_extension_sdk::testing::MockHost;
    let host = MockHost::new(software_builder());
    host.assert_describe(
        "validation_rules",
        r#"{ "category": "validation_rules", "items": [ { "check": "no_outgoing_edges", "code": "W001", "constraint": null, "edge_type": "BehaviorImplementsFeature", "field": null, "message_template": "behavior '{id}' does not implement any feature", "severity": "warning", "target_kind": "behavior", "wasm_function": null } ] }"#,
    );
}
