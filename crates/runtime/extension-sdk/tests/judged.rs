//! An SDK judged rule (ADR 0290) builds the descriptor the host reads: its outcomes, each target's inputs, the
//! field that exempts an entity and the owner state a target waits for (ADR 0362).
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_extension_sdk::prelude::*;

#[specforge_test_macros::test(requirement = "FEATURE-058")]
fn a_judged_rule_builds_the_descriptor_the_host_reads() {
    let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/judged", "1.0.0"));
    b.rule("W311", |r| {
        r.severity(ValidationSeverity::Warning)
            .message_template("the check of {id} does not hold")
            .judged("checker", |j| {
                j.outcomes(&["passed", "issues"])
                    .passing(&["passed"])
                    .target("plan", |t| {
                        t.input("plan", "self")
                            .input("decisions", "self.phase<decision.phase")
                            .unless_set(&["pattern"]);
                    })
                    .target("task", |t| {
                        t.input("task", "self").once_complete();
                    })
                    .target("decision", |t| {
                        t.input("decision", "self").while_open();
                    })
                    .values(&["phase_spec.scores"])
                    .outcome_fields(&["plan.verdict"]);
            });
    });
    let rule = &b.declaration().validation_rules[0];
    assert_eq!(rule.check, "judged");
    assert_eq!(
        serde_json::to_value(rule.judged.as_ref().unwrap()).unwrap(),
        serde_json::json!({
            "judge": "checker", "outcomes": ["passed", "issues"], "passing": ["passed"],
            "targets": [{"kind": "plan", "unless_set": ["pattern"], "inputs": [
                {"name": "plan", "selector": "self"},
                {"name": "decisions", "selector": "self.phase<decision.phase"},
            ]}, {"kind": "task", "judged_while": "complete", "inputs": [
                {"name": "task", "selector": "self"},
            ]}, {"kind": "decision", "judged_while": "pending", "inputs": [
                {"name": "decision", "selector": "self"},
            ]}],
            "values": ["phase_spec.scores"], "outcome_fields": ["plan.verdict"],
        })
    );
}
