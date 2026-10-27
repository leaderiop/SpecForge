//! A computed field in the language server (ADR 0287).

use serde_json::json;
use specforge_extension_sdk::prelude::*;
use specforge_lsp::answers;
use specforge_test_macros::test as specforge_test;
use tower_lsp::lsp_types::{CompletionResponse, Position};

use crate::served::Served;

fn declare(c: &mut ContributionsBuilder) {
    c.kind("thing", |k| {
        k.keyword("thing");
        k.field("status", |f| {
            f.field_type(FieldType::String).computed();
        });
        k.field("note", |f| {
            f.field_type(FieldType::String);
        });
    });
    c.pass("give_status", |p| {
        p.compute()
            .computes(&["thing.status"])
            .run(|_: &PassInput| PassOutput {
                values: vec![PassValue {
                    entity: "a".into(),
                    field: "status".into(),
                    value: json!("passed"),
                }],
                ..Default::default()
            });
    });
}

#[specforge_test(
    behavior = "compute_field_values",
    verify = "hover marks a computed field and completion offers none"
)]
fn hover_marks_a_computed_field_and_completion_offers_none() {
    let served = Served::new(&[("main.spec", "thing a \"A\" {\n  \n}\n")])
        .extension("@t/computed", declare)
        .open(&["main.spec"]);
    let hover = served.hover_on("main.spec", "a").expect("a hover");
    assert!(hover.contains("`status` = passed *(computed)*"), "{hover}");

    let Some(CompletionResponse::Array(items)) = answers::completion(
        &served.state(),
        &served.uri("main.spec"),
        Position::new(1, 2),
    ) else {
        panic!("completion answers a list");
    };
    let labels: Vec<String> = items.into_iter().map(|i| i.label).collect();
    assert!(labels.contains(&"note".to_string()), "{labels:?}");
    assert!(!labels.contains(&"status".to_string()), "{labels:?}");
}
