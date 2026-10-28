//! A host write keeps the harness ledger, and refuses what a field's writer forbids (ADR 0287).

use std::sync::Arc;

use serde_json::json;
use specforge_ops::authoring::edit::{self, Input};
use specforge_ops::authoring::rename;
use specforge_ops::read::navigate::Navigator;
use specforge_ops::view::ProjectView;
use specforge_package::Sha256;
use specforge_project::testing::{
    FakeHistory, GSD_EXTENSION, TestProject, gsd_history_runtime, history,
};
use specforge_test_macros::test as specforge_test;

const BASE: &str = "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\n";

/// The code a typed edit is refused with: the field flags' (`computed_field`, `harness_field`, ADR 0289) before it is
/// planned, else E080 once it is judged.
fn refused(project: &TestProject, entity: &str, field: &str, value: &str) -> Option<String> {
    let compiled = project.compile_in(Arc::new(gsd_history_runtime(json!([]))));
    edit::run(
        &ProjectView::of(&compiled),
        &[crate::edit::set(entity, field, Input::One(value.into()))],
        true,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .err()
    .map(|e| e.error.code.to_string())
}

fn gsd(spec: &str) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[GSD_EXTENSION])
        .installing_enabled()
        .file("a.spec", format!("{BASE}{spec}"))
}

#[specforge_test(
    behavior = "refuse_written_computed",
    verify = "a typed edit of a computed, harness or immutable field is refused"
)]
fn a_typed_edit_of_a_computed_harness_or_immutable_field_is_refused() {
    let verification = gsd("verification V \"V\" {\n  phase 03\n}\n");
    assert_eq!(
        refused(&verification, "v1.0/03/V", "status", "passed").as_deref(),
        Some("computed_field")
    );
    let uat = gsd("uat_test U \"U\" {\n  phase 03\n}\n");
    assert_eq!(
        refused(&uat, "v1.0/03/U", "result", "pass").as_deref(),
        Some("harness_field")
    );
    let decision = "decision D \"D\" {\n  phase 03\n  origin context\n}\n";
    let committed = gsd(decision);
    let fake = Arc::new(FakeHistory::repo(
        committed.root(),
        &"a".repeat(40),
        vec![("a.spec".into(), format!("{BASE}{decision}").into_bytes())],
    ));
    let _installed = history::install(fake);
    assert_eq!(
        refused(&committed, "v1.0/03/D", "origin", "project").as_deref(),
        Some("E080")
    );
}

/// The ledger line that records `value` (JSON text) for `field` of `path`.
fn line(path: &str, field: &str, value: &str) -> String {
    let bytes = format!("{path}\0{field}\0{value}");
    format!("{path} {field} {}\n", Sha256::of(bytes.as_bytes()))
}

fn rename(project: &TestProject, old: &str, new: &str) -> Result<(), String> {
    let compiled = project.compile_in(Arc::new(gsd_history_runtime(json!([]))));
    let nav = Navigator::new(ProjectView::of(&compiled));
    rename::run(
        &nav,
        old,
        new,
        false,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .map(|_| ())
    .map_err(|r| r.error.message)
}

fn e083(project: &TestProject) -> usize {
    let compiled = project.compile_in(Arc::new(gsd_history_runtime(json!([]))));
    compiled
        .diagnostics()
        .iter()
        .filter(|d| d.code == "E083")
        .count()
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "a rename records its move and re-keys its lines"
)]
fn a_rename_records_its_move_and_rekeys_its_lines() {
    let project = gsd("uat_test U1 \"U\" {\n  phase 03\n  result pass\n}\n");
    let header = "# specforge-harness 1\n";
    project.write(
        "specforge-harness",
        format!("{header}{}", line("v1.0/03/U1", "result", "\"pass\"")),
    );
    assert_eq!(e083(&project), 0);
    rename(&project, "v1.0/03", "05").unwrap();
    let ledger = std::fs::read_to_string(project.root().join("specforge-harness")).unwrap();
    assert!(ledger.contains("v1.0/05 moved:v1.0/03 "), "{ledger}");
    assert!(ledger.contains("v1.0/05/U1 result "), "{ledger}");
    assert!(!ledger.contains("v1.0/03/U1 result"), "{ledger}");
    assert_eq!(e083(&project), 0, "{ledger}");
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "two writers both land their lines"
)]
fn two_writers_both_land_their_lines() {
    let uat =
        |phase: &str| format!("uat_test U{phase} \"U\" {{\n  phase {phase}\n  result pass\n}}\n");
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[GSD_EXTENSION])
        .installing_enabled()
        .file("m.spec", "milestone v1.0 \"One\" {}\n")
        .file(
            "a.spec",
            format!(
                "phase 03 \"Three\" {{\n  milestone v1.0\n}}\n\n{}",
                uat("03")
            ),
        )
        .file(
            "b.spec",
            format!(
                "phase 04 \"Four\" {{\n  milestone v1.0\n}}\n\n{}",
                uat("04")
            ),
        );
    let header = "# specforge-harness 1\n";
    project.write(
        "specforge-harness",
        format!(
            "{header}{}{}",
            line("v1.0/03/U03", "result", "\"pass\""),
            line("v1.0/04/U04", "result", "\"pass\"")
        ),
    );
    let compiled = project.compile_in(Arc::new(gsd_history_runtime(json!([]))));
    let nav = Navigator::new(ProjectView::of(&compiled));
    let first = rename::plan(&nav, "v1.0/03", "05").unwrap();
    let second = rename::plan(&nav, "v1.0/04", "06").unwrap();
    std::thread::scope(|scope| {
        for plan in [&first, &second] {
            scope.spawn(|| {
                specforge_ops::authoring::apply::testing::apply(
                    plan.change(),
                    specforge_ops::testing::caller(),
                )
                .unwrap()
            });
        }
    });
    let ledger = std::fs::read_to_string(project.root().join("specforge-harness")).unwrap();
    assert!(ledger.contains("v1.0/05/U03 result "), "{ledger}");
    assert!(ledger.contains("v1.0/06/U04 result "), "{ledger}");
    assert_eq!(e083(&project), 0, "{ledger}");
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "a rename planned on an unchanged ledger re-keys a line another writer added"
)]
fn a_rename_planned_on_an_unchanged_ledger_rekeys_a_line_another_writer_added() {
    let project = gsd("uat_test U1 \"U\" {\n  phase 03\n  result pass\n}\n");
    let compiled = project.compile_in(Arc::new(gsd_history_runtime(json!([]))));
    let nav = Navigator::new(ProjectView::of(&compiled));
    let plan = rename::plan(&nav, "v1.0/03", "05").unwrap();
    // The line arrives after the plan was made, before it is written.
    project.write(
        "specforge-harness",
        format!(
            "# specforge-harness 1\n{}",
            line("v1.0/03/U1", "result", "\"pass\"")
        ),
    );
    specforge_ops::authoring::apply::testing::apply(
        plan.change(),
        specforge_ops::testing::caller(),
    )
    .unwrap();
    let ledger = std::fs::read_to_string(project.root().join("specforge-harness")).unwrap();
    assert!(ledger.contains("v1.0/05/U1 result "), "{ledger}");
    assert!(!ledger.contains("v1.0/03/U1 result"), "{ledger}");
}

