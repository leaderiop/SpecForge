//! The gate guards, black-box. A gate is evaluated only at a command's or prompt's entry and gsd has none until
//! r41/r42, so what shows here is what must hold without one: a guard never refuses a write (D-r39-12).

use crate::harness::project;

#[test]
fn a_write_with_an_open_window_is_accepted() {
    let p = project("verify");
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    // BW1 is open, the phase is unverified and the audit has gaps: every gate guard has something to say at a gate.
    let before = p.read("spec/ops/windows.spec");
    let run = p.run(&["set", "BW1", "description", "A TODO stays in the store"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!run.all().contains("E080"), "{}", run.all());
    assert_ne!(p.read("spec/ops/windows.spec"), before);
    assert_eq!(p.fields_of("BW1")["status"], "open");
}

#[test]
fn e512_is_silent_under_compat() {
    let p = project("verify");
    let line = |p: &crate::harness::Project| {
        p.run(&["settings"])
            .all()
            .lines()
            .find(|l| l.contains("workflow.windows_enforce"))
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    // `workflow.windows_enforce` gates E512: off under gsd-compat, on under gsd-strict.
    assert!(
        line(&p).starts_with("workflow.windows_enforce = false"),
        "{}",
        line(&p)
    );
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    assert!(
        line(&p).starts_with("workflow.windows_enforce = true"),
        "{}",
        line(&p)
    );
}
