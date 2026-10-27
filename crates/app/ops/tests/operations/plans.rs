//! An extension's edit plan (ADR 0289): a writing command answers typed operations, each naming the revision of its
//! file; the host checks them against the field flags and the revisions, plans them with the typed edit engine,
//! judges the result and writes it all or nothing, with the harness ledger. The fixture is `@t/core`, `@t/ui` and
//! `@t/other` ([`specforge_project::testing::GsdWrites`]); a command answers the plan the test spells.
//!
//! [`run_command`] is the one place a test calls the host: the tickets that change how a plan is applied change it.

use std::sync::Arc;

use serde_json::{Map, Value, json};
use specforge_ops::authoring::apply::{PlanOutcome, execute};
use specforge_ops::extension::command::{CommandFormat, ExtensionCommands};
use specforge_ops::view::ProjectView;
use specforge_package::Sha256;
use specforge_project::testing::{
    CORE, GSD_WRITES_EXTENSIONS, GUARDED, GsdWrites, OTHER, TestProject, UI, gsd_writes_answer,
};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::{InProcessRuntime, RecordedCall};

use crate::writes::{changed_since, files_under};

pub const MILESTONE: &str =
    "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n";
pub const ONE: &str = "uat_test U1 \"One\" {\n  phase 03\n}\n";
pub const TWO: &str =
    "uat_test U2 \"Two\" {\n  phase 03\n}\n\nverification V \"V\" {\n  phase 03\n}\n";

/// A project of three files: the milestone and its phase, and two files each holding a UAT test.
pub fn project() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file("m.spec", MILESTONE)
        .file("03-01.spec", ONE)
        .file("03-02.spec", TWO)
}

/// The revision of `file` as it is on disk now: `sha256:` and the digest of its bytes.
pub fn revision(project: &TestProject, file: &str) -> String {
    let bytes = std::fs::read(project.root().join(file)).unwrap();
    format!("sha256:{}", Sha256::of(&bytes))
}

/// `set` of `field` on `entity`, planned on `revision`.
pub fn set(entity: &str, field: &str, value: Value, revision: &str) -> Value {
    json!({"op": "set", "entity": entity, "field": field, "value": value, "revision": revision})
}

/// A runtime serving the fixture, `exports` answering as given: (extension, export, exit code, operations).
pub fn serving(answers: &[(&str, &str, i32, Value)]) -> Arc<InProcessRuntime> {
    let mut served = GsdWrites::new();
    for (extension, export, exit_code, ops) in answers {
        served = served.answering(
            extension,
            export,
            gsd_writes_answer(*exit_code, ops.clone()),
        );
    }
    Arc::new(served.runtime())
}

pub fn compiled(project: &TestProject, runtime: &Arc<InProcessRuntime>) -> CompiledProject {
    project.compile_in(Arc::clone(runtime) as SharedRuntime)
}

/// What a writing command's run did.
pub struct Ran {
    pub calls: Vec<RecordedCall>,
    /// The project files that changed, relative to the project root, but for the host's own `.specforge/`.
    pub wrote: Vec<String>,
    /// The refusal of the plan the command answered: its code and its data.
    pub refusal: Option<(String, Value)>,
}

impl Ran {
    /// The input the host sent `export`.
    pub fn input(&self, export: &str) -> Value {
        self.calls
            .iter()
            .rev()
            .find(|call| call.export == export)
            .map(|call| call.input.clone())
            .unwrap_or_else(|| panic!("{export} was not called"))
    }

    pub fn wrote(&self, file: &str) -> bool {
        self.wrote.iter().any(|f| f == file)
    }
}

