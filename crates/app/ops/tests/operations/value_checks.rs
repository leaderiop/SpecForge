//! Each checked value's outcome on the read surfaces (ADR 0336): inspect, `--with-checks`, `specforge extensions`.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_ops::extension;
use specforge_ops::read::export::{Format, Request};
use specforge_ops::read::inspect;
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_project::testing::TestProject;
use specforge_project::values::CheckStatus;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

use super::compiled_support::exported_text;

const EXT: &str = "@pin/surface";

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.code("E001", |d| {
        d.title("A todo").explanation("A todo.");
    });
    c.checker("no_todo", |k| {
        k.applies_to(&["string"])
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                let found = item.text.contains("TODO");
                PlannedItem::findings(
                    found
                        .then(|| CheckFinding::new("E001", "says TODO"))
                        .into_iter()
                        .collect(),
                )
            });
    });
    c.kind("service", |k| {
        k.keyword("service");
        k.field("home", |f| {
            f.field_type(FieldType::String).check("no_todo");
        });
    });
    c
}

fn compiled() -> CompiledProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file(
            "a.spec",
            "service a \"A\" {\n  home \"TODO later\"\n}\n\nservice b \"B\" {\n  home \"fine\"\n}\n",
        )
        .compile_in(Arc::new(InProcessRuntime::new().with(extension)))
}

fn node(document: &str, id: &str) -> Value {
    let document: Value = serde_json::from_str(document).unwrap();
    document["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()
        .clone()
}

#[specforge_test(
    behavior = "inspect_entity_facts",
    verify = "an entity lists each checked value's outcome"
)]
fn an_entity_lists_each_checked_values_outcome() {
    let project = compiled();
    let view = ProjectView::of(&project);
    let failed = inspect::inspect(&view, "a").unwrap().checks;
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].field, "home");
    assert_eq!(failed[0].value, "TODO later");
    assert_eq!(failed[0].checker, format!("{EXT}:no_todo"));
    assert_eq!(failed[0].status, CheckStatus::Failed);
    assert_eq!(failed[0].outcome.as_deref(), Some("ok"));
    assert_eq!(failed[0].codes, ["E001"]);
    assert_eq!(
        inspect::inspect(&view, "b").unwrap().checks[0].status,
        CheckStatus::Passed
    );
}

#[specforge_test(
    behavior = "export_agent_graph_format",
    verify = "with_checks adds each value's outcomes to a graph or context export, and a plain export is unchanged"
)]
fn with_checks_adds_outcomes_and_plain_export_is_unchanged() {
    let project = compiled();
    for format in [Format::Graph, Format::Context] {
        let text = |with_checks| {
            exported_text(
                &project,
                &Request {
                    format: Some(format),
                    with_checks,
                    ..Default::default()
                },
            )
            .unwrap()
        };
        let plain = text(false);
        assert!(
            node(&plain, "b").get("checks").is_none(),
            "{format:?}: {plain}"
        );
        let checked = node(&text(true), "b");
        assert_eq!(
            checked["checks"][0]["status"],
            json!("passed"),
            "{format:?}"
        );
        let mut without = checked.clone();
        without.as_object_mut().unwrap().remove("checks");
        assert_eq!(without, node(&plain, "b"), "{format:?}");
    }
}

#[specforge_test(
    behavior = "list_installed_extensions",
    verify = "the extensions listing names each checker"
)]
fn extensions_lists_checkers() {
    let project = compiled();
    let document = extension::list(&ProjectView::of(&project));
    let checkers = &document.extensions[0].checkers;
    assert_eq!(checkers.len(), 1, "{document:?}");
    assert_eq!(checkers[0].line(), "no_todo (string)");
}
