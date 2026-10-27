//! A command or collector may write (ADR 0289): it declares `writes`, answers an edit plan, and the plan's ops
//! carry the revisions the call received. A kind and a field may be human-only; a pass may be a guard.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::json;
use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::testing::{MockHost, call_every_command};
use specforge_extension_sdk::{PlanOp, Verdict};
use specforge_test_macros::test as specforge_test;

fn writer() -> ContributionsBuilder {
    let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/w", "1.0.0"));
    b.kind("task", |k| {
        k.field("status", |f| {
            f.field_type(FieldType::String);
        });
    });
    b.kind("override", |k| {
        k.human().field("reason", |f| {
            f.field_type(FieldType::String).human();
        });
    });
    b.command("done", |c| {
        c.title("Done")
            .description("Mark a task done")
            .writes()
            .handler(|call| {
                let mut plan = call.plan();
                plan.title("mark t1 done")
                    .set("t1", "status", "done")
                    .create(
                        "task",
                        "t2",
                        "Two",
                        "new.spec",
                        Default::default(),
                        Vec::new(),
                    );
                plan.set("t2", "status", "todo");
                CommandOutput::ok("ok\n").with_plan(plan.finish())
            });
    });
    b.pass("gate", |p| {
        p.guard().run(|_: &PassInput| Vec::<PassDiagnostic>::new());
    });
    b.collector("record", |c| {
        c.writes()
            .collect(|_: &CollectInput| Ok(CollectOutput::default()));
    });
    b
}

fn graph() -> CommandGraph {
    CommandGraph::new(
        vec![GraphNode {
            id: "t1".into(),
            kind: "task".into(),
            file: Some("main.spec".into()),
            ..Default::default()
        }],
        Vec::new(),
    )
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a writing declaration reaches the wire and its plan carries the call's revisions"
)]
fn a_writing_command_declares_writes_and_names_the_revisions_of_the_call() {
    let host = MockHost::new(writer());
    let declaration = host.declaration();
    let command = &declaration.surfaces.commands[0];
    assert!(command.writes);
    assert!(declaration.collectors[0].writes);
    assert_eq!(declaration.passes[0].phase.as_deref(), Some("guard"));
    let task = declaration
        .entities
        .iter()
        .find(|k| k.name == "override")
        .unwrap();
    assert_eq!(task.writer.as_deref(), Some("human"));
    assert_eq!(task.fields[0].writer.as_deref(), Some("human"));
    let wire = serde_json::to_value(command).unwrap();
    assert_eq!(wire["writes"], true);
    let read_only = ContributionsBuilder::new(ExtensionMeta::new("@acme/r", "1.0.0"));
    assert!(
        serde_json::to_value(&read_only.declaration().surfaces).unwrap()["commands"]
            .as_array()
            .is_none_or(|c| c.iter().all(|c| c.get("writes").is_none()))
    );

    let outputs = call_every_command(&writer(), &graph(), "2026-10-27", |_, _| String::new());
    let plan = outputs[0].1.plan.as_ref().unwrap();
    assert_eq!(plan.title.as_deref(), Some("mark t1 done"));
    let revisions: Vec<&str> = plan
        .ops
        .iter()
        .map(|op| match op {
            PlanOp::Set { revision, .. } | PlanOp::Create { revision, .. } => revision.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    // t1 is in main.spec the call holds; new.spec it does not (`absent`); t2 is created by this plan.
    assert_eq!(revisions, ["sha256:test", "absent", "absent"]);
    assert_eq!(
        serde_json::to_value(&outputs[0].1).unwrap()["plan"]["ops"][0]["op"],
        "set"
    );
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a writing declaration reaches the wire and its plan carries the call's revisions"
)]
fn a_writing_command_may_not_take_the_hosts_dry_run() {
    let result = std::panic::catch_unwind(|| {
        let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/w", "1.0.0"));
        b.command("c", |c| {
            c.arg("dry_run", |a| {
                a.flag();
            })
            .writes()
            .handler(|_| CommandOutput::ok(""));
        });
    });
    assert!(result.is_err());
    // A read-only command may take it: its declaration means what it always did.
    let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/r", "1.0.0"));
    b.command("c", |c| {
        c.arg("dry_run", |a| {
            a.flag();
        })
        .handler(|_| CommandOutput::ok(""));
    });
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a writing declaration reaches the wire and its plan carries the call's revisions"
)]
fn a_verdict_op_has_no_file_revision() {
    let mut plan = PlanBuilder::new(Default::default(), Default::default());
    plan.set_verdict(Verdict {
        entity: "t".into(),
        rule: "r".into(),
        outcome: "pass".into(),
        ..Default::default()
    });
    let value = serde_json::to_value(plan.finish()).unwrap();
    assert_eq!(value["ops"][0]["op"], "set_verdict");
    assert!(value["ops"][0].get("revision").is_none());
    assert_eq!(value["ops"][0]["cited"], json!([]));
}
