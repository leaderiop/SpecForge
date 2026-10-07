use specforge_emitter::model::{FieldLevel, ModelFormat, ModelOptions};
use specforge_emitter::{
    GraphProtocolSchema, SchemaEntityKind, SchemaExtensionInfo, SchemaField, SchemaVersion,
};
use specforge_ops::model::{model, render_schema};
use specforge_test::prelude::*;

use crate::view_support::{Project, registries};

/// A schema built by hand, its one field typed `field_type`.
fn schema_with_a_field_typed(field_type: &str) -> GraphProtocolSchema {
    GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![SchemaExtensionInfo {
            name: "@t/soft".to_string(),
            version: "1.0.0".to_string(),
        }],
        entity_kinds: vec![SchemaEntityKind {
            name: "behavior".to_string(),
            source_extension: "@t/soft".to_string(),
            testable: true,
            dot_color: None,
            fields: vec![SchemaField {
                name: "sneaky".to_string(),
                field_type: field_type.to_string(),
                required: false,
                enum_values: None,
                edge: None,
                target_kind: None,
                description: None,
                default_value: None,
                source_extension: "@t/soft".to_string(),
            }],
        }],
        edge_types: vec![],
    }
}

#[specforge_test(
    behavior = "expose_model_mcp_tool",
    verify = "model warnings are W146 diagnostics on both surfaces"
)]
fn model_warnings_are_w146() {
    let options = ModelOptions {
        format: ModelFormat::Json,
        fields: FieldLevel::All,
        ..ModelOptions::default()
    };
    let outcome = render_schema(&schema_with_a_field_typed("stirng"), &[], &options);
    assert_eq!(outcome.warnings.len(), 1, "{:?}", outcome.warnings);
    let warning = &outcome.warnings[0];
    assert_eq!(warning.code, "W146");
    assert_eq!(warning.severity, specforge_common::Severity::Warning);
    assert_eq!(
        warning.message,
        "model: behavior field 'sneaky': unknown field type 'stirng'; rendering as string"
    );
    // The field is still rendered, as a string.
    assert!(outcome.rendered.contains("sneaky"), "{}", outcome.rendered);

    // A known type warns about nothing.
    let known = render_schema(&schema_with_a_field_typed("string"), &[], &options);
    assert!(known.warnings.is_empty(), "{:?}", known.warnings);

    // Nor does a schema built from the registries, whatever they declare.
    let project = Project::new("", registries(&["behavior"], &["constraint"]));
    let built = model(&project.view(), &options);
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
}

// Pin (plan 09 T0): flipped by T5, which names the type as declared.
#[test]
fn the_model_names_a_bool_field_boolean() {
    // As `@specforge/formal` declares `abstract`, through the registry build
    // (the model groups kinds by the extensions that declared them).
    let mut declaration = specforge_protocol_types::ExtensionDeclaration::default();
    declaration.handshake.name = "@t/formal".to_string();
    declaration.entities = vec![specforge_protocol_types::EntityKindDescriptor {
        name: "behavior".to_string(),
        keyword: Some("behavior".to_string()),
        fields: vec![specforge_protocol_types::FieldDescriptor {
            name: "abstract".to_string(),
            field_type: "bool".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    }];
    let project = Project::new("", specforge_registry::build_registries(vec![declaration]));
    let rendered = |format| {
        let options = ModelOptions {
            format,
            fields: FieldLevel::All,
            ..ModelOptions::default()
        };
        model(&project.view(), &options).rendered
    };

    let markdown = rendered(ModelFormat::Markdown);
    assert!(markdown.contains("| abstract | boolean |"), "{markdown}");
    assert!(rendered(ModelFormat::Mermaid).contains("boolean abstract"));
    assert!(rendered(ModelFormat::Dbml).contains("abstract boolean"));
    let json: serde_json::Value =
        serde_json::from_str(&rendered(ModelFormat::Json)).expect("the json model parses");
    let abstract_field = json["entities"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|e| e["fields"].as_array().into_iter().flatten())
        .find(|f| f["name"] == "abstract")
        .expect("abstract is a field of the model");
    assert_eq!(abstract_field["field_type"], "boolean");
}