/// Run command `id` of the extension whose short name is `short` over `compiled`, and report what it wrote.
pub fn run_command(
    project: &TestProject,
    compiled: &CompiledProject,
    runtime: &InProcessRuntime,
    short: &str,
    id: &str,
) -> Ran {
    let before = files_under(project.root());
    let view = ProjectView::of(compiled);
    let commands = ExtensionCommands::build(view.registries());
    let command = commands
        .of(short)
        .find(|c| c.id() == id)
        .unwrap_or_else(|| panic!("no command {short} {id}"));
    let run = execute(
        &view,
        command,
        &Map::new(),
        CommandFormat::Json,
        false,
        specforge_ops::testing::caller(),
    )
    .unwrap();
    let refusal = match run.plan {
        Some(PlanOutcome::Refused(error)) => Some((
            error.code.to_string(),
            error.data.map_or(Value::Null, |data| *data),
        )),
        _ => None,
    };
    let wrote = changed_since(project.root(), &before)
        .into_iter()
        .filter(|f| !f.starts_with(".specforge/"))
        .collect();
    Ran {
        calls: runtime.calls(),
        wrote,
        refusal,
    }
}

/// The ledger line that records `value` (JSON text) for `field` of `path`.
pub fn line(path: &str, field: &str, value: &str) -> String {
    let bytes = format!("{path}\0{field}\0{value}");
    format!("{path} {field} {}\n", Sha256::of(bytes.as_bytes()))
}

fn ledger(project: &TestProject) -> String {
    std::fs::read_to_string(project.root().join("specforge-harness")).unwrap_or_default()
}

fn codes(project: &TestProject, runtime: &Arc<InProcessRuntime>, code: &str) -> usize {
    compiled(project, runtime)
        .diagnostics()
        .iter()
        .filter(|d| d.code == code)
        .count()
}

/// A plan of `record` that sets `result` of `U1`, planned on the current bytes of `03-01.spec`.
fn record_one(project: &TestProject) -> Vec<(&'static str, &'static str, i32, Value)> {
    let ops = json!([set(
        "v1.0/03/U1",
        "result",
        json!("pass"),
        &revision(project, "03-01.spec")
    )]);
    vec![(CORE, "cmd__record", 0, ops)]
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a writing command receives the revision of every source and its plan is applied"
)]
fn a_writing_command_gets_revisions_and_answers_a_plan() {
    let p = project();
    let runtime = serving(&record_one(&p));
    let ran = run_command(&p, &compiled(&p, &runtime), &runtime, "core", "record");
    let input = ran.input("cmd__record");
    let revisions = input["revisions"]
        .as_object()
        .expect("a writing command's input holds revisions");
    assert_eq!(
        revisions.keys().collect::<Vec<_>>(),
        ["03-01.spec", "03-02.spec", "m.spec"]
    );
    assert!(
        revisions
            .values()
            .all(|r| r.as_str().is_some_and(|r| r.starts_with("sha256:")))
    );
    assert!(
        ran.wrote("03-01.spec") && ran.wrote("specforge-harness"),
        "{:?}",
        ran.wrote
    );
    assert!(
        std::fs::read_to_string(p.root().join("03-01.spec"))
            .unwrap()
            .contains("result \"pass\"")
    );
    assert!(ledger(&p).contains(&line("v1.0/03/U1", "result", "\"pass\"")));
    assert_eq!(codes(&p, &runtime, "E083"), 0);
}

#[specforge_test(
    behavior = "refuse_stale_revision",
    verify = "a plan naming a stale revision is refused and writes nothing"
)]
fn a_plan_naming_a_stale_revision_is_refused() {
    let p = project();
    let runtime = serving(&record_one(&p));
    let compile = compiled(&p, &runtime);
    // The file changes after the command's compile, before its plan is applied.
    std::fs::write(p.root().join("03-01.spec"), format!("// edited\n{ONE}")).unwrap();
    let ran = run_command(&p, &compile, &runtime, "core", "record");
    let (code, data) = ran.refusal.expect("the plan is refused");
    assert_eq!(code, "stale_revision");
    assert_eq!(data["file"], "03-01.spec");
    assert!(
        data["planned"].as_str().unwrap().starts_with("sha256:") && data["current"].is_string()
    );
    assert!(ran.wrote.is_empty(), "{:?}", ran.wrote);
}

