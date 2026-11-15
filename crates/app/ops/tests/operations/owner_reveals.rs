//! A code whose findings wait for an owner state (ADR 0363): the host keeps such a finding only on an entity in that
//! state, read where a computed status decides it, and the write that moves the entity into it reveals the finding,
//! which is no error the write adds (E080). Any other write that leaves one is refused as before.

use std::sync::Arc;

use specforge_extension_sdk::prelude::*;
use specforge_ops::authoring::edit::{self, Input};
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
                        format!("plan '{}' is flagged", e.id),
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

#[specforge_test(
    behavior = "validate_typed_edit",
    verify = "a finding a write reveals by moving its entity into the owner state its code waits for is no error the write adds"
)]
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
