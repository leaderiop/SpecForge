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
    assert_eq!(row.inputs.len(), 11, "{:?}", row.inputs);
    for code in &row.codes {
        assert_inputs_match_rules_md(code);
        let judged = judged_descriptor(code).expect("judged");
        assert_eq!(judged["judge"], "gsd-plan-checker", "{code}");
        assert_eq!(judged["outcomes"], serde_json::json!(["passed", "issues"]));
        assert_eq!(judged["passing"], serde_json::json!(["passed"]));
    }
}

/// E501 reads a truth's tests two ways (owner decision 04-U3, ADR 0331 D7): its `[gsd:<id>]` tags and its proof's
/// test facts; without `@code/core` the proof chain is inert.
#[test]
fn the_truth_verdict_reads_its_tags_and_its_proof() {
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
    assert_eq!(tests["selector"], "test(self, self.proof)");
}
