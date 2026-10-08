use serde_json::Value;
use specforge_emitter::model::{FieldLevel, ModelFormat, ModelOptions, ModelRoot};
use specforge_ops::model::model;
use specforge_protocol_types::{
    EdgeTypeDescriptor, EntityKindDescriptor, ExtensionDeclaration, FieldDescriptor,
    HandshakeResponse,
};
use specforge_registry::build_registries;
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

/// `@t/soft` declares `behavior` (its `events` a reference list to `event`
/// over `Triggers`) and `event`; `@t/test` is loaded and declares no kind.
fn two_extensions() -> Project {
    let declaration = |name: &str| ExtensionDeclaration {
        handshake: HandshakeResponse {
            name: name.into(),
            version: "1.0.0".into(),
            ..HandshakeResponse::default()
        },
        ..ExtensionDeclaration::default()
    };
    let mut soft = declaration("@t/soft");
    soft.entities = vec![
        EntityKindDescriptor {
            name: "behavior".into(),
            fields: vec![FieldDescriptor {
                name: "events".into(),
                field_type: "reference_list".into(),
                edge: Some("Triggers".into()),
                target_kind: Some("event".into()),
                ..FieldDescriptor::default()
            }],
            ..EntityKindDescriptor::default()
        },
        EntityKindDescriptor {
            name: "event".into(),
            ..EntityKindDescriptor::default()
        },
    ];
    soft.edges = vec![EdgeTypeDescriptor {
        label: "Triggers".into(),
        source_kind: Some("behavior".into()),
        target_kind: Some("event".into()),
        ..EdgeTypeDescriptor::default()
    }];
    Project::new("", build_registries(vec![soft, declaration("@t/test")]))
}

/// The JSON model `options` draws over `project`.
fn json_model(project: &Project, options: ModelOptions) -> Value {
    let text = model(
        &project.view(),
        &ModelOptions {
            format: ModelFormat::Json,
            ..options
        },
    );
    serde_json::from_str(&text).expect("the json model parses")
}

/// The kinds the JSON model lists, in order.
fn kinds_of(project: &Project, options: ModelOptions) -> Vec<String> {
    json_model(project, options)["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect()
}

/// Each extension of the JSON model: (name, entity_count, edge_count).
fn counts_of(project: &Project, options: ModelOptions) -> Vec<(String, u64, u64)> {
    json_model(project, options)["extensions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["name"].as_str().unwrap().to_string(),
                e["entity_count"].as_u64().unwrap(),
                e["edge_count"].as_u64().unwrap(),
            )
        })
        .collect()
}

// P1 - pins today's bug; the ticket that makes the model refuse replaces it.
#[specforge_test(
    behavior = "filter_model",
    verify = "unknown kind name is silently ignored"
)]
fn a_name_the_project_does_not_have_selects_nothing() {
    let project = two_extensions();
    assert_eq!(
        kinds_of(&project, ModelOptions::default()),
        ["behavior", "event"]
    );
    let model = json_model(&project, ModelOptions::default());
    assert_eq!(
        model["relationships"][0]["name"], "Triggers",
        "the guard: one relationship"
    );
    for options in [
        ModelOptions {
            root: Some(ModelRoot {
                kind: "behaviour".into(),
                depth: None,
            }),
            ..ModelOptions::default()
        },
        ModelOptions {
            extension: Some("soft".into()),
            ..ModelOptions::default()
        },
        ModelOptions {
            kinds: vec!["behaviour".into()],
            ..ModelOptions::default()
        },
    ] {
        assert!(
            kinds_of(&project, options.clone()).is_empty(),
            "{options:?}"
        );
    }
}

// P2 - kept.
#[specforge_test(
    behavior = "filter_model",
    verify = "multiple filters compose as intersection"
)]
fn the_model_filters_compose_as_an_intersection() {
    let project = two_extensions();
    // The root's kind is not kept when `kinds` omits it.
    assert_eq!(
        kinds_of(
            &project,
            ModelOptions {
                root: Some(ModelRoot {
                    kind: "event".into(),
                    depth: None,
                }),
                kinds: vec!["behavior".into()],
                ..ModelOptions::default()
            }
        ),
        ["behavior"]
    );
    assert_eq!(
        kinds_of(
            &project,
            ModelOptions {
                extension: Some("@t/soft".into()),
                kinds: vec!["event".into()],
                ..ModelOptions::default()
            }
        ),
        ["event"]
    );
    // Loaded, and declares no kind.
    assert!(
        kinds_of(
            &project,
            ModelOptions {
                extension: Some("@t/test".into()),
                ..ModelOptions::default()
            }
        )
        .is_empty()
    );
}

// P3 - kept.
#[test]
fn an_extensions_counts_after_a_filter() {
    let project = two_extensions();
    assert_eq!(
        counts_of(
            &project,
            ModelOptions {
                kinds: vec!["behavior".into()],
                ..ModelOptions::default()
            }
        ),
        [("@t/soft".into(), 1, 1), ("@t/test".into(), 0, 0)]
    );
    assert_eq!(
        counts_of(
            &project,
            ModelOptions {
                extension: Some("@t/test".into()),
                ..ModelOptions::default()
            }
        ),
        [("@t/soft".into(), 0, 0), ("@t/test".into(), 0, 0)]
    );
}
