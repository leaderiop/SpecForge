//! `@code/core` is an optional peer of `@gsd/core`: its test facts are what E501's truth input `tests` selects
//! (`test(self<test.names)`, the facts that name the truth) and what the passes that read the thread declare. No
//! authored field names a test (ADR 0371 D5), so a project checks the same without the peer and with it: neither way is
//! a W021 or an I004.

use super::harness::{Project, project};

/// Every diagnostic of `specforge check --format json`.
fn diagnostics(p: &Project) -> Vec<serde_json::Value> {
    let run = p.run(&["check", "--format", "json"]);
    let found: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    found.as_array().expect("a list").clone()
}

fn coded<'a>(found: &'a [serde_json::Value], code: &str) -> Vec<&'a serde_json::Value> {
    found.iter().filter(|d| d["code"] == code).collect()
}

#[test]
fn a_project_without_code_core_has_no_w021() {
    let found = diagnostics(&project("execute"));
    assert!(coded(&found, "W021").is_empty(), "{found:#?}");
    assert!(coded(&found, "I004").is_empty(), "{found:#?}");
}

#[test]
fn with_code_core_a_project_has_no_w021() {
    let found = diagnostics(&project("execute").with_builtins(&["@code/core"]));
    assert!(coded(&found, "W021").is_empty(), "{found:#?}");
    assert!(coded(&found, "I004").is_empty(), "{found:#?}");
}
