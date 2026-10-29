//! The gate guards, black-box. A gate is evaluated only at a command's or prompt's entry and gsd has none until
//! r41/r42, so what shows here is what must hold without one: a guard never refuses a write (D-r39-12). The guard logic is
//! tested on pass inputs in `passes::gates::tests`; only `declaration_verify` runs them through the SDK's MockHost.

use crate::harness::project;

#[test]
fn a_write_that_opens_a_window_is_accepted() {
    let p = project("verify");
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    // The guards would refuse an open window, an unverified phase and a half-done audit at a gate.
    let before = p.codes();
    assert!(!before.contains(&"gsd/E512".to_string()), "{before:?}");
    // A NEW open window: were E512 judged on writes, the write would add an error the project did not have.
    let run = p.run(&[
        "new",
        "broken_window",
        "BW2",
        "Another TODO",
        "--in",
        "spec/ops/windows.spec",
        "--kind",
        "todo",
        "--phase",
        "v1.0/01",
        "--file",
        "src/tasks.js",
        "--description",
        "Another TODO remains",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of("BW2")["status"], "open");
    let after = p.codes();
    assert!(!after.contains(&"gsd/E512".to_string()), "{after:?}");
    assert!(
        after.iter().all(|c| before.contains(c)),
        "{before:?} -> {after:?}"
    );
}

#[test]
fn windows_enforce_is_off_under_compat_and_on_under_strict() {
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
