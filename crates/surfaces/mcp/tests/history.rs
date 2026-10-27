//! MCP validate judges history per request (ADR 0287).

use std::sync::Arc;

use crate::support::*;
use serde_json::{Value, json};
use specforge_project::SharedRuntime;
use specforge_project::testing::{FakeHistory, GSD_EXTENSION, gsd_history_runtime, history};
use specforge_test_macros::test as specforge_test;

const BASE: &str = "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\n";

fn decision(origin: &str) -> String {
    format!("{BASE}decision D \"D\" {{\n  phase 03\n  origin {origin}\n}}\n")
}

fn codes(reply: &Value) -> Vec<String> {
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    serde_json::from_str::<Vec<Value>>(text)
        .unwrap()
        .iter()
        .filter_map(|d| d["code"].as_str().map(String::from))
        .collect()
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "validate since judges per request"
)]
fn validate_since_judges_per_request() {
    let runtime: SharedRuntime = Arc::new(gsd_history_runtime(json!([])));
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[GSD_EXTENSION])
        .file("a.spec", decision("project"));
    // HEAD holds what the tree holds; `main` held another origin.
    let mut fake = FakeHistory::repo(
        project.root(),
        &"a".repeat(40),
        vec![("a.spec".into(), decision("project").into_bytes())],
    );
    fake.named.push((
        "main".into(),
        specforge_project::history::Sha::new("b".repeat(40)),
        vec![("a.spec".into(), decision("context").into_bytes())],
    ));
    let _installed = history::install(Arc::new(fake));
    let mut server = Served::initialized_in(project, runtime);
    let plain = server.call_tool("specforge.validate", json!({}));
    assert!(!codes(&plain).contains(&"E084".to_string()), "{plain}");
    let since = server.call_tool("specforge.validate", json!({"since": "main"}));
    assert!(codes(&since).contains(&"E084".to_string()), "{since}");
    // the served session keeps HEAD
    let again = server.call_tool("specforge.validate", json!({}));
    assert!(!codes(&again).contains(&"E084".to_string()), "{again}");
    let unknown = server.call_tool("specforge.validate", json!({"since": "nosuch"}));
    assert_eq!(unknown["result"]["isError"], true, "{unknown}");
}