#[specforge_test(
    behavior = "collect_harness_ledger",
    verify = "the first ledger write adds the union attribute"
)]
fn the_first_ledger_write_adds_the_union_attribute() {
    let project = gsd("uat_test U1 \"U\" {\n  phase 03\n  result pass\n}\n");
    project.write(
        "specforge-harness",
        format!(
            "# specforge-harness 1\n{}",
            line("v1.0/03/U1", "result", "\"pass\"")
        ),
    );
    let attributes = || std::fs::read_to_string(project.root().join(".gitattributes")).ok();
    assert_eq!(attributes(), None);
    rename(&project, "v1.0/03", "05").unwrap();
    assert_eq!(
        attributes().as_deref(),
        Some("/specforge-harness merge=union\n")
    );
    rename(&project, "v1.0/05", "06").unwrap();
    assert_eq!(
        attributes().as_deref(),
        Some("/specforge-harness merge=union\n")
    );
}

#[specforge_test(
    behavior = "collect_harness_ledger",
    verify = "harness gc rewrites the ledger canonically and --dry-run writes nothing"
)]
fn harness_gc_rewrites_the_ledger_canonically() {
    let project = gsd("uat_test U1 \"U\" {\n  phase 03\n  result pass\n}\n");
    let kept = line("v1.0/03/U1", "result", "\"pass\"");
    let gone = line("v1.0/03/Gone", "result", "\"pass\"");
    let messy = format!("# specforge-harness 1\n{kept}{gone}{kept}");
    project.write("specforge-harness", &messy);
    let gc = |dry_run| {
        let compiled = project.compile_in(Arc::new(gsd_history_runtime(json!([]))));
        specforge_ops::authoring::harness::run(
            &ProjectView::of(&compiled),
            dry_run,
            &specforge_ops::authoring::apply::testing::origin(),
            specforge_ops::testing::caller(),
        )
        .unwrap();
    };
    let ledger = || std::fs::read_to_string(project.root().join("specforge-harness")).unwrap();
    gc(true);
    assert_eq!(ledger(), messy, "--dry-run writes nothing");
    gc(false);
    assert_eq!(ledger(), format!("# specforge-harness 1\n{kept}"));
}

#[specforge_test(
    behavior = "collect_harness_ledger",
    verify = "every host write of a project with a harness field takes the write lock, and one without creates no lock file"
)]
fn every_write_takes_the_lock_only_where_a_ledger_can_exist() {
    let owner = |project: &TestProject,
                 runtime: Arc<specforge_wasm::testing::InProcessRuntime>,
                 entity: &str,
                 field: &str| {
        let compiled = project.compile_in(runtime);
        edit::run(
            &ProjectView::of(&compiled),
            &[crate::edit::set(entity, field, Input::One("x".into()))],
            false,
            &specforge_ops::authoring::apply::testing::origin(),
            specforge_ops::testing::caller(),
        )
        .unwrap();
    };
    // a harness project: an edit that leaves the ledger as it is still takes the lock
    let harnessed = gsd("");
    owner(
        &harnessed,
        Arc::new(gsd_history_runtime(json!([]))),
        "v1.0/03",
        "goal",
    );
    assert!(harnessed.root().join(".specforge/write.lock").exists());
    // a project with no harness field has no ledger to serialize: no lock file
    let plain = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[GSD_EXTENSION])
        .installing_enabled()
        .file("a.spec", "deliverable D \"D\" {}\n");
    owner(
        &plain,
        Arc::new(specforge_project::testing::gsd_member_history_runtime(
            false,
        )),
        "D",
        "owner",
    );
    assert!(!plain.root().join(".specforge/write.lock").exists());
}