#[cfg(unix)]
#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a plan is all or nothing, the ledger included"
)]
fn a_plan_is_all_or_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let p = project();
    std::fs::create_dir(p.root().join("locked")).unwrap();
    std::fs::set_permissions(
        p.root().join("locked"),
        std::fs::Permissions::from_mode(0o555),
    )
    .unwrap();
    let ops = json!([
        set("v1.0/03/U1", "result", json!("pass"), &revision(&p, "03-01.spec")),
        set("v1.0/03/U2", "result", json!("pass"), &revision(&p, "03-02.spec")),
        {"op": "create", "kind": "decision", "id": "D-01", "title": "D", "file": "locked/d.spec",
         "revision": "absent", "fields": {"phase": "03", "origin": "context"}},
    ]);
    let runtime = serving(&[(CORE, "cmd__record", 0, ops)]);
    let ran = run_command(&p, &compiled(&p, &runtime), &runtime, "core", "record");
    std::fs::set_permissions(
        p.root().join("locked"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    assert_eq!(ran.refusal.expect("the plan fails").0, "file_unwritable");
    assert!(ran.wrote.is_empty(), "{:?}", ran.wrote);
    assert!(ledger(&p).is_empty());
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "two plans on disjoint files both land with their ledger lines"
)]
fn two_plans_on_disjoint_files_both_land() {
    let p = project();
    let rev = |file| revision(&p, file);
    let first = serving(&[(
        CORE,
        "cmd__record",
        0,
        json!([set(
            "v1.0/03/U1",
            "result",
            json!("pass"),
            &rev("03-01.spec")
        )]),
    )]);
    let second = serving(&[(
        CORE,
        "cmd__record",
        0,
        json!([set(
            "v1.0/03/U2",
            "result",
            json!("fail"),
            &rev("03-02.spec")
        )]),
    )]);
    let (one, two) = (compiled(&p, &first), compiled(&p, &second));
    std::thread::scope(|scope| {
        let a = scope.spawn(|| run_command(&p, &one, &first, "core", "record"));
        let b = scope.spawn(|| run_command(&p, &two, &second, "core", "record"));
        for ran in [a.join().unwrap(), b.join().unwrap()] {
            assert!(ran.refusal.is_none(), "{:?}", ran.refusal);
        }
    });
    assert!(
        std::fs::read_to_string(p.root().join("03-01.spec"))
            .unwrap()
            .contains("result \"pass\"")
    );
    assert!(
        std::fs::read_to_string(p.root().join("03-02.spec"))
            .unwrap()
            .contains("result \"fail\"")
    );
    let ledger = ledger(&p);
    assert!(
        ledger.contains(&line("v1.0/03/U1", "result", "\"pass\"")),
        "{ledger}"
    );
    assert!(
        ledger.contains(&line("v1.0/03/U2", "result", "\"fail\"")),
        "{ledger}"
    );
}

#[specforge_test(
    behavior = "refuse_stale_revision",
    verify = "two plans whose union adds an error refuse the later"
)]
fn two_plans_whose_union_adds_an_error_refuse_the_later() {
    let p = project();
    let create = |file: &str| {
        json!([{"op": "create", "kind": "decision", "id": "D-01", "title": "D", "file": file,
                "revision": "absent", "fields": {"phase": "03", "origin": "context"}}])
    };
    let first = serving(&[(CORE, "cmd__record", 0, create("d1.spec"))]);
    let second = serving(&[(CORE, "cmd__record", 0, create("d2.spec"))]);
    let (one, two) = (compiled(&p, &first), compiled(&p, &second));
    let landed = run_command(&p, &one, &first, "core", "record");
    assert!(landed.wrote("d1.spec") && landed.refusal.is_none());
    let later = run_command(&p, &two, &second, "core", "record");
    assert_eq!(later.refusal.expect("the later plan is refused").0, "E080");
    assert!(!p.root().join("d2.spec").exists());
}

