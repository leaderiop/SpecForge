//! The core hook `core.spec_guard` (ADR 0292): a host's raw write of a `.spec` file is judged as a hand edit, and
//! one that sets a computed, harness or human-only value is denied, naming the tool that writes it.

use serde_json::{Value, json};
use specforge_project::testing::GsdHosts;
use specforge_test_macros::test as specforge_test;

use crate::host_support::{Hosted, hosted, hosts_project, run_core_hook};

const VERIFIED: &str = "verification V1 \"One\" {\n  phase 03\n  status \"passed\"\n}\n";

fn edit(hosted: &Hosted, file: &str, old: &str, new: &str) -> Value {
    let root = hosted.project.root().display().to_string();
    json!({"event": "pre_tool", "host": "claude-code", "session": "s1",
           "tool": {"name": "Edit", "class": "edit",
                    "input": {"file_path": format!("{root}/{file}"), "old_string": old, "new_string": new}}})
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "the spec guard denies a raw write of a computed value, naming the field and the tool that writes it"
)]
fn the_spec_guard_denies_a_raw_write_of_a_computed_value() {
    let project = hosts_project(json!({}), json!({}));
    project.write("v.spec", "verification V1 \"One\" {\n  phase 03\n}\n");
    let hosted = hosted(project, GsdHosts::new());
    let run = run_core_hook(
        &hosted,
        "spec_guard",
        &edit(
            &hosted,
            "v.spec",
            "phase 03\n",
            "phase 03\n  status \"passed\"\n",
        ),
    );
    assert_eq!(run.answer["decision"], "deny", "{run:?}");
    let reason = run.answer["reason"].as_str().unwrap();
    assert!(
        reason.contains("status") && reason.contains("computed") && reason.contains("V1"),
        "{reason}"
    );
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "the spec guard denies a raw write of a human-only value"
)]
fn the_spec_guard_denies_a_raw_write_of_a_human_only_value() {
    let hosted = hosted(hosts_project(json!({}), json!({})), GsdHosts::new());
    let root = hosted.project.root().display().to_string();
    let write = json!({"event": "pre_tool", "host": "claude-code", "session": "s1",
        "tool": {"name": "Write", "class": "write",
                 "input": {"file_path": format!("{root}/o.spec"),
                           "content": "override O1 \"Waive\" {\n  by \"agent\"\n  phase 03\n}\n"}}});
    let run = run_core_hook(&hosted, "spec_guard", &write);
    assert_eq!(run.answer["decision"], "deny", "{run:?}");
    assert!(
        run.answer["reason"]
            .as_str()
            .unwrap()
            .contains("written by a human"),
        "{run:?}"
    );
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "the spec guard allows prose and reports the errors an edit adds as context"
)]
fn the_spec_guard_allows_prose_and_reports_new_errors_as_context() {
    let hosted = hosted(hosts_project(json!({}), json!({})), GsdHosts::new());
    let prose = run_core_hook(
        &hosted,
        "spec_guard",
        &edit(&hosted, "m.spec", "goal \"Ship\"", "goal \"Ship it\""),
    );
    assert!(
        prose.answer["decision"].is_null() || prose.answer["decision"] == "allow",
        "{prose:?}"
    );
    assert!(prose.answer["context"].is_null(), "{prose:?}");

    // A reference to a phase that does not exist: an error, but not one the guard denies.
    let broken = run_core_hook(
        &hosted,
        "spec_guard",
        &edit(
            &hosted,
            "m.spec",
            "milestone v1.0\n  goal",
            "milestone v9.9\n  goal",
        ),
    );
    assert!(
        broken.answer["decision"].is_null() || broken.answer["decision"] == "allow",
        "{broken:?}"
    );
    assert!(
        broken.answer["context"].as_str().unwrap().contains("v9.9"),
        "{broken:?}"
    );

    // A path that is no spec file is not judged and not loaded.
    let other = run_core_hook(&hosted, "spec_guard", &edit(&hosted, "README.md", "a", "b"));
    assert!(
        other.exports.is_empty() && other.answer["decision"].is_null(),
        "{other:?}"
    );
    let _ = VERIFIED;
}
