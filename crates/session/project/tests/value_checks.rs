//! The values phase (ADR 0336): every value whose field has checks reaches its checker, and a finding lands on the
//! value, under its extension's namespace.

use std::sync::Arc;

use specforge_common::{Diagnostic, Severity};
use specforge_extension_sdk::prelude::*;
use specforge_project::CompiledProject;
use specforge_project::testing::TestProject;
use specforge_project::values::CheckStatus;
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{WasmCallResult, WasmTrapInfo};

const EXT: &str = "@pin/values";

/// `service` with a list `docs`, a scalar `home` and a block list `links` with the member `href`; `no_todo` warns on a
/// text holding `TODO`, naming its bytes; `strict` is an error on one; `off_list` reports a code it does not declare.
fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.code("W001", |d| {
        d.title("A todo").explanation("A todo.");
    });
    c.code("W002", |d| {
        d.title("Another").explanation("Another.");
    });
    c.code("E001", |d| {
        d.title("A strict todo").explanation("A strict todo.");
    });
    let todo = |code: &'static str| {
        move |item: &CheckItem| {
            let findings = item.text.find("TODO").map(|at| {
                CheckFinding::new(code, format!("{} says TODO", item.text)).at(at, at + 4)
            });
            PlannedItem::findings(findings.into_iter().collect())
        }
    };
    c.checker("no_todo", |k| {
        k.applies_to(&["string", "string_list"])
            .codes(&["W001"])
            .plan(todo("W001"));
    });
    c.checker("strict", |k| {
        k.applies_to(&["string"])
            .codes(&["E001"])
            .plan(todo("E001"));
    });
    c.checker("off_list", |k| {
        k.applies_to(&["string"])
            .codes(&["W001"])
            .plan(todo("W002"));
    });
    c.kind("service", |k| {
        k.keyword("service");
        k.field("docs", |f| {
            f.field_type(FieldType::StringList).check("no_todo");
        });
        k.field("home", |f| {
            f.field_type(FieldType::String).check("no_todo");
        });
        k.field("code", |f| {
            f.field_type(FieldType::String).check("off_list");
        });
        k.field("either", |f| {
            f.field_type(FieldType::String)
                .any_of([CheckRef::named("strict"), CheckRef::named("no_todo")]);
        });
        k.field("both", |f| {
            f.field_type(FieldType::String)
                .any_of([CheckRef::named("strict"), CheckRef::named("strict")]);
        });
        k.field("links", |f| {
            f.field_type(FieldType::BlockList);
            f.member("href", |m| {
                m.field_type(FieldType::String).check("no_todo");
            });
        });
    });
    c
}

fn compile(runtime: InProcessRuntime, spec: &str) -> CompiledProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", spec)
        .compile_in(Arc::new(runtime))
}

fn checked(spec: &str) -> CompiledProject {
    compile(InProcessRuntime::new().with(extension), spec)
}

fn coded(project: &CompiledProject, code: &str) -> Vec<Diagnostic> {
    project
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == code)
        .collect()
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a pure checker's finding is on the list item it read"
)]
fn a_pure_checkers_finding_is_on_the_list_item() {
    let spec = "service a \"A\" {\n  docs [\"fine\", \"TODO later\"]\n}\n";
    let project = checked(spec);
    let found = coded(&project, "pin.values/W001");
    assert_eq!(found.len(), 1, "{:?}", project.diagnostics());
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].origin(), Some(EXT));
    assert_eq!(found[0].subject(), Some("a"));
    let span = found[0].span.as_ref().expect("on the item");
    assert_eq!(span.start_line, 2);
    // Inside the quotes of the second item, not at the first or the whole list.
    let line = spec.lines().nth(1).unwrap();
    assert_eq!(&line[span.start_col - 1..span.end_col - 1], "TODO");

    let rows = project.value_checks().of("a");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (
            rows[1].field.as_str(),
            rows[1].item,
            rows[1].checker.as_str()
        ),
        ("docs", 1, "@pin/values:no_todo")
    );
    assert_eq!(rows[1].codes, ["W001"]);
    assert_eq!(rows[0].status, CheckStatus::Passed);
    // A warning is no failure.
    assert_eq!(rows[1].status, CheckStatus::Passed);
    assert_eq!(project.checked_values().of("a").len(), 2);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a finding's byte range points inside the quotes of the value"
)]
fn a_finding_sub_span_points_inside_the_quotes() {
    let spec = "service a \"A\" {\n  home \"see TODO here\"\n}\n";
    let project = checked(spec);
    let found = coded(&project, "pin.values/W001");
    assert_eq!(found.len(), 1, "{:?}", project.diagnostics());
    let span = found[0].span.as_ref().expect("placed");
    let line = spec.lines().nth(1).unwrap();
    assert_eq!(&line[span.start_col - 1..span.end_col - 1], "TODO");
}

