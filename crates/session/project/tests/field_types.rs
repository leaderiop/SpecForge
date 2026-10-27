//! Field values against their declared types, through the graph build and
//! the environment's checks every surface (check, watch, LSP) runs.

use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_parser::FieldValue;
use specforge_project::testing::Assembled;
use specforge_protocol_types::{
    EntityKindDescriptor, ExtensionDeclaration, FieldDescriptor, HandshakeResponse,
};
use specforge_test_macros::test as specforge_test;

/// An extension declaring a `ticket` with one field of each checked type.
fn ticket_manifest() -> ExtensionDeclaration {
    let field = |name: &str, field_type: &str, enum_values: &[&str]| FieldDescriptor {
        name: name.into(),
        field_type: field_type.into(),
        enum_values: enum_values.iter().map(|v| v.to_string()).collect(),
        ..FieldDescriptor::default()
    };
    ExtensionDeclaration {
        handshake: HandshakeResponse {
            name: "@test/tickets".into(),
            version: "1.0.0".into(),
            ..HandshakeResponse::default()
        },
        entities: vec![EntityKindDescriptor {
            name: "Ticket".into(),
            keyword: Some("ticket".into()),
            fields: vec![
                field("priority", "enum", &["low", "medium", "high"]),
                field("urgent", "bool", &[]),
                field("points", "integer", &[]),
                field("labels", "string_list", &[]),
                field("summary", "string", &[]),
                field("depends_on", "reference_list", &[]),
                field("parent", "reference", &[]),
            ],
            ..EntityKindDescriptor::default()
        }],
        ..ExtensionDeclaration::default()
    }
}

/// Build and check `source` against the ticket extension's registries.
fn build_and_check(source: &str) -> (Graph, Vec<Diagnostic>) {
    let project = Assembled::over(specforge_registry::build_registries(
        vec![ticket_manifest()],
    ));
    let env = project.compiled().environment();
    let pop_diags: Vec<_> = env.registries().diagnostics().collect();
    assert!(pop_diags.is_empty(), "{pop_diags:?}");
    let parsed = specforge_parser::parse(source, "main.spec");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let build = env.graph_of([parsed]);
    let entities = env.entity_snapshot(&build);
    let mut diags = build.diagnostics().to_vec();
    diags.extend(env.run_checks(&entities, &specforge_registry::MemoryDisk::new()));
    (build.into_parts().0, diags)
}

