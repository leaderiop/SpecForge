//! A code a later step of the authoring settles (ADR 0385): a typed edit whose plan creates the finding's subject, or
//! an entity under it, and alters nothing the project held there adds no error by that finding (E080), and the
//! finding is still reported. A write that creates neither (an edit, a delete, a creation elsewhere), one that also
//! deletes, sets, unsets or moves what the subject held, a code that does not declare it, another extension's code, a
//! host error and a migration are counted as before.

use std::sync::Arc;

use serde_json::json;
use specforge_extension_sdk::prelude::*;
use specforge_ops::authoring::apply::apply_plan;
use specforge_ops::authoring::edit::{self, Edit, Input, Step};
use specforge_ops::authoring::migrate::{MigrationInput, judge};
use specforge_ops::view::ProjectView;
use specforge_project::testing::TestProject;
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_protocol_types::EditPlan;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

use crate::plans::revision;

const EXT: &str = "@test/later";
const OTHER: &str = "@test/other";
/// A question no decision resolves: settled later.
const OPEN: &str = "test.later/E901";
/// A flagged question: settled by the write that makes it.
const FLAGGED: &str = "test.later/E902";
/// A requirement of a phase in none of its plans, from its first: settled later.
const SHORT: &str = "test.later/E903";
/// A note not done: the other extension's own E901, which it does not declare settled later.
const UNDONE: &str = "test.other/E901";

/// `@test/later`: questions resolved by decisions; requirements of a phase, which its plans, scoped by it, hold.
fn later() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.code("E901", |d| {
        d.title("Open question")
            .explanation("Resolve the question with a decision.")
            .settled_later();
    });
    c.code("E902", |d| {
        d.title("Flagged question").explanation("Unflag it.");
    });
    c.code("E903", |d| {
        d.title("Requirement in no plan")
            .explanation("Plan the rest of the phase.")
            .settled_later();
    });
    c.kind("decision", |k| {
        k.keyword("decision");
    });
    c.kind("question", |k| {
        k.keyword("question")
            .field("resolved_by", |f| {
                f.field_type(FieldType::Reference).target_kind("decision");
            })
            .field("flag", |f| {
                f.field_type(FieldType::Bool);
            });
    });
    c.kind("phase", |k| {
        k.keyword("phase");
    });
    c.kind("requirement", |k| {
        k.keyword("requirement").field("phase", |f| {
            f.field_type(FieldType::Reference).target_kind("phase");
        });
    });
    c.kind("plan", |k| {
        k.keyword("plan")
            .scoped_by("phase")
            .field("phase", |f| {
                f.field_type(FieldType::Reference).target_kind("phase");
            })
            .field("requirements", |f| {
                f.field_type(FieldType::StringList);
            });
    });
    c.pass("later", |p| {
        p.phase("check").run(findings);
    });
    c
}

fn findings(input: &PassInput) -> Vec<PassDiagnostic> {
    let of = |kind: &'static str| input.entities.iter().filter(move |e| e.kind == kind);
    let field = |e: &PassEntity, name: &str| e.fields.get(name).cloned();
    let mut out = Vec::new();
    for question in of("question") {
        if field(question, "resolved_by").is_none() {
            out.push(
                PassDiagnostic::new("E901", PassSeverity::Error, "the question is open")
                    .with_entity(&question.id),
            );
        }
        if field(question, "flag").as_deref() == Some("true") {
            out.push(
                PassDiagnostic::new("E902", PassSeverity::Error, "the question is flagged")
                    .with_entity(&question.id),
            );
        }
    }
    for phase in of("phase") {
        let plans: Vec<&PassEntity> = of("plan")
            .filter(|plan| {
                plan.id
                    .rsplit_once('/')
                    .is_some_and(|(scope, _)| scope == phase.id)
            })
            .collect();
        if plans.is_empty() {
            continue;
        }
        let held: Vec<String> = plans
            .iter()
            .filter_map(|plan| field(plan, "requirements"))
            .flat_map(|list| list.split(", ").map(String::from).collect::<Vec<_>>())
            .collect();
        for requirement in of("requirement") {
            if field(requirement, "phase").as_deref() == Some(phase.id.as_str())
                && !held.contains(&requirement.id)
            {
                let message = format!("requirement {} is in no plan", requirement.id);
                out.push(
                    PassDiagnostic::new("E903", PassSeverity::Error, message)
                        .with_entity(&phase.id),
                );
            }
        }
    }
    out
}