#[test]
fn a_value_with_an_escape_keeps_the_whole_value_as_its_span() {
    let spec = "service a \"A\" {\n  home \"a\\\"TODO\"\n}\n";
    let project = checked(spec);
    let found = coded(&project, "pin.values/W001");
    assert_eq!(found.len(), 1, "{:?}", project.diagnostics());
    let span = found[0].span.as_ref().expect("placed");
    let line = spec.lines().nth(1).unwrap();
    assert_eq!(&line[span.start_col - 1..span.end_col - 1], "\"a\\\"TODO\"");
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a code the checker did not declare is W150 and the finding is kept"
)]
fn a_code_the_checker_did_not_declare_is_w150_and_kept() {
    let project =
        checked("service a \"A\" {\n  code \"TODO\"\n}\nservice b \"B\" {\n  code \"TODO\"\n}\n");
    let misused = coded(&project, "W150");
    assert_eq!(misused.len(), 1, "{:?}", project.diagnostics());
    assert!(misused[0].message.contains("off_list"), "{misused:?}");
    assert_eq!(coded(&project, "pin.values/W002").len(), 2);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a failing checker is E028 once per run and its values are not judged"
)]
fn a_failing_checker_is_e028_once_and_its_values_are_not_judged() {
    let runtime = InProcessRuntime::new().with(extension).answer_raw(
        EXT,
        "__check_values",
        WasmCallResult::Trap(WasmTrapInfo {
            kind: "trap".into(),
            message: "boom".into(),
            export_name: "__check_values".into(),
        }),
    );
    let project = compile(
        runtime,
        "service a \"A\" {\n  docs [\"TODO\", \"TODO\"]\n  home \"TODO\"\n}\n",
    );
    assert_eq!(
        coded(&project, "E028").len(),
        1,
        "{:?}",
        project.diagnostics()
    );
    assert!(coded(&project, "pin.values/W001").is_empty());
    let rows = project.value_checks().of("a");
    assert_eq!(rows.len(), 3);
    assert!(
        rows.iter().all(|r| r.status == CheckStatus::NotRun),
        "{rows:?}"
    );
    assert!(rows.iter().all(|r| r.outcome_class.is_none()), "{rows:?}");
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "any_of passes when one branch yields no error-level finding"
)]
fn any_of_passes_when_one_branch_does() {
    // `strict` errors, `no_todo` only warns: the value passes, with what the passing branch said.
    let project = checked("service a \"A\" {\n  either \"TODO\"\n}\n");
    assert!(
        coded(&project, "pin.values/E001").is_empty(),
        "{:?}",
        project.diagnostics()
    );
    assert_eq!(coded(&project, "pin.values/W001").len(), 1);
    let rows = project.value_checks().of("a");
    assert!(
        rows.iter().all(|r| r.status == CheckStatus::Passed),
        "{rows:?}"
    );

    // No branch passes: every branch's findings, each saying what would do.
    let project = checked("service a \"A\" {\n  both \"TODO\"\n}\n");
    let errors = coded(&project, "pin.values/E001");
    assert_eq!(errors.len(), 2, "{:?}", project.diagnostics());
    assert!(
        errors[0]
            .suggestion
            .as_deref()
            .is_some_and(|s| s.starts_with("one of: ")),
        "{errors:?}"
    );
    let rows = project.value_checks().of("a");
    assert!(
        rows.iter().all(|r| r.status == CheckStatus::Failed),
        "{rows:?}"
    );
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a check on a block member reads each written item"
)]
fn a_check_on_a_block_member_reads_each_item() {
    let project = checked(
        "service a \"A\" {\n  links [\n    { href \"fine\" },\n    { href \"TODO\" },\n  ]\n}\n",
    );
    let rows = project.value_checks().of("a");
    let fields: Vec<&str> = rows.iter().map(|r| r.field.as_str()).collect();
    assert_eq!(
        fields,
        ["links[0].href", "links[1].href"],
        "{:?}",
        project.diagnostics()
    );
    let found = coded(&project, "pin.values/W001");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].span.as_ref().unwrap().start_line, 4);
}

