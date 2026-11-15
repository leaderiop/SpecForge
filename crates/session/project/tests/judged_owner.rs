//! A judged target that waits for an owner state (ADR 0362): its entities are judged on every compile and the compute
//! passes read every judgement, and once the owner states are known on the merged snapshot an entity in the other state
//! owes no verdict: no finding, and no judgement for a check pass, a guard or an output.

use std::sync::Arc;

use specforge_extension_sdk::prelude::*;
use specforge_project::CompiledProject;
use specforge_project::judged::Cited;
use specforge_project::testing::TestProject;
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

use crate::judged::record_from;

const WAITS: &str = "@test/waits";

/// `@test/waits`: a `plan` complete on `status ∈ [complete]`, its `status` computed from `done` by the compute pass
/// `progress`; a `task` scoped by `plan`, judged by `W902` once its plan is complete; and the check pass `seen`.
fn waits() -> InProcessRuntime {
    InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(WAITS, "1.0.0"));
        c.kind("plan", |k| {
            k.complete_when("status", &["complete"])
                .field("done", |f| {
                    f.field_type(FieldType::Bool);
                })
                .field("status", |f| {
                    f.vocabulary(&Vocabulary::new(&["draft", "complete"]))
                        .computed();
                });
        });
        c.kind("task", |k| {
            k.scoped_by("plan").field("plan", |f| {
                f.field_type(FieldType::Reference).target_kind("plan");
            });
        });
        c.pass("progress", |p| {
            p.compute().computes(&["plan.status"]).run(progress);
        });
        c.pass("seen", |p| {
            p.phase("check")
                .run(|_: &PassInput| Vec::<PassDiagnostic>::new());
        });
        c.rule("W902", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("task '{id}' was not reviewed")
                .judged("reviewer", |j| {
                    j.outcomes(&["passed", "failed"])
                        .passing(&["passed"])
                        .target("task", |t| {
                            t.input("task", "self").once_complete();
                        });
                });
        });
        c.agent("reviewer", |a| {
            a.description("Judges a task");
        });
        c
    })
}

/// Each plan's `status`: `complete` once it is `done`, else `draft`.
fn progress(input: &PassInput) -> PassOutput {
    let values = input
        .entities
        .iter()
        .filter(|e| e.kind == "plan")
        .map(|e| PassValue {
            entity: e.id.clone(),
            field: "status".to_string(),
            value: match e.fields.get("done").map(String::as_str) {
                Some("true") => "complete".into(),
                _ => "draft".into(),
            },
        })
        .collect();
    PassOutput {
        values,
        ..PassOutput::default()
    }
}

/// Plan `p1` done with its task `t1`, and plan `p2`, done when `done`, with its task `t2`.
fn spec(done: bool) -> String {
    format!(
        "plan p1 \"P1\" {{\n  done true\n}}\n\nplan p2 \"P2\" {{\n  done {done}\n}}\n\n\
         task t1 \"T1\" {{\n  plan p1\n}}\n\ntask t2 \"T2\" {{\n  plan p2\n}}\n"
    )
}

fn project(spec: &str) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[WAITS])
        .installing_enabled()
        .file("a.spec", spec)
}

/// The rule's qualified code.
fn rule(compiled: &CompiledProject) -> String {
    let registries = compiled.environment().registries();
    let rule = registries.judged_rules().next().expect("the judged rule");
    rule.code.clone()
}

/// The ids of the entities the rule reports on, in report order.
fn reported(compiled: &CompiledProject) -> Vec<String> {
    let code = rule(compiled);
    compiled
        .diagnostics()
        .iter()
        .filter(|d| d.code == code)
        .map(|d| d.message.split('\'').nth(1).unwrap_or_default().to_string())
        .collect()
}

/// The ids of the tasks the last call of `export` was handed a judgement of the rule on.
fn handed(runtime: &InProcessRuntime, export: &str, code: &str) -> Vec<String> {
    let input = runtime
        .calls()
        .into_iter()
        .rev()
        .find(|c| c.export == export)
        .map(|c| c.input)
        .unwrap_or_else(|| panic!("{export} was never called"));
    input["entities"]
        .as_array()
        .expect("entities")
        .iter()
        .filter(|e| {
            e["judgements"]
                .as_array()
                .is_some_and(|all| all.iter().any(|j| j["rule"] == code))
        })
        .map(|e| e["id"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>()
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a judged target that waits for an owner state is owed a verdict only in it, decided after the compute passes"
)]
fn a_target_waiting_for_its_owner_is_judged_only_once_it_is_complete() {
    let runtime = Arc::new(waits());
    let compiled = project(&spec(false)).compile_in(Arc::clone(&runtime) as _);
    let code = rule(&compiled);
    assert_eq!(
        reported(&compiled),
        ["p1/t1"],
        "{:?}",
        compiled.diagnostics()
    );
    assert!(compiled.judgements().get("p1/t1", &code).is_some());
    assert!(compiled.judgements().get("p2/t2", &code).is_none());
    // the compute passes read every judgement; a check pass only the owed ones
    assert_eq!(
        handed(&runtime, "__pass_progress", &code),
        ["p1/t1", "p2/t2"]
    );
    assert_eq!(handed(&runtime, "__pass_seen", &code), ["p1/t1"]);
    let done = project(&spec(true)).compile_in(Arc::clone(&runtime) as _);
    assert_eq!(reported(&done), ["p1/t1", "p2/t2"]);
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a verdict recorded while its owner is pending is read once the owner is complete"
)]
fn a_verdict_recorded_early_counts_once_the_owner_is_complete() {
    let runtime: Arc<InProcessRuntime> = Arc::new(waits());
    let project = project(&spec(false));
    let compiled = project.compile_in(Arc::clone(&runtime) as _);
    let code = rule(&compiled);
    record_from(
        &project,
        &compiled,
        "p2/t2",
        &code,
        "failed",
        &Cited::default(),
        "",
    );
    let pending = project.compile_in(Arc::clone(&runtime) as _);
    assert_eq!(reported(&pending), ["p1/t1"], "{:?}", pending.diagnostics());
    assert!(pending.judgements().get("p2/t2", &code).is_none());
    project.write(
        "a.spec",
        project.read("a.spec").replace("done false", "done true"),
    );
    let complete = project.compile_in(Arc::clone(&runtime) as _);
    let judgement = complete.judgements().get("p2/t2", &code).expect("owed now");
    assert_eq!(
        (judgement.state.as_str(), judgement.outcome.as_deref()),
        ("fresh", Some("failed")),
        "{judgement:?}"
    );
    let message = complete
        .diagnostics()
        .iter()
        .find(|d| d.code == code && d.message.contains("'p2/t2'"))
        .map(|d| d.message.clone())
        .unwrap_or_default();
    assert!(message.contains("is 'failed'"), "{message}");
}
