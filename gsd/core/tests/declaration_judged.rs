//! The judged rules `@gsd/core` declares (`spec/rules.md`, "Judged rules' inputs"), pinned through the SDK's MockHost.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{assert_inputs_match_rules_md, judged_descriptor, judged_rows};

#[test]
fn plan_checker_inputs_match_rules_md() {
    let row = judged_rows()
        .into_iter()
        .find(|r| r.codes.iter().any(|c| c == "W311"))
        .expect("the plan check row");
    assert_eq!(row.codes, ["W311", "W313", "W315", "W316"]);
    assert_eq!(row.inputs.len(), 14, "{:?}", row.inputs);
    for code in &row.codes {
        assert_inputs_match_rules_md(code);
        let judged = judged_descriptor(code).expect("judged");
        assert_eq!(judged["judge"], "gsd-plan-checker", "{code}");
        assert_eq!(judged["outcomes"], serde_json::json!(["passed", "issues"]));
        assert_eq!(judged["passing"], serde_json::json!(["passed"]));
    }
}

/// The verifier judges a prohibition that names no enforcing test, once its plan is complete; its verdict holds or not.
#[test]
fn a_judgment_tier_prohibition_is_judged_by_the_verifier_w525() {
    assert_inputs_match_rules_md("W525");
    let judged = judged_descriptor("W525").expect("judged");
    assert_eq!(judged["judge"], "gsd-verifier");
    assert_eq!(
        judged["outcomes"],
        serde_json::json!(["held", "violated", "unverified"])
    );
    assert_eq!(judged["passing"], serde_json::json!(["held"]));
}

/// Every judged rule's declared targets read exactly what `rules.md` lists for it.
#[test]
fn every_judged_rule_reads_what_rules_md_lists() {
    for rule in gsd_core::judged::JUDGED {
        assert_inputs_match_rules_md(rule.code);
    }
}

/// E501 reads a truth's tests one way (ADR 0331 D7, amended by ADR 0371 D5): the test facts that name it (a
/// `[gsd:<id>]` tag in a test's name, `@code/core`'s `test.names`); without `@code/core` the chain is inert.
#[test]
fn the_truth_verdict_reads_the_tests_that_name_it() {
    assert_inputs_match_rules_md("E501");
    let judged = judged_descriptor("E501").expect("judged");
    let truth = judged["targets"]
        .as_array()
        .and_then(|targets| targets.iter().find(|t| t["kind"] == "truth"))
        .expect("the truth target");
    let tests = truth["inputs"]
        .as_array()
        .and_then(|inputs| inputs.iter().find(|i| i["name"] == "tests"))
        .expect("a tests input");
    assert_eq!(tests["selector"], "test(self<test.names)");
}
