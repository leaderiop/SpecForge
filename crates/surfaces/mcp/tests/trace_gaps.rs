//! The trace prompt's gaps over references that do not resolve (ADR 0232).

use crate::support::*;
use serde_json::json;
use specforge_test::prelude::*;

#[specforge_test(
    behavior = "provide_mcp_trace_prompt",
    verify = "the trace prompt lists no gap for a field that writes a reference that does not resolve"
)]
fn an_unresolved_reference_is_no_trace_prompt_gap() {
    // `pay` writes a feature that does not exist, and no invariant.
    let mut server = TestProject::new()
        .file(
            "main.spec",
            "behavior pay \"Pay\" {\n    features [nowhere]\n}\n",
        )
        .serve(&[TestExtension::software()]);
    let payload = prompt_payload(&get_prompt(
        &mut server,
        "specforge://prompts/trace",
        json!({"entity_id": "pay"}),
    ));
    let gaps: Vec<&str> = payload["coverage_gaps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|gap| gap["gap_context"].as_str().unwrap())
        .collect();
    assert_eq!(
        gaps,
        ["behavior 'pay' has no 'invariants' link to an invariant"],
        "{payload}"
    );

    // The trace tool reports the same one.
    let document = tool(&mut server, "specforge.trace", json!({"entity_id": "pay"}));
    let missing: Vec<&str> = document["missing"]
        .as_array()
        .unwrap()
        .iter()
        .map(|link| link["edge_label"].as_str().unwrap())
        .collect();
    assert_eq!(missing, ["invariants"], "{document}");
}
