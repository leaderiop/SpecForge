use specforge_emitter::model::{FieldLevel, ModelFormat, ModelOptions};
use specforge_ops::model::model;
use specforge_test::prelude::*;

use crate::view_support::Project;

#[specforge_test(
    behavior = "build_model_intermediate",
    verify = "a field's type is named as its extension declares it; DBML writes its own column type"
)]
fn the_model_names_a_field_type_as_declared() {
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
        model(&project.view(), &options)
    };

    let markdown = rendered(ModelFormat::Markdown);
    assert!(markdown.contains("| abstract | bool |"), "{markdown}");
    assert!(rendered(ModelFormat::Mermaid).contains("bool abstract"));
    assert!(rendered(ModelFormat::Dot).contains("abstract</td><td>bool</td>"));
    // DBML writes its own column type.
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
    assert_eq!(abstract_field["field_type"], "bool");
}