/// `@test/other`: notes, each not done one reported under its own `E901`, which is not settled later.
fn other() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(OTHER, "1.0.0"));
    c.code("E901", |d| {
        d.title("Note not done").explanation("Do it.");
    });
    c.kind("note", |k| {
        k.keyword("note").field("done", |f| {
            f.field_type(FieldType::Bool);
        });
    });
    c.pass("notes", |p| {
        p.phase("check").run(|input: &PassInput| {
            input
                .entities
                .iter()
                .filter(|e| {
                    e.kind == "note" && e.fields.get("done").map(String::as_str) != Some("true")
                })
                .map(|e| {
                    PassDiagnostic::new("E901", PassSeverity::Error, "the note is not done")
                        .with_entity(&e.id)
                })
                .collect::<Vec<_>>()
        });
    });
    c
}

fn runtime() -> SharedRuntime {
    Arc::new(InProcessRuntime::new().with(later).with(other))
}

fn project(spec: &str) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT, OTHER])
        .installing_enabled()
        .file("a.spec", spec)
}

/// The codes of the extensions' findings `project` reports.
fn reported(project: &CompiledProject) -> Vec<String> {
    let mut found: Vec<String> = project.diagnostics().into_iter().map(|d| d.code).collect();
    found.retain(|c| c.starts_with("test."));
    found
}

/// `steps` planned over `spec`'s project: the codes of the findings the write would report (`Ok`), or of the errors
/// E080 refuses it with (`Err`).
fn planned(spec: &str, steps: &[Step]) -> Result<Vec<String>, Vec<String>> {
    let compiled = project(spec).compile_in(runtime());
    match edit::plan_steps(&ProjectView::of(&compiled), steps, false) {
        Ok(plan) => Ok(plan
            .report()
            .added
            .0
            .iter()
            .map(|d| d.code.clone())
            .collect()),
        Err(refusal) => {
            assert_eq!(
                refusal.error.code.to_string(),
                "E080",
                "{}",
                refusal.error.message
            );
            // E080's message lists each added error, `  error[<code>]: <message>`.
            let codes = refusal.error.message.lines().filter_map(|line| {
                let rest = line.trim_start().strip_prefix("error[")?;
                Some(rest.split_once(']')?.0.to_string())
            });
            Err(codes.collect())
        }
    }
}

fn create(kind: &str, id: &str, fields: &[(&str, &str)]) -> Step {
    Step::Edit(Edit::Create {
        kind: kind.into(),
        id: id.into(),
        title: id.into(),
        file: "a.spec".into(),
        fields: fields
            .iter()
            .map(|(f, v)| (f.to_string(), Input::One((*v).into())))
            .collect(),
        verify: Vec::new(),
    })
}

/// A plan of the phase `PH` holding `requirements`.
fn plan(id: &str, requirements: &[&str]) -> Step {
    Step::Edit(Edit::Create {
        kind: "plan".into(),
        id: id.into(),
        title: id.into(),
        file: "a.spec".into(),
        fields: [
            ("phase".to_string(), Input::One("PH".into())),
            ("requirements".to_string(), crate::edit::many(requirements)),
        ]
        .into(),
        verify: Vec::new(),
    })
}

fn delete(id: &str) -> Step {
    Step::Edit(Edit::Delete {
        entity_id: id.into(),
        unlink_references: false,
    })
}

const DECISION: &str = "decision D1 \"D1\" {}\n";
/// A phase and its two requirements, no plan yet.
const PHASE: &str = "phase PH \"PH\" {}\n\nrequirement R1 \"R1\" {\n  phase PH\n}\n\nrequirement R2 \"R2\" {\n  phase PH\n}\n";
/// The phase planned: each requirement in one plan.
const PLANS: &str = "plan P1 \"P1\" {\n  phase PH\n  requirements [\"R1\"]\n}\n\nplan P2 \"P2\" {\n  phase PH\n  requirements [\"R2\"]\n}\n";

