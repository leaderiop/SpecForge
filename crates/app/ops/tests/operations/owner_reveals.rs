//! A code whose findings wait for an owner state (ADR 0363): the host keeps such a finding only on an entity in that
//! state, read where a computed status decides it, and the write that moves the entity into it reveals the finding,
//! which is no error the write adds (E080). Any other write that leaves one is refused as before, among them a write
//! whose plan creates, deletes or renames the entity, whatever id the project held, and a write over a project whose
//! checks were skipped.

use std::sync::Arc;

use specforge_extension_sdk::prelude::*;
use specforge_ops::authoring::edit::{self, Edit, Input, Step};
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_project::testing::TestProject;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@test/reveal";
/// Reported on a flagged plan, only once it is complete.
const ONCE_COMPLETE: &str = "test.reveal/E901";
/// Reported on a flagged plan, whatever its state.
const ALWAYS: &str = "test.reveal/E902";

/// `@test/reveal`: a `plan` complete on `status ∈ [complete]`, its `status` computed from `done` by the compute pass
/// `progress`, which also reports [`ONCE_COMPLETE`] and [`ALWAYS`] on every plan whose `flag` is set.
fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.code("E901", |d| {
        d.title("Flagged once complete")
            .explanation("A complete plan is flagged.")
            .once_complete();
    });
    c.code("E902", |d| {
        d.title("Flagged").explanation("A plan is flagged.");
    });
    c.kind("plan", |k| {
        k.keyword("plan")
            .complete_when("status", &["complete"])
            .field("done", |f| {
                f.field_type(FieldType::Bool);
            })
            .field("flag", |f| {
                f.field_type(FieldType::Bool);
            })
            .field("status", |f| {
                f.vocabulary(&Vocabulary::new(&["draft", "complete"]))
                    .computed();
            });
    });
    c.pass("progress", |p| {
        p.compute().computes(&["plan.status"]).run(progress);
    });
    c
}

fn progress(input: &PassInput) -> PassOutput {
    let plans = || input.entities.iter().filter(|e| e.kind == "plan");
    let set = |e: &PassEntity, field: &str| e.fields.get(field).map(String::as_str) == Some("true");
    PassOutput {
        values: plans()
            .map(|e| PassValue {
                entity: e.id.clone(),
                field: "status".to_string(),
                value: if set(e, "done") { "complete" } else { "draft" }.into(),
            })
            .collect(),
        diagnostics: plans()
            .filter(|e| set(e, "flag"))
            .flat_map(|e| {
                ["E901", "E902"].map(|code| {
                    PassDiagnostic::new(
                        code,
                        PassSeverity::Error,
                        // No id: a renamed plan's finding pairs by its subject alone.
                        "the plan is flagged",
                    )
                    .with_entity(&e.id)
                })
            })
            .collect(),
        ..PassOutput::default()
    }
}

fn compiled(spec: &str) -> CompiledProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", spec)
        .compile_in(Arc::new(InProcessRuntime::new().with(extension)))
}

fn codes(project: &CompiledProject) -> Vec<String> {
    let mut found: Vec<String> = project.diagnostics().into_iter().map(|d| d.code).collect();
    found.retain(|c| c.starts_with("test.reveal/"));
    found
}

/// The code a set of `field` to `true` on the plan `P` is refused with, previewed; `None` when it is not.
fn refused(project: &CompiledProject, field: &str) -> Option<String> {
    edit::run(
        &ProjectView::of(project),
        &[crate::edit::set("P", field, Input::One("true".into()))],
        true,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .err()
    .map(|e| e.error.code.to_string())
}

/// The added errors `steps` are refused with, planned over `project`: their codes, or `None` when it is not refused.
fn refused_steps(project: &CompiledProject, steps: &[Step]) -> Option<Vec<String>> {
    let refusal = edit::plan_steps(&ProjectView::of(project), steps, false).err()?;
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
    Some(codes.collect())
}

/// The create of a flagged, done plan `id`.
fn create(id: &str) -> Step {
    Step::Edit(Edit::Create {
        kind: "plan".into(),
        id: id.into(),
        title: id.into(),
        file: "a.spec".into(),
        fields: [("flag", "true"), ("done", "true")]
            .into_iter()
            .map(|(f, v)| (f.to_string(), Input::One(v.into())))
            .collect(),
        verify: Vec::new(),
    })
}

/// A plan the write completes reveals a waiting finding only when it is the plan the project held: never one the
/// write creates, deletes and creates again, or renames, and never over a project whose checks were skipped.
#[specforge_test(requirement = "FEATURE-131")]
fn only_the_entity_the_project_held_reveals_a_finding() {
    let pending = compiled("plan PA \"PA\" {\n  flag true\n}\n");
    let complete = |id: &str| Step::Edit(crate::edit::set(id, "done", Input::One("true".into())));
    // the positive case, through the same planner
    assert_eq!(refused_steps(&pending, &[complete("PA")]), None);
    // (a) created already complete
    let created = refused_steps(&pending, &[create("QA")]).expect("refused");
    assert!(created.contains(&ONCE_COMPLETE.to_string()), "{created:?}");
    // (b) deleted and created again under the same id in one edit: another entity
    let delete = Step::Edit(Edit::Delete {
        entity_id: "PA".into(),
        unlink_references: false,
    });
    assert_eq!(
        refused_steps(&pending, &[delete, create("PA")]),
        Some(vec![ONCE_COMPLETE.to_string()])
    );
    // (c) renamed, then completed
    let rename = Step::Rename {
        entity: "PA".into(),
        to: "RA".into(),
    };
    assert_eq!(
        refused_steps(&pending, &[rename, complete("RA")]),
        Some(vec![ONCE_COMPLETE.to_string()])
    );
    // (d) the checks were skipped: no owner state is known before the write
    let skipped = compiled("plan PA \"PA\" {\n  flag true\n}\n").skipping_checks();
    assert_eq!(
        refused_steps(&skipped, &[complete("PA")]),
        Some(vec![ONCE_COMPLETE.to_string()])
    );
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_finding_a_write_reveals_is_no_error_it_adds() {
    // Pending: the finding that waits for completion is not reported; the other is.
    let pending = compiled("plan P \"P\" {\n  flag true\n}\n");
    assert_eq!(codes(&pending), [ALWAYS]);
    // The write that completes the plan reveals it: not refused, and reported once written.
    assert_eq!(refused(&pending, "done"), None);
    let complete = compiled("plan P \"P\" {\n  flag true\n  done true\n}\n");
    assert_eq!(codes(&complete), [ONCE_COMPLETE, ALWAYS]);
    // On a complete plan, a write that makes the finding appear adds it: E080, as before.
    let unflagged = compiled("plan P \"P\" {\n  done true\n}\n");
    assert_eq!(codes(&unflagged), Vec::<String>::new());
    assert_eq!(refused(&unflagged, "flag").as_deref(), Some("E080"));
    // A pending plan flagged by the write gains the finding that does not wait: E080 too.
    let quiet = compiled("plan P \"P\" {}\n");
    assert_eq!(refused(&quiet, "flag").as_deref(), Some("E080"));
}