#[test]
fn a_project_with_no_checks_calls_no_checker() {
    let runtime = InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@pin/none", "1.0.0"));
        c.kind("gadget", |k| {
            k.keyword("gadget");
        });
        c
    });
    let runtime = Arc::new(runtime);
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&["@pin/none"])
        .installing_enabled()
        .file("a.spec", "gadget a \"A\" {}\n")
        .compile_in(runtime.clone());
    assert!(project.value_checks().is_empty());
    assert!(!runtime.calls().iter().any(|c| c.export == "__check_values"));
}

#[test]
fn the_guest_is_called_once_per_extension_with_every_value() {
    let runtime = Arc::new(InProcessRuntime::new().with(extension));
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file(
            "a.spec",
            "service a \"A\" {\n  docs [\"x\", \"y\"]\n  home \"z\"\n}\n",
        )
        .compile_in(runtime.clone());
    let calls: Vec<_> = runtime
        .calls()
        .into_iter()
        .filter(|c| c.export == "__check_values")
        .collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].input["phase"], "plan");
    assert_eq!(calls[0].input["items"].as_array().unwrap().len(), 3);
    assert_eq!(calls[0].input["items"][1]["item"], 1);
    assert_eq!(calls[0].input["items"][1]["text"], "y");
}

/// `calls` of `__check_values` so far.
fn check_calls(runtime: &InProcessRuntime) -> usize {
    runtime
        .calls()
        .iter()
        .filter(|call| call.export == "__check_values")
        .count()
}

const TODO: &str = "service a \"A\" {\n  docs [\"TODO one\", \"fine\"]\n}\n";

/// The environment of a run that keeps its journal at the project.
fn cached(
    project: &TestProject,
    runtime: &Arc<InProcessRuntime>,
) -> specforge_project::Environment {
    let env = project.environment_in(Arc::clone(runtime) as _);
    let journal = specforge_effects::EffectJournal::at(project.root());
    env.with_effects(specforge_effects::Effects::cached(journal))
}

