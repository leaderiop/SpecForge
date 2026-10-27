//! A writing collector's edit plan (ADR 0289), applied after its report is read.

use serde_json::json;
use specforge_ops::asking::Asking;
use specforge_ops::testing::{ScriptedMachine, ScriptedRun};
use specforge_protocol_types::CollectOutput;
use specforge_test_macros::test as specforge_test;

use super::{Fixture, Runner};

#[specforge_test(
    behavior = "answer_edit_plan",
    verify = "a writing collector receives the graph and its plan is applied after the report"
)]
#[ignore = "K5 T9"]
fn a_collector_plan_creates_commits_and_records_results() {
    let mut plan = serde_json::to_value(CollectOutput::default()).unwrap();
    plan["plan"] = json!({"ops": [
        {"op": "create", "kind": "behavior", "id": "c", "title": "C", "revision": "absent"}
    ]});
    let fx = Fixture::new(vec![Runner::new("git").writing(plan)]);
    let machine = ScriptedMachine::new()
        .approving(fx.approval("git"))
        .answering(ScriptedRun {
            files: vec![("".into(), "{}".to_string())],
            exit_code: Some(0),
            ..ScriptedRun::default()
        });

    fx.run(Asking::Nobody, &machine).unwrap();

    let written: String = std::fs::read_dir(fx.root())
        .unwrap()
        .filter_map(|e| std::fs::read_to_string(e.unwrap().path()).ok())
        .collect();
    assert!(written.contains("behavior c \"C\""), "{written}");
}
