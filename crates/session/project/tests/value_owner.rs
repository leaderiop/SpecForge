//! A field's checks and its holder's owner state (ADR 0350): a planned value waits until its owner is complete, a
//! while-open value is checked only while its owner is pending, both decided on the merged snapshot after the compute
//! passes.

use std::sync::Arc;

use specforge_common::Diagnostic;
use specforge_extension_sdk::prelude::*;
use specforge_project::CompiledProject;
use specforge_project::testing::TestProject;
use specforge_project::values::{CheckStatus, ValueBudget, ValueRow};
use specforge_registry::PlainDisk;
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

const OWNER: &str = "@test/owner";

/// `@test/owner`: a `plan` complete on `status ∈ [complete]`, its `status` computed from `done` by the compute pass
/// `progress`; its `files` each a file under the project root once it is complete, its `gone` a file while it is
/// pending, its `refs` a file always, and its `notes` read by the checker `named` once it is complete. A `task` scoped
/// by `plan`, declaring no `complete_when`, whose `files` wait for its plan.
fn owner() -> InProcessRuntime {
    InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(OWNER, "1.0.0"));
        c.peer("@specforge/checks", "^1.0");
        c.checker("named", |k| {
            k.applies_to(&["string", "string_list"])
                .plan(|_| PlannedItem::findings(Vec::new()));
        });
        c.kind("plan", |k| {
            k.complete_when("status", &["complete"])
                .field("done", |f| {
                    f.field_type(FieldType::Bool);
                })
                .field("status", |f| {
                    f.vocabulary(&Vocabulary::new(&["draft", "complete"]))
                        .computed();
                })
                .field("files", |f| {
                    f.field_type(FieldType::StringList)
                        .file(Root::Project)
                        .planned();
                })
                .field("gone", |f| {
                    f.field_type(FieldType::StringList)
                        .file(Root::Project)
                        .while_open();
                })
                .field("refs", |f| {
                    f.field_type(FieldType::StringList).file(Root::Project);
                })
                .field("notes", |f| {
                    f.field_type(FieldType::StringList).check("named").planned();
                });
        });
        c.kind("task", |k| {
            k.scoped_by("plan")
                .field("plan", |f| {
                    f.field_type(FieldType::Reference).target_kind("plan");
                })
                .field("files", |f| {
                    f.field_type(FieldType::StringList)
                        .file(Root::Project)
                        .planned();
                });
        });
        c.pass("progress", |p| {
            p.compute().computes(&["plan.status"]).run(progress);
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

fn project(spec: &str) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[OWNER, "@specforge/checks"])
        .installing_enabled()
        .file("a.spec", spec)
}

/// `@test/owner` with `@specforge/checks` from the builtins' component beside it.
fn runtime() -> Arc<InProcessRuntime> {
    Arc::new(owner().beside(specforge_project::testing::component_runtime()))
}

/// `spec` compiled with [`runtime`].
fn compile(spec: &str) -> CompiledProject {
    project(spec).compile_in(runtime())
}

fn coded(project: &CompiledProject, code: &str) -> Vec<Diagnostic> {
    project
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == code)
        .collect()
}

/// The status of each of `id`'s rows of `field`, in order.
fn statuses(project: &CompiledProject, id: &str, field: &str) -> Vec<CheckStatus> {
    project
        .value_checks()
        .of(id)
        .iter()
        .filter(|row: &&ValueRow| row.field == field)
        .map(|row| row.status)
        .collect()
}

/// A plan, done or not, naming `files`.
fn plan(done: bool, field: &str, files: &[&str]) -> String {
    let items: Vec<String> = files.iter().map(|f| format!("\"{f}\"")).collect();
    format!(
        "plan p1 \"P1\" {{\n  done {done}\n  {field} [{}]\n}}\n",
        items.join(", ")
    )
}

#[specforge_test(
    behavior = "check_values_by_owner_state",
    verify = "a planned value's checks wait while its owner is pending, and its row says planned"
)]
fn a_planned_file_waits_for_its_owner() {
    let project = compile(&plan(false, "files", &["new.js"]));
    assert!(
        coded(&project, "E016").is_empty(),
        "{:?}",
        project.diagnostics()
    );
    assert_eq!(statuses(&project, "p1", "files"), [CheckStatus::Planned]);
    let row = &project.value_checks().of("p1")[0];
    assert_eq!(
        (row.outcome_class, &row.digest, row.codes.len()),
        (None, &None, 0)
    );
}

#[specforge_test(
    behavior = "check_values_by_owner_state",
    verify = "a planned value is checked once its owner is complete, its computed state deciding"
)]
fn a_complete_owners_planned_file_is_e016() {
    let project = compile(&plan(true, "files", &["new.js"]));
    let found = coded(&project, "E016");
    assert_eq!(found.len(), 1, "{:?}", project.diagnostics());
    let span = found[0].span.as_ref().expect("on the value");
    assert_eq!((span.start_line, span.start_col), (3, 11));
    assert_eq!(statuses(&project, "p1", "files"), [CheckStatus::Failed]);
}

