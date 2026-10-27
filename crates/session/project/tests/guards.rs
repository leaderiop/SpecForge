//! A guard pass (ADR 0289) runs only when a write is judged: it reads the project before the write as `previous`
//! and refuses the write through E080. The same text written by hand compiles with no guard finding.

use std::sync::Arc;

use serde_json::json;
use specforge_project::testing::{GSD_WRITES_EXTENSIONS, GsdWrites, TestProject};
use specforge_test::prelude::*;

const SPEC: &str = "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n";

#[ignore = "K5 T12"]
#[specforge_test(
    behavior = "run_guard_passes",
    verify = "a guard pass refuses a write and is silent on compile"
)]
fn a_guard_pass_refuses_a_write_and_is_silent_on_compile() {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file("a.spec", SPEC);
    // The guard answers one error whatever it is given: it runs only when a write is judged.
    let finding = json!([{"code": "E700", "severity": "Error", "message": "phase 03 is not complete", "entity": "v1.0/03"}]);
    let runtime = Arc::new(GsdWrites::new().guarding(finding).runtime());
    let compiled = project.compile_in(runtime.clone());
    assert!(
        compiled
            .diagnostics()
            .iter()
            .all(|d| !d.code.ends_with("E700"))
    );
    assert!(
        runtime
            .calls()
            .iter()
            .all(|c| c.export != "__pass_phase_complete"),
        "a compile never runs a guard pass"
    );
    let edited = compiled.edited(&[("a.spec".to_string(), SPEC.replace("Three", "Tres"))].into());
    let guarded: Vec<_> = edited
        .diagnostics()
        .into_iter()
        .filter(|d| d.code.ends_with("E700"))
        .collect();
    assert_eq!(guarded.len(), 1, "{guarded:?}");
}
