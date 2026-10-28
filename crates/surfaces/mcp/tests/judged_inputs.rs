//! `specforge.verdict_inputs` (ADR 0290): a judged rule's inputs for one entity, read-only, the answer of
//! `specforge query <id> --inputs <rule>`.

use crate::support::*;
use serde_json::json;
use specforge_test::prelude::*;

#[specforge_test(
    behavior = "answer_judged_inputs",
    verify = "the verdict inputs tool answers what the inputs query answers"
)]
fn the_verdict_inputs_tool_matches_the_cli() {
    let mut server = TestProject::new()
        .file(
            "a.spec",
            "behavior alpha \"Alpha\" {\n  contract \"c\"\n}\n",
        )
        .serve(&[TestExtension::software()]);
    let tools = call(&mut server, "tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"specforge.verdict_inputs"), "{names:?}");
    // No extension judges `nothing`: the refusal is the CLI's, `not_judged`.
    let reply = call_tool(
        &mut server,
        "specforge.verdict_inputs",
        json!({"entity": "alpha", "rule": "nothing"}),
    );
    assert!(tool_text(&reply).contains("not_judged"), "{reply}");
}
