//! `specforge.inspect` of a collected fact names where it is written (ADR 0346 D3): its `located` places, beside the
//! store span that identifies it.

use std::sync::Arc;

use serde_json::json;
use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, stored};
use specforge_test::prelude::*;

use crate::support::*;

const KEY: &str = "0123456789abcdef0123456789abcdef01234567";

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "inspect of a fact names where it is written"
)]
fn inspect_of_a_fact_names_where_it_is_written() {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&COLLECTED_EXTENSIONS)
        .installing_enabled()
        .file("truths.spec", "truth T1 \"One\" {\n}\n");
    let place =
        json!({"file": "a.test.js", "start_line": 4, "start_col": 1, "end_line": 4, "end_col": 9});
    stored(
        project.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": "js:a.test.js#adds", "at": [place], "fields": {"file": "a.test.js"}}]),
    );
    let mut served = project.serve_in(Arc::new(Collected::new().runtime()));

    let reply = tool_json(&call_tool(
        &mut served,
        "specforge.inspect",
        json!({"entity_id": "js:a.test.js#adds"}),
    ));

    assert_eq!(reply["located"], json!([place]), "{reply}");
    assert_eq!(
        reply["file"],
        format!(".specforge/facts/{KEY}/node.json"),
        "{reply}"
    );
    let authored = tool_json(&call_tool(
        &mut served,
        "specforge.inspect",
        json!({"entity_id": "T1"}),
    ));
    assert!(authored.get("located").is_none(), "{authored}");
}