fn project_files(spec: &str) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", spec)
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a checker asked again in a later run is not called"
)]
fn a_second_check_run_calls_no_checker() {
    let project = project_files(TODO);
    let runtime = Arc::new(InProcessRuntime::new().with(extension));
    let mut session = project.session_in(Arc::clone(&runtime) as _);
    let first = check_calls(&runtime);
    assert_eq!(first, 1, "one plan call for the extension");
    let found = coded(session.project(), "pin.values/W001");
    project.write(
        "a.spec",
        "service a \"A\" {\n  docs [\"TODO one\", \"fine\"]\n  home \"fine\"\n}\n",
    );
    specforge_project::testing::disk_changed(&mut session, &["a.spec"]).expect("a change");
    assert_eq!(
        check_calls(&runtime),
        first + 1,
        "only the new value is asked"
    );
    project.write("a.spec", TODO);
    specforge_project::testing::disk_changed(&mut session, &["a.spec"]).expect("a change");
    assert_eq!(
        check_calls(&runtime),
        first + 1,
        "every value was asked before"
    );
    assert_eq!(coded(session.project(), "pin.values/W001"), found);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a cold run reads the checker cache from disk"
)]
fn a_cold_cli_run_reads_the_disk_cache() {
    let project = project_files(TODO);
    let runtime = Arc::new(InProcessRuntime::new().with(extension));
    let compile = |env| CompiledProject::compile_through(env, &specforge_registry::PlainDisk);
    let first = compile(cached(&project, &runtime));
    assert_eq!(check_calls(&runtime), 1);
    let books: Vec<_> = std::fs::read_dir(project.root().join(".specforge/effects/checks"))
        .expect("written where the journal is")
        .collect();
    assert_eq!(books.len(), 1);
    let again = compile(cached(&project, &runtime));
    assert_eq!(check_calls(&runtime), 1, "a new process asks nothing");
    assert_eq!(
        coded(&again, "pin.values/W001"),
        coded(&first, "pin.values/W001")
    );
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a detached project writes no checker cache"
)]
fn a_detached_journal_writes_no_cache() {
    let project = project_files(TODO);
    let runtime = Arc::new(InProcessRuntime::new().with(extension));
    let env = project.environment_in(Arc::clone(&runtime) as _);
    let env = env.with_effects(specforge_effects::Effects::cached(
        specforge_effects::EffectJournal::detached(),
    ));
    CompiledProject::compile_through(env, &specforge_registry::PlainDisk);
    assert_eq!(check_calls(&runtime), 1);
    assert!(!project.root().join(".specforge/effects/checks").exists());
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "changed args miss the checker cache"
)]
fn changed_args_miss_the_cache() {
    fn checking(args: serde_json::Value) -> ContributionsBuilder {
        let mut c = extension();
        c.kind("widget", move |k| {
            k.keyword("widget");
            k.field("home", |f| {
                f.field_type(FieldType::String)
                    .check_with("levelled", args.clone());
            });
        });
        c.checker("levelled", |k| {
            k.applies_to(&["string"])
                .arg("level", |a| {
                    a.field_type(FieldType::Integer);
                })
                .codes(&["W001"])
                .plan(|_: &CheckItem| PlannedItem::findings(Vec::new()));
        });
        c
    }
    fn one() -> ContributionsBuilder {
        checking(serde_json::json!({"level": 1}))
    }
    fn two() -> ContributionsBuilder {
        checking(serde_json::json!({"level": 2}))
    }
    let spec = "widget w \"W\" {\n  home \"fine\"\n}\n";
    let asked = |factory: fn() -> ContributionsBuilder, project: &TestProject| {
        let runtime = Arc::new(InProcessRuntime::new().with(factory));
        CompiledProject::compile_through(cached(project, &runtime), &specforge_registry::PlainDisk);
        check_calls(&runtime)
    };
    let project = project_files(spec);
    assert_eq!(asked(one, &project), 1);
    assert_eq!(asked(one, &project), 0);
    assert_eq!(asked(two, &project), 1);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "changed settings miss the checker cache"
)]
fn changed_settings_miss_the_cache() {
    fn levelled() -> ContributionsBuilder {
        let mut c = extension();
        c.setting("level", |s| {
            s.integer().default(1);
        });
        c
    }
    let runtime = Arc::new(InProcessRuntime::new().with(levelled));
    let asked = |project: &TestProject| {
        let before = check_calls(&runtime);
        CompiledProject::compile_through(cached(project, &runtime), &specforge_registry::PlainDisk);
        check_calls(&runtime) - before
    };
    let project = project_files(TODO);
    assert_eq!(asked(&project), 1);
    assert_eq!(asked(&project), 0);
    let project = project.config(|c| c["settings"] = serde_json::json!({EXT: {"level": 2}}));
    assert_eq!(asked(&project), 1);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a new blob digest misses the checker cache"
)]
fn a_new_blob_digest_misses_the_cache() {
    use specforge_effects::Provenance;
    use specforge_wasm::WasmRuntime;

    let project = project_files(TODO);
    let runtime = Arc::new(InProcessRuntime::new().with(extension));
    let module = specforge_wasm::testing::spelled_module(EXT, "1.0.0", &[]);
    // Loaded after the environment, which loads the installed blob: what the run reads is this digest.
    let asked = |blob: &[u8]| {
        let before = check_calls(&runtime);
        let env = cached(&project, &runtime);
        runtime
            .load(EXT, &module, Provenance::third_party(blob))
            .expect("the in-process extension loads");
        CompiledProject::compile_through(env, &specforge_registry::PlainDisk);
        check_calls(&runtime) - before
    };
    assert_eq!(asked(b"blob one"), 1);
    assert_eq!(asked(b"blob one"), 0);
    assert_eq!(asked(b"blob two"), 1);
}

#[specforge_test_macros::test(
    behavior = "digest_judged_inputs",
    verify = "a verdict over a checked value digests its outcome class"
)]
fn a_verdict_over_a_checked_value_digests_its_outcome_class() {
    use specforge_project::ValueOutcome;

    let project = checked("service a \"A\" {\n  docs [\"fine\"]\n}\n");
    let digested = project.value_checks().outcome_classes();
    assert_eq!(
        project.checked_values(),
        &digested,
        "what the verdict reads"
    );
    let values = digested.of("a");
    assert_eq!(values.len(), 1);
    assert_eq!(
        values[0].outcome,
        ValueOutcome::Class(specforge_effects::OutcomeClass::Ok)
    );
}