/// The code a plan of one `op` from `short`'s command `id` is refused with, if it is.
pub(crate) fn refused(
    p: &TestProject,
    extension: &str,
    short: &str,
    id: &str,
    ops: Value,
) -> Option<String> {
    let export = format!("cmd__{id}");
    let runtime = serving(&[(extension, &export, 0, ops)]);
    run_command(p, &compiled(p, &runtime), &runtime, short, id)
        .refusal
        .map(|(code, _)| code)
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a plan may not write a computed value"
)]
fn a_plan_may_not_write_a_computed_value() {
    let p = project();
    let ops = json!([set(
        "v1.0/03/V",
        "status",
        json!("passed"),
        &revision(&p, "03-02.spec")
    )]);
    assert_eq!(
        refused(&p, CORE, "core", "record", ops).as_deref(),
        Some("computed_field")
    );
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a harness value is set only by a plan of its extension or of a peer"
)]
fn a_harness_value_is_set_only_by_its_extension_or_a_peer() {
    let p = project();
    let one = |p: &TestProject| {
        json!([set(
            "v1.0/03/U1",
            "result",
            json!("pass"),
            &revision(p, "03-01.spec")
        )])
    };
    assert_eq!(refused(&p, CORE, "core", "record", one(&p)), None);
    let q = project();
    assert_eq!(refused(&q, UI, "ui", "mark", one(&q)), None);
    assert!(ledger(&q).contains(&line("v1.0/03/U1", "result", "\"pass\"")));
    let r = project();
    assert_eq!(
        refused(&r, OTHER, "other", "record", one(&r)).as_deref(),
        Some("harness_field")
    );
    // A core typed edit never sets one.
    let compile = r.compile_in(Arc::new(GsdWrites::new().runtime()));
    let edit = crate::edit::set(
        "v1.0/03/U1",
        "result",
        specforge_ops::authoring::edit::Input::One("pass".into()),
    );
    let error = specforge_ops::authoring::edit::run(
        &ProjectView::of(&compile),
        &[edit],
        true,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .unwrap_err();
    assert_eq!(error.error.code, "harness_field");
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a plan from a command that does not declare writes, or that failed, is not applied"
)]
fn a_plan_from_a_command_that_does_not_declare_writes_is_refused() {
    let p = project();
    let ops = |p: &TestProject| {
        json!([set(
            "v1.0/03/U1",
            "result",
            json!("pass"),
            &revision(p, "03-01.spec")
        )])
    };
    assert_eq!(
        refused(&p, CORE, "core", "quiet", ops(&p)).as_deref(),
        Some("undeclared_write")
    );
    // A failed call (exit 1) answers a plan nobody applies.
    let q = project();
    let runtime = serving(&[(CORE, "cmd__record", 1, ops(&q))]);
    let ran = run_command(&q, &compiled(&q, &runtime), &runtime, "core", "record");
    assert!(
        ran.refusal.is_none() && ran.wrote.is_empty(),
        "{:?}",
        ran.wrote
    );
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a plan renames in order and re-keys the ledger once"
)]
fn a_plan_renames_in_order() {
    let uat = |phase: &str, name: &str| {
        format!("uat_test {name} \"U\" {{\n  phase {phase}\n  result pass\n}}\n")
    };
    let p = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file(
            "m.spec",
            format!("milestone v1.0 \"One\" {{}}\n\nphase 03 \"Three\" {{\n  milestone v1.0\n}}\n\nphase 04 \"Four\" {{\n  milestone v1.0\n}}\n\n{}{}", uat("03", "U3"), uat("04", "U4")),
        );
    p.write(
        "specforge-harness",
        format!(
            "# specforge-harness 1\n{}{}",
            line("v1.0/03/U3", "result", "\"pass\""),
            line("v1.0/04/U4", "result", "\"pass\"")
        ),
    );
    let rev = revision(&p, "m.spec");
    let ops = json!([
        {"op": "rename", "entity": "v1.0/04", "to": "05", "revision": rev},
        {"op": "rename", "entity": "v1.0/03", "to": "04", "revision": rev},
    ]);
    let runtime = serving(&[(CORE, "cmd__record", 0, ops)]);
    let ran = run_command(&p, &compiled(&p, &runtime), &runtime, "core", "record");
    assert!(ran.refusal.is_none());
    let text = std::fs::read_to_string(p.root().join("m.spec")).unwrap();
    assert!(
        text.contains("phase 05") && text.contains("phase 04") && !text.contains("phase 03"),
        "{text}"
    );
    let ledger = ledger(&p);
    assert!(
        ledger.contains("v1.0/05 moved:v1.0/04 ") && ledger.contains("v1.0/04 moved:v1.0/03 "),
        "{ledger}"
    );
    assert!(
        ledger.contains("v1.0/04/U3 result ") && ledger.contains("v1.0/05/U4 result "),
        "{ledger}"
    );
    assert_eq!(codes(&p, &runtime, "E083"), 0, "{ledger}");
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a malformed plan is invalid_plan and names the operation"
)]
fn a_malformed_plan_is_invalid_plan() {
    let p = project();
    let rev = revision(&p, "03-01.spec");
    let good = set("v1.0/03/U1", "title", json!("x"), &rev);
    let cases = [
        // A file outside the spec root.
        json!([good, {"op": "create", "kind": "decision", "id": "D-01", "title": "D", "file": "../d.spec", "revision": "absent"}]),
        // Two revisions for one file.
        json!([good, set("v1.0/03/U1", "title", json!("y"), "sha256:00")]),
        // A revision missing.
        json!([good, {"op": "retitle", "entity": "v1.0/03/U1", "title": "T"}]),
        // `verdict` as a field.
        json!([good, set("v1.0/03/U1", "verdict", json!("x"), &rev)]),
    ];
    for ops in cases {
        let runtime = serving(&[(CORE, "cmd__record", 0, ops.clone())]);
        let ran = run_command(&p, &compiled(&p, &runtime), &runtime, "core", "record");
        let (code, data) = ran.refusal.unwrap_or_else(|| panic!("{ops} was applied"));
        assert_eq!(code, "invalid_plan", "{ops}");
        assert_eq!(data["op"], 2, "{ops}");
        assert!(ran.wrote.is_empty());
    }
}

