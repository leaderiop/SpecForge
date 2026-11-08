//! A field's checks and its holder's owner state (ADR 0329 D4): what a file check on a pending owner's field reports.

use std::sync::Arc;

use specforge_common::Diagnostic;
use specforge_extension_sdk::prelude::*;
use specforge_project::CompiledProject;
use specforge_project::testing::TestProject;
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

const OWNER: &str = "@test/owner";

/// `@test/owner`: a `plan` complete on `status ∈ [complete]`, its `status` computed from `done` by the compute pass
/// `progress`, and its `files` each a file under the project root (`@specforge/checks:file`).
fn owner() -> InProcessRuntime {
    InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(OWNER, "1.0.0"));
        c.peer("@specforge/checks", "^1.0");
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
                    f.field_type(FieldType::StringList).file(Root::Project);
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

/// `spec` compiled with `@test/owner` and `@specforge/checks` from the builtins' component.
fn compile(spec: &str) -> CompiledProject {
    let runtime = owner().beside(specforge_project::testing::component_runtime());
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[OWNER, "@specforge/checks"])
        .installing_enabled()
        .file("a.spec", spec)
        .compile_in(Arc::new(runtime))
}

fn coded(project: &CompiledProject, code: &str) -> Vec<Diagnostic> {
    project
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == code)
        .collect()
}

const PENDING: &str = "plan p1 \"Open\" {\n  done false\n  files [\"new.js\"]\n}\n";

#[specforge_test(
    behavior = "check_field_values",
    verify = "a pending owner's planned file is E016 until its gate exists"
)]
fn a_pending_owners_file_is_checked_today() {
    let project = compile(PENDING);
    let found = coded(&project, "E016");
    assert_eq!(found.len(), 1, "{:?}", project.diagnostics());
    let span = found[0].span.as_ref().expect("on the value");
    assert_eq!((span.start_line, span.start_col), (3, 11));
}
