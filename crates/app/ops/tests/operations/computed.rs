//! Every surface shows a computed value as a field (ADR 0287).

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_ops::read::export::{Format, Request};
use specforge_ops::read::inspect;
use specforge_ops::view::ProjectView;
use specforge_project::testing::{GSD_EXTENSION, TestProject, gsd_history_runtime};
use specforge_test_macros::test as specforge_test;

use super::compiled_support::exported_text;

#[specforge_test(
    behavior = "compute_field_values",
    verify = "every surface shows a computed value as a field and marks it computed"
)]
fn every_surface_shows_a_computed_value_as_a_field() {
    let runtime =
        gsd_history_runtime(json!([{"entity": "v1.0/03/V", "field": "status", "value": "passed"}]));
    let compiled = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[GSD_EXTENSION])
        .installing_enabled()
        .file(
            "a.spec",
            "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\nverification V \"V\" {\n  phase 03\n}\n",
        )
        .compile_in(Arc::new(runtime));
    let view = ProjectView::of(&compiled);

    for format in [Format::Graph, Format::Context] {
        let document: Value = serde_json::from_str(
            &exported_text(
                &compiled,
                &Request {
                    format: Some(format),
                    ..Default::default()
                },
            )
            .unwrap(),
        )
        .unwrap();
        let node = document["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "v1.0/03/V")
            .unwrap();
        assert_eq!(node["fields"]["status"], "passed", "{format:?}: {node}");
        assert_eq!(node["computed"], json!(["status"]), "{format:?}: {node}");
    }

    let facts = inspect::inspect(&view, "v1.0/03/V").unwrap();
    assert_eq!(facts.members.fields["status"].as_value(), "passed");
    assert!(facts.members.computed.contains("status"));
    // The graph beside it keeps authored values only.
    assert!(
        compiled
            .graph()
            .node("v1.0/03/V")
            .unwrap()
            .fields
            .get("status")
            .is_none()
    );
}
