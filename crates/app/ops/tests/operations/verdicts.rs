//! `set_verdict`, an edit plan op checked through the judged-rules port (ADR 0289, D6). The host has no judged
//! rule until K7: every `set_verdict` is `not_judged`.

use serde_json::{Value, json};
use specforge_project::testing::{CORE, TestProject};
use specforge_test_macros::test as specforge_test;

use crate::plans::{project, refused};

fn verdict(rule: &str, outcome: &str, judge: &str, cited: &[&str], basis: Value) -> Value {
    json!([{"op": "set_verdict", "entity": "v1.0/03/U1", "rule": rule, "outcome": outcome,
        "judge": judge, "model": "m", "findings": [], "cited": cited, "basis": basis}])
}

#[ignore = "K5 T13"]
#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "set_verdict is refused without a judged rule"
)]
fn set_verdict_is_refused_without_a_judged_rule() {
    let p = project();
    let ops = verdict("quality", "pass", "judge", &[], json!({}));
    assert_eq!(
        refused(&p, CORE, "core", "record", ops).as_deref(),
        Some("not_judged")
    );
}

#[ignore = "K5 T13"]
#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "set_verdict checks the rule and its basis"
)]
fn set_verdict_checks_the_rule_and_its_basis() {
    // The fake rule `quality` targets `verification`, declares outcomes `pass` and `fail`, and belongs to
    // `@t/core`'s judge `judge`; T13 gives the host this port through `with_rules`.
    let p = project();
    let reasons = |ops: Value| with_rules(&p, "record", ops);
    assert_eq!(
        reasons(verdict("quality", "pass", "judge", &[], json!({}))),
        "kind"
    );
    assert_eq!(
        reasons(verdict("quality", "meh", "judge", &[], json!({}))),
        "outcome"
    );
    assert_eq!(
        reasons(verdict("quality", "pass", "other", &[], json!({}))),
        "judge"
    );
    assert_eq!(
        reasons(verdict("quality", "pass", "judge", &["nowhere"], json!({}))),
        "cite"
    );
    assert_eq!(
        reasons(verdict(
            "quality",
            "pass",
            "judge",
            &[],
            json!({"cited": "sha256:0"})
        )),
        "stale_revision"
    );
}

#[ignore = "K7"]
#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "set_verdict writes its verdict and its ledger line"
)]
fn set_verdict_writes_its_verdict_and_its_ledger_line() {
    let p = project();
    let ops = verdict("quality", "pass", "judge", &[], json!({}));
    assert_eq!(refused(&p, CORE, "core", "record", ops), None);
    assert!(
        std::fs::read_to_string(p.root().join("03-02.spec"))
            .unwrap()
            .contains("verdict")
    );
}

/// What T13 adds: the host with a fake `JudgedRules`, answering the refusal reason or code of `ops`.
fn with_rules(_project: &TestProject, _command: &str, _ops: Value) -> String {
    panic!("K5 T13 injects the fake JudgedRules through the apply step's port")
}