#[specforge_test(
    behavior = "check_values_by_owner_state",
    verify = "a kind with no complete_when follows its nearest scope ancestor's owner state"
)]
fn a_scope_ancestors_owner_state_decides() {
    let spec = |done: bool| {
        format!(
            "plan p1 \"P1\" {{\n  done {done}\n}}\n\ntask t1 \"T1\" {{\n  plan p1\n  files [\"new.js\"]\n}}\n"
        )
    };
    let pending = compile(&spec(false));
    assert!(
        coded(&pending, "E016").is_empty(),
        "{:?}",
        pending.diagnostics()
    );
    assert_eq!(statuses(&pending, "p1/t1", "files"), [CheckStatus::Planned]);
    let complete = compile(&spec(true));
    assert_eq!(
        coded(&complete, "E016").len(),
        1,
        "{:?}",
        complete.diagnostics()
    );
    assert_eq!(statuses(&complete, "p1/t1", "files"), [CheckStatus::Failed]);
}

#[specforge_test(
    behavior = "check_values_by_owner_state",
    verify = "a value its owner holds back calls no checker"
)]
fn a_pending_owner_calls_no_checker() {
    let runtime = runtime();
    let compiled = project(&plan(false, "notes", &["one", "two"])).compile_in(runtime.clone());
    let calls = runtime
        .calls()
        .iter()
        .filter(|call| call.export == "__check_values")
        .count();
    assert_eq!(calls, 0);
    assert_eq!(
        statuses(&compiled, "p1", "notes"),
        [CheckStatus::Planned, CheckStatus::Planned]
    );

    let runtime = self::runtime();
    project(&plan(true, "notes", &["one"])).compile_in(runtime.clone());
    let calls = runtime
        .calls()
        .iter()
        .filter(|call| call.export == "__check_values")
        .count();
    assert_eq!(calls, 1, "the second stage asks once its owner is complete");
}

#[specforge_test(
    behavior = "check_values_by_owner_state",
    verify = "a while-open value is checked while its owner is pending, and its row says closed once it is complete"
)]
fn a_while_open_value_is_checked_only_while_its_owner_is_pending() {
    let open = compile(&plan(false, "gone", &["old.js"]));
    assert_eq!(coded(&open, "E016").len(), 1, "{:?}", open.diagnostics());
    assert_eq!(statuses(&open, "p1", "gone"), [CheckStatus::Failed]);
    let closed = compile(&plan(true, "gone", &["old.js"]));
    assert!(
        coded(&closed, "E016").is_empty(),
        "{:?}",
        closed.diagnostics()
    );
    assert_eq!(statuses(&closed, "p1", "gone"), [CheckStatus::Closed]);
    assert!(
        coded(&closed, "I213").is_empty(),
        "a closed value waits for nothing"
    );
}

#[specforge_test(
    behavior = "check_values_by_owner_state",
    verify = "a gated value is no judged input, waiting or checked"
)]
fn a_planned_value_is_no_judged_input() {
    let spec = |done: bool| {
        format!(
            "plan p1 \"P1\" {{\n  done {done}\n  files [\"new.js\"]\n  refs [\"here.md\"]\n}}\n"
        )
    };
    let pending = project(&spec(false));
    pending.write("here.md", "# here\n");
    let waiting = pending.compile_in(runtime());
    let complete = project(&spec(true));
    complete.write("here.md", "# here\n");
    complete.write("new.js", "// new\n");
    let checked = complete.compile_in(runtime());
    assert_eq!(statuses(&checked, "p1", "files"), [CheckStatus::Passed]);
    // What a verdict over p1 digests is the same before and after: the ungated `refs` alone.
    for project in [&waiting, &checked] {
        let digested = project.checked_values().of("p1");
        let fields: Vec<&str> = digested.iter().map(|v| v.field.as_str()).collect();
        assert_eq!(fields, ["refs"]);
    }
    assert_eq!(
        waiting.checked_values().of("p1"),
        checked.checked_values().of("p1")
    );
}

#[specforge_test(
    behavior = "check_values_by_owner_state",
    verify = "the values waiting for their owner are counted once in I213, by field, and never in I206 or I207"
)]
fn waiting_values_are_counted_once_in_i213() {
    let project = compile(&plan(false, "files", &["a.js", "b.js"]));
    let found = coded(&project, "I213");
    assert_eq!(found.len(), 1, "{:?}", project.diagnostics());
    assert_eq!(
        found[0].message,
        "2 values wait for their owner to complete (plan.files 2): they are checked once it is"
    );
    for code in ["I206", "I207"] {
        assert!(
            coded(&project, code).is_empty(),
            "{code}: {:?}",
            project.diagnostics()
        );
    }
}

#[specforge_test(
    behavior = "check_values_by_owner_state",
    verify = "a gated value spends the compile's one value budget"
)]
fn a_gated_value_spends_the_runs_one_budget() {
    let dir =
        project("plan p1 \"P1\" {\n  done true\n  files [\"new.js\"]\n  refs [\"here.md\"]\n}\n");
    dir.write("here.md", "# here\n");
    dir.write("new.js", "// new\n");
    let budget = ValueBudget {
        local_probes: 1,
        ..ValueBudget::RUN
    };
    let env = dir.environment_in(runtime()).with_value_budget(budget);
    let project = CompiledProject::compile_through(env, &PlainDisk);
    let rows: Vec<CheckStatus> = project
        .value_checks()
        .of("p1")
        .iter()
        .map(|row| row.status)
        .collect();
    assert_eq!(rows, [CheckStatus::Passed, CheckStatus::NotRun]);
    let summary = coded(&project, "I206");
    assert_eq!(summary.len(), 1, "{:?}", project.diagnostics());
    assert!(
        summary[0].message.contains("budget 1"),
        "{}",
        summary[0].message
    );
}