#[specforge_test(
    behavior = "run_guard_passes",
    verify = "a plan a guard pass finds an error in is refused with E080 and nothing is written"
)]
fn a_plan_a_guard_pass_refuses_writes_nothing() {
    let p = project();
    let ops = json!([set(
        "v1.0/03",
        "goal",
        json!(GUARDED),
        &revision(&p, "m.spec")
    )]);
    let runtime = Arc::new(
        GsdWrites::new()
            .guarding_a_goal()
            .answering(CORE, "cmd__record", gsd_writes_answer(0, ops))
            .runtime(),
    );
    let ran = run_command(&p, &compiled(&p, &runtime), &runtime, "core", "record");
    assert_eq!(ran.refusal.map(|(code, _)| code).as_deref(), Some("E080"));
    assert!(ran.wrote.is_empty(), "{:?}", ran.wrote);
}

#[specforge_test(
    behavior = "run_guard_passes",
    verify = "a guard that already objects to the project does not refuse the write that fixes it"
)]
fn a_guard_that_already_objects_does_not_refuse_the_write_that_fixes_it() {
    // The project already holds the entity the guard objects to; renaming another entity changes nothing about it.
    let p = project().file(
        "g.spec",
        format!("phase 04 \"Four\" {{\n  milestone v1.0\n  goal \"{GUARDED}\"\n}}\n"),
    );
    let ops = json!([set(
        "v1.0/03",
        "goal",
        json!("fine"),
        &revision(&p, "m.spec")
    )]);
    let runtime = Arc::new(
        GsdWrites::new()
            .guarding_a_goal()
            .answering(CORE, "cmd__record", gsd_writes_answer(0, ops))
            .runtime(),
    );
    let ran = run_command(&p, &compiled(&p, &runtime), &runtime, "core", "record");
    assert_eq!(ran.refusal, None);
    assert_eq!(ran.wrote, ["m.spec"]);
}
