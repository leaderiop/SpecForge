//! An analyze pass that declares the `delta` input is sent the delta of the compile it analyzes (ADR 0376 D6); one
//! that does not is sent no `delta` key.

use std::sync::Arc;

use serde_json::json;
use specforge_extension_sdk::prelude::*;
use specforge_ops::analysis::analyze::AnalyzeOptions;
use specforge_project::testing::{TestProject, pass_input};
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

use crate::compiled_support::analyzed;

const EXT: &str = "@test/delta";

fn nothing(_: &PassInput) -> Vec<PassDiagnostic> {
    Vec::new()
}

/// `@test/delta`: the kinds `item` and `note`, the analyze pass `changed` reading the delta of `item`, and `plain`.
fn delta_extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.kind("item", |_| {});
    c.kind("note", |_| {});
    c.pass("changed", |p| {
        p.phase("analyze")
            .input(Input::delta(&["item"]))
            .run(nothing);
    });
    c.pass("plain", |p| {
        p.phase("analyze").run(nothing);
    });
    c
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_analyze_pass_is_sent_the_delta_of_the_compile_it_analyzes() {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", "item a \"A\" {\n}\n\nnote n \"N\" {\n}\n");
    let runtime = Arc::new(InProcessRuntime::new().with(delta_extension));
    let compiled = project.compile_in(runtime.clone());
    analyzed(&compiled, &AnalyzeOptions::default());
    let delta = &pass_input(&runtime, "changed")["delta"];
    let held = compiled
        .delta()
        .expect("an analyze pass declares the delta");
    assert_eq!(
        *delta,
        serde_json::to_value(held.restricted_to(&["item"])).unwrap()
    );
    assert_eq!(delta["base"], json!({"kind": "none"}));
    assert_eq!(delta["nodes"].as_array().unwrap().len(), 1, "{delta}");
    assert_eq!(delta["nodes"][0]["id"], "a");
    assert!(pass_input(&runtime, "plain").get("delta").is_none());
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_analyze_pass_after_an_update_that_skipped_the_checks_is_sent_the_delta_since_the_last_check()
{
    const SPEC: &str = "item a \"A\" {\n}\n\nitem b \"B\" {\n}\n";
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", SPEC);
    let runtime = Arc::new(InProcessRuntime::new().with(delta_extension));
    let mut session = project.session_in(runtime.clone());
    // typing: b retitled with a parse error after it, so the checks are skipped
    let typing = format!("{}item c {{\n", SPEC.replace("\"B\"", "\"Bee\""));
    let buffer = specforge_project::Buffer::new(project.root().join("a.spec"), typing);
    session.hold_with(
        &[buffer],
        specforge_project::CheckMode::SyntaxOnlyIfParseErrors,
    );
    assert!(
        session.project().delta().is_none(),
        "the checks were skipped"
    );
    analyzed(session.project(), &AnalyzeOptions::default());
    let delta = &pass_input(&runtime, "changed")["delta"];
    assert_eq!(delta["base"], json!({"kind": "previous"}), "{delta}");
    let changed: Vec<(&str, &str)> = delta["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["id"].as_str().unwrap(), n["change"].as_str().unwrap()))
        .collect();
    // the half-typed `c` is in the graph the analysis reads
    assert_eq!(changed, [("b", "modified"), ("c", "added")], "{delta}");
}
