//! The spec guard reads a patch-text tool (Codex `apply_patch`, Cursor `ApplyPatch`) like a Write or an Edit
//! (review K9 M3).

use serde_json::json;
use specforge_project::testing::GsdHosts;
use specforge_test_macros::test as specforge_test;

use crate::host_support::{hosted, hosts_project, run_core_hook};

fn patch(hosted: &crate::host_support::Hosted, text: &str) -> serde_json::Value {
    let _ = hosted;
    json!({"event": "pre_tool", "host": "codex", "session": "s1",
           "tool": {"name": "apply_patch", "class": "edit", "input": {"input": text}}})
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "the spec guard judges the spec files a patch-text tool leaves and ignores a patch of other files"
)]
fn the_spec_guard_judges_a_patch() {
    let project = hosts_project(json!({}), json!({}));
    project.write("v.spec", "verification V1 \"One\" {\n  phase 03\n}\n");
    let hosted = hosted(project, GsdHosts::new());

    let update = "*** Begin Patch\n*** Update File: v.spec\n@@\n   phase 03\n+  status \"passed\"\n*** End Patch\n";
    let run = run_core_hook(&hosted, "spec_guard", &patch(&hosted, update));
    assert_eq!(run.answer["decision"], "deny", "{run:?}");
    assert!(run.answer["reason"].as_str().unwrap().contains("status"));

    let add = "*** Begin Patch\n*** Add File: o.spec\n+override O1 \"Waive\" {\n+  by \"agent\"\n+  phase 03\n+}\n*** End Patch\n";
    let run = run_core_hook(&hosted, "spec_guard", &patch(&hosted, add));
    assert_eq!(run.answer["decision"], "deny", "{run:?}");

    let prose = "*** Begin Patch\n*** Update File: README.md\n@@\n-a\n+b\n*** End Patch\n";
    let run = run_core_hook(&hosted, "spec_guard", &patch(&hosted, prose));
    assert!(
        run.exports.is_empty() && run.answer["decision"].is_null(),
        "{run:?}"
    );
}