fn planned_phase() -> String {
    format!("{PHASE}\n{PLANS}")
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_finding_on_an_entity_the_write_creates_adds_no_error() {
    // The write lists the finding it leaves.
    assert_eq!(
        planned(DECISION, &[create("question", "Q1", &[])]),
        Ok(vec![OPEN.to_string()])
    );
    // Created, then renamed in the same write: the plan creates it at its new id.
    let rename = Step::Rename {
        entity: "Q2".into(),
        to: "Q3".into(),
    };
    assert_eq!(
        planned(DECISION, &[create("question", "Q2", &[]), rename]),
        Ok(vec![OPEN.to_string()])
    );
    // Written, `check` reports it; the later step that answers the question settles it.
    let p = project(DECISION);
    let compiled = p.compile_in(runtime());
    let Step::Edit(question) = create("question", "Q1", &[]) else {
        unreachable!()
    };
    edit::run(
        &ProjectView::of(&compiled),
        &[question],
        false,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .expect("written");
    let written = p.compile_in(runtime());
    assert_eq!(reported(&written), [OPEN]);
    edit::run(
        &ProjectView::of(&written),
        &[crate::edit::set(
            "Q1",
            "resolved_by",
            Input::One("D1".into()),
        )],
        false,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .expect("answered");
    assert_eq!(reported(&p.compile_in(runtime())), Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_finding_on_an_entity_the_write_creates_something_under_adds_no_error() {
    // The phase's first plan: the finding is on the phase, the plan the write creates is under it.
    assert_eq!(
        planned(PHASE, &[plan("P1", &["R1"])]),
        Ok(vec![SHORT.to_string()])
    );
    // A later edit of the plan the write creates alters nothing the project held.
    let retarget = Step::Edit(crate::edit::set(
        "PH/P1",
        "requirements",
        crate::edit::many(&["R2"]),
    ));
    assert_eq!(
        planned(PHASE, &[plan("P1", &["R1"]), retarget]),
        Ok(vec![SHORT.to_string()])
    );
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_write_that_creates_nothing_there_still_adds_the_finding() {
    // Unsetting the decision that resolves an existing question.
    let resolved = format!("{DECISION}\nquestion Q1 \"Q1\" {{\n  resolved_by D1\n}}\n");
    let unset = Step::Edit(Edit::Unset {
        entity_id: "Q1".into(),
        field: "resolved_by".into(),
    });
    assert_eq!(planned(&resolved, &[unset]), Err(vec![OPEN.to_string()]));
    // Deleting the one plan that holds a requirement.
    assert_eq!(
        planned(&planned_phase(), &[delete("PH/P2")]),
        Err(vec![SHORT.to_string()])
    );
    // Creating elsewhere in the same write: the question's own finding is excused, the phase's is not.
    assert_eq!(
        planned(
            &planned_phase(),
            &[delete("PH/P2"), create("question", "Q9", &[])]
        ),
        Err(vec![SHORT.to_string()])
    );
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_write_that_also_alters_what_the_subject_held_still_adds_the_finding() {
    let refused = || Err(vec![SHORT.to_string()]);
    // Deleting the plan that holds R2 and creating another under the phase: the creation does not excuse the delete.
    assert_eq!(
        planned(&planned_phase(), &[delete("PH/P2"), plan("P3", &["R1"])]),
        refused()
    );
    // Setting a held plan's requirements away from R1, or unsetting them, beside a plan created under the phase.
    let away = Step::Edit(crate::edit::set(
        "PH/P1",
        "requirements",
        crate::edit::many(&["R9"]),
    ));
    assert_eq!(
        planned(&planned_phase(), &[away, plan("P3", &["R2"])]),
        refused()
    );
    let unset = Step::Edit(crate::edit::unset("PH/P1", "requirements"));
    assert_eq!(
        planned(&planned_phase(), &[unset, plan("P3", &["R2"])]),
        refused()
    );
    // Moving the plan that holds R2 to another phase (its scope field), beside a plan created under this one.
    let other = format!("{}\nphase PO \"PO\" {{}}\n", planned_phase());
    let away = Step::Edit(crate::edit::set("PH/P2", "phase", Input::One("PO".into())));
    assert_eq!(planned(&other, &[away, plan("P3", &["R1"])]), refused());
}

#[specforge_test(requirement = "FEATURE-131")]
fn only_the_owners_settled_later_code_is_exempt() {
    // A code that does not declare it, on the entity the write creates.
    assert_eq!(
        planned(
            DECISION,
            &[create(
                "question",
                "Q1",
                &[("resolved_by", "D1"), ("flag", "true")]
            )]
        ),
        Err(vec![FLAGGED.to_string()])
    );
    // Another extension's code, of the same local number, which it does not declare settled later.
    assert_eq!(
        planned(DECISION, &[create("note", "N1", &[])]),
        Err(vec![UNDONE.to_string()])
    );
    // A host error on the entity the write creates: no extension's code.
    let host = planned(
        DECISION,
        &[create("question", "Q1", &[("resolved_by", "nowhere")])],
    )
    .expect_err("refused");
    assert!(
        !host.is_empty() && host.iter().all(|code| !code.contains('/')),
        "{host:?}"
    );
}

/// `ops` planned over `spec`'s project at its revision, another writer landing `z.spec` (`landed`) after the compile:
/// the plan is judged again over the disk, under the lock. The project's diagnostics once written, or the refusal.
fn applied_after(spec: &str, ops: serde_json::Value, landed: &str) -> Result<Vec<String>, String> {
    let p = project(spec);
    let compiled = p.compile_in(runtime());
    let at = revision(&p, "a.spec");
    let mut ops = ops;
    for op in ops.as_array_mut().unwrap() {
        op["revision"] = at.clone().into();
    }
    let plan: EditPlan = serde_json::from_value(json!({ "ops": ops })).unwrap();
    p.write("z.spec", landed);
    let edited = apply_plan(
        &ProjectView::of(&compiled),
        &plan,
        &specforge_ops::authoring::apply::testing::origin(),
        false,
        specforge_ops::testing::caller(),
    )
    .map_err(|refusal| {
        assert_eq!(refusal.error.code.to_string(), "E080");
        refusal.error.message
    })?;
    assert!(!edited.wrote.previewed());
    Ok(reported(&p.compile_in(runtime())))
}

#[specforge_test(requirement = "FEATURE-131")]
fn the_re_judge_under_the_lock_exempts_it_too() {
    let question =
        json!([{"op": "create", "kind": "question", "id": "Q1", "title": "Q1", "file": "a.spec"}]);
    assert_eq!(
        applied_after(DECISION, question, "decision DZ \"DZ\" {}\n"),
        Ok(vec![OPEN.to_string()])
    );
}

/// The write deletes a plan that holds only what another plan holds too, and creates a third: nothing it is judged
/// over when planned reports a finding. The other writer's requirement R3, which only the deleted plan held, lands
/// before the lock: only the re-judge sees R3 in no plan, and it counts it, since the write deleted a plan of the phase.
#[specforge_test(requirement = "FEATURE-131")]
fn the_re_judge_under_the_lock_counts_it_when_the_write_altered_the_subject() {
    let spec = "phase PH \"PH\" {}\n\nrequirement R1 \"R1\" {\n  phase PH\n}\n\nplan P1 \"P1\" {\n  phase PH\n  \
                requirements [\"R1\"]\n}\n\nplan P2 \"P2\" {\n  phase PH\n  requirements [\"R1\", \"R3\"]\n}\n";
    let ops = json!([
        {"op": "delete", "entity": "PH/P2"},
        {"op": "create", "kind": "plan", "id": "P3", "title": "P3", "file": "a.spec",
         "fields": {"phase": "PH", "requirements": ["R1"]}},
    ]);
    // Planned over the disk as compiled, the write is clean: it is refused under the lock alone.
    assert_eq!(
        planned(spec, &[delete("PH/P2"), plan("P3", &["R1"])]),
        Ok(Vec::new())
    );
    let refused = applied_after(spec, ops, "requirement R3 \"R3\" {\n  phase PH\n}\n")
        .expect_err("refused under the lock");
    assert!(refused.contains(&format!("error[{SHORT}]")), "{refused}");
}

#[specforge_test(requirement = "FEATURE-122")]
fn a_migration_counts_it() {
    let pre = project(DECISION).compile_in(runtime());
    let text = format!("{DECISION}\nquestion Q1 \"Q1\" {{}}\n");
    let planned = std::collections::BTreeMap::from([("a.spec".to_string(), text)]);
    let input = MigrationInput {
        from: "0.9".into(),
        to: "1.0".into(),
        files: vec![],
    };
    let verdict = judge(&pre, &planned, &input);
    assert!(verdict.compared);
    assert_eq!(verdict.added_errors, 1, "{:?}", verdict.post_diagnostics);
}