fn e061(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.code == "E061").collect()
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a value that is not the declared integer, bool or enum type is an error"
)]
fn values_that_are_not_the_declared_bool_or_enum_are_errors() {
    let (_graph, diags) = build_and_check(
        r#"ticket t1 "T" {
  priority urgent
  urgent maybe
  points 3
}
"#,
    );

    let errors = e061(&diags);
    let messages: Vec<&str> = errors.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        messages,
        vec![
            "field 'priority' of ticket 't1' is declared enum (low, medium, high), but was given urgent",
            "field 'urgent' of ticket 't1' is declared bool, but was given maybe",
        ]
    );
    for d in &errors {
        assert_eq!(d.severity, specforge_common::Severity::Error);
    }
    // Each points at its value.
    let spans: Vec<(usize, usize)> = errors
        .iter()
        .map(|d| {
            let span = d.span.as_ref().unwrap();
            (span.start_line, span.start_col)
        })
        .collect();
    assert_eq!(spans, vec![(2, 12), (3, 10)]);
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "an enum value suggests the closest declared value"
)]
fn an_enum_value_suggests_the_closest_declared_value() {
    let (_graph, diags) = build_and_check(
        r#"ticket t1 "T" {
  priority hgh
}
"#,
    );

    let errors = e061(&diags);
    assert_eq!(errors.len(), 1, "{diags:?}");
    assert_eq!(
        errors[0].suggestion.as_deref(),
        Some("did you mean 'high'?")
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "Check Field Value Types: declared field types hold — registries_populated_fired, off_form_warned, single_values_listed, mismatches_diagnosed, undeclared_untouched"
)]
fn check_field_value_types_contract() {
    let (graph, diags) = build_and_check(
        r#"ticket t1 "T" {
  priority "medium"
  urgent "true"
  points "8"
  labels "one"
  summary 42
  note "free text"
}
"#,
    );
    // requires registries_populated: the field types come from the manifest.
    // ensures single_values_listed and coercion: the stored values carry
    // the declared types, with no diagnostic.
    assert!(e061(&diags).is_empty(), "{diags:?}");
    let node = graph.node("t1").unwrap();
    assert!(
        matches!(node.fields.get("labels"), Some(FieldValue::List(l)) if l.len() == 1 && l[0].text == "one")
    );
    assert!(matches!(
        node.fields.get("urgent"),
        Some(FieldValue::Boolean(true))
    ));
    assert!(matches!(
        node.fields.get("points"),
        Some(FieldValue::Integer(8))
    ));
    assert!(matches!(node.fields.get("summary"), Some(FieldValue::String(s)) if s == "42"));
    assert!(matches!(node.fields.get("priority"), Some(FieldValue::String(s)) if s == "medium"));
    // ensures undeclared_untouched: an unknown field keeps its value and is
    // W020's, not E061's.
    assert!(matches!(node.fields.get("note"), Some(FieldValue::String(s)) if s == "free text"));
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W020" && d.message.contains("'note'"))
    );

    // ensures mismatches_diagnosed.
    let (_graph, diags) = build_and_check(
        r#"ticket t2 "T" {
  points "many"
  summary ["a", "b"]
}
"#,
    );
    assert_eq!(e061(&diags).len(), 2, "{diags:?}");
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "no reference field of the repository's spec holds a bare number, date or boolean"
)]
fn no_reference_field_in_the_corpus_holds_a_number_or_a_date() {
    use specforge_parser::{FieldValue, Written};
    use std::collections::{HashMap, HashSet};

    // keyword -> the names of its reference and reference_list fields, over the builtins.
    let mut reference_fields: HashMap<String, HashSet<String>> = HashMap::new();
    for kind in specforge_component::committed::pinned_builtin_declarations()
        .unwrap()
        .iter()
        .flat_map(|d| &d.entities)
    {
        let Some(keyword) = &kind.keyword else {
            continue;
        };
        for field in &kind.fields {
            if matches!(field.field_type.as_str(), "reference" | "reference_list") {
                reference_fields
                    .entry(keyword.clone())
                    .or_default()
                    .insert(field.name.clone());
            }
        }
    }
    assert!(!reference_fields.is_empty());
    let mut checked = 0usize;
    for (path, source) in specforge_parser::testing::corpus() {
        let file = specforge_parser::parse(source, &path.to_string_lossy());
        for entity in &file.entities {
            let Some(names) = reference_fields.get(entity.kind.raw.as_str()) else {
                continue;
            };
            for entry in entity.fields.entries() {
                if !names.contains(entry.key.as_str()) {
                    continue;
                }
                let literal = match &entry.value {
                    FieldValue::Integer(_) | FieldValue::Boolean(_) | FieldValue::Date(_) => true,
                    FieldValue::List(items) => items.iter().any(|i| i.written == Written::Literal),
                    _ => false,
                };
                assert!(
                    !literal,
                    "{}: {} {} holds a number, date or boolean in `{}`",
                    path.display(),
                    entity.kind.raw.as_str(),
                    entity.id.raw.as_str(),
                    entry.key.as_str()
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked > 100,
        "only {checked} reference values were checked"
    );
}

#[specforge_test(
    behavior = "check_field_value_types",
    verify = "a reference field reads a bare word as the name it spells, whether or not the word could be a number or a date"
)]
fn a_reference_field_names_a_gsd_entity() {
    let (graph, diags) = build_and_check(
        r#"ticket 03-02 "A" {
}
ticket 03 "B" {
}
ticket 10 "C" {
}
ticket 2-1 "D" {
}
ticket v1.0 "E" {
}
ticket t1 "T" {
  parent 2-1
  depends_on [03-02, 03, v1.0]
}
ticket t2 "T" {
  parent 10
}
ticket t3 "T" {
  parent -5
}
"#,
    );
    let targets = |id: &str| -> Vec<String> {
        let mut t: Vec<String> = graph
            .edges_from(id)
            .iter()
            .map(|e| e.target.to_string())
            .collect();
        t.sort();
        t
    };
    assert_eq!(targets("t1"), ["03", "03-02", "2-1", "v1.0"]);
    assert_eq!(targets("t2"), ["10"]);
    let errors = e061(&diags);
    assert_eq!(errors.len(), 1, "{diags:?}");
    assert!(errors[0].message.contains("parent") && errors[0].message.contains("t3"));
    assert!(diags.iter().all(|d| d.code != "E003"), "{diags:?}");
}
