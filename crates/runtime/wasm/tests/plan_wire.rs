//! An edit plan is on the wire (ADR 0289, protocol 1.9.0): a writing command's input carries the revisions of the
//! sources, its answer carries the plan, and every op reads back as it was written.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use specforge_protocol_types::{
    Cite, CommandInput, CommandOutput, EditPlan, PlanObligation, PlanOp, RawGraph,
};

fn wire(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/wire")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[specforge_test_macros::test(
    behavior = "apply_edit_plan",
    verify = "an edit plan and a writing command's input read back as they were written"
)]
fn a_writing_commands_input_and_answer_are_the_goldens() {
    let golden = wire("command.writes.input.json");
    let input = CommandInput {
        args: golden["args"].as_object().unwrap().clone(),
        cwd: golden["cwd"].as_str().unwrap().to_string(),
        format: serde_json::from_value(golden["format"].clone()).unwrap(),
        today: golden["today"].as_str().unwrap().to_string(),
        now: golden["now"].as_str().unwrap().to_string(),
        graph: RawGraph::new(golden["graph"].to_string()).unwrap(),
        revisions: serde_json::from_value(golden["revisions"].clone()).unwrap(),
        ..Default::default()
    };
    assert_eq!(serde_json::to_value(&input).unwrap(), golden);

    let answer = wire("command.plan.output.json");
    let output: CommandOutput = serde_json::from_value(answer.clone()).unwrap();
    assert_eq!(output.plan.as_ref().unwrap().ops.len(), 2);
    assert_eq!(serde_json::to_value(&output).unwrap(), answer);
    let plain = serde_json::to_value(CommandOutput::ok("x")).unwrap();
    assert!(plain.get("plan").is_none(), "{plain}");
}

#[specforge_test_macros::test(
    behavior = "apply_edit_plan",
    verify = "an edit plan and a writing command's input read back as they were written"
)]
fn every_plan_op_reads_back_as_written() {
    let rev = "sha256:ab".to_string();
    let ops = vec![
        PlanOp::Create {
            kind: "task".into(),
            id: "t".into(),
            title: "T".into(),
            file: "a.spec".into(),
            revision: "absent".into(),
            fields: BTreeMap::from([("status".to_string(), json!("todo"))]),
            verify: vec![PlanObligation {
                kind: Some("unit".into()),
                text: "t works".into(),
            }],
        },
        PlanOp::Set {
            entity: "t".into(),
            field: "status".into(),
            value: json!("done"),
            revision: rev.clone(),
        },
        PlanOp::Unset {
            entity: "t".into(),
            field: "status".into(),
            revision: rev.clone(),
        },
        PlanOp::Append {
            entity: "t".into(),
            field: "tags".into(),
            value: json!("x"),
            revision: rev.clone(),
        },
        PlanOp::Remove {
            entity: "t".into(),
            field: "tags".into(),
            index: 1,
            revision: rev.clone(),
        },
        PlanOp::Link {
            entity: "t".into(),
            field: "needs".into(),
            target: "u".into(),
            revision: rev.clone(),
        },
        PlanOp::Unlink {
            entity: "t".into(),
            field: "needs".into(),
            target: "u".into(),
            revision: rev.clone(),
        },
        PlanOp::Retitle {
            entity: "t".into(),
            title: "New".into(),
            revision: rev.clone(),
        },
        PlanOp::AddVerify {
            entity: "t".into(),
            kind: None,
            text: "a".into(),
            revision: rev.clone(),
        },
        PlanOp::RemoveVerify {
            entity: "t".into(),
            kind: Some("unit".into()),
            text: "a".into(),
            revision: rev.clone(),
        },
        PlanOp::Delete {
            entity: "t".into(),
            revision: rev.clone(),
            unlink_references: true,
        },
        PlanOp::Rename {
            entity: "t".into(),
            to: "u".into(),
            revision: rev,
        },
        PlanOp::SetVerdict {
            entity: "t".into(),
            rule: "r".into(),
            outcome: "pass".into(),
            judge: "j".into(),
            model: "m".into(),
            findings: vec![json!({"note": "ok"})],
            cited: vec![
                Cite::Entity { entity: "t".into() },
                Cite::File {
                    file: "a.rs".into(),
                },
                Cite::Test {
                    test: "a_works".into(),
                },
            ],
            values: BTreeMap::from([("score".to_string(), json!(3))]),
            basis: BTreeMap::from([("a.spec".to_string(), "sha256:cd".to_string())]),
        },
    ];
    let plan = EditPlan {
        ops,
        title: Some("all".into()),
    };
    let read: EditPlan = serde_json::from_value(serde_json::to_value(&plan).unwrap()).unwrap();
    assert_eq!(read, plan);

    // An op with no revision decodes (the host refuses it `invalid_plan`, naming the op).
    let missing: PlanOp =
        serde_json::from_value(json!({"op": "unset", "entity": "t", "field": "f"})).unwrap();
    assert!(matches!(missing, PlanOp::Unset { revision, .. } if revision.is_empty()));
}

#[specforge_test_macros::test(
    behavior = "apply_edit_plan",
    verify = "a cite naming more than one thing is not a cite"
)]
fn a_cite_naming_more_than_one_thing_is_not_a_cite() {
    for ok in [
        json!({"entity": "a"}),
        json!({"file": "b"}),
        json!({"test": "c"}),
    ] {
        serde_json::from_value::<Cite>(ok.clone()).unwrap_or_else(|e| panic!("{ok}: {e}"));
    }
    for bad in [
        json!({"entity": "a", "file": "b"}),
        json!({"file": "b", "test": "c"}),
        json!({"entity": "a", "extra": "b"}),
        json!({}),
    ] {
        assert!(
            serde_json::from_value::<Cite>(bad.clone()).is_err(),
            "{bad}"
        );
    }
}
