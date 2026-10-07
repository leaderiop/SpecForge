//! Pins of how operations fail and judge today (plan 02, ticket T0): an
//! unusable test report, a format run's verdict inputs, the project a
//! migration compiles. Plain tests: they pin today's behaviour, bugs
//! included; the ticket that changes a pinned fact flips its pin.

use crate::view_support::{Project, registries};
use specforge_ops::OpErrorKind;
use specforge_ops::analyze::{AnalyzeError, AnalyzeOptions, ReportSource, analyze};
use specforge_ops::format::{Mode, Request, run};
use std::path::Path;

const SOURCE: &str = "behavior b \"B\" {\n  verify unit \"it works\"\n}\n";

fn project() -> Project {
    Project::new(SOURCE, registries(&["behavior"], &[]))
}

fn write_report(project: &Project, text: &str) {
    std::fs::write(project.dir.path().join("specforge-report.json"), text).unwrap();
}

/// Make the report a directory: reading it fails with something that is
/// neither "not found" nor "permission denied".
fn report_is_a_directory(project: &Project) {
    std::fs::create_dir(project.dir.path().join("specforge-report.json")).unwrap();
}

/// Make the report unreadable (`chmod 000`); `false` when the OS still lets
/// this process read it (running as root).
#[cfg(unix)]
fn lock_report(project: &Project) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let path = project.dir.path().join("specforge-report.json");
    std::fs::write(&path, r#"{"results":{}}"#).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    std::fs::read(&path).is_err()
}

fn plan() -> serde_json::Value {
    serde_json::json!({"entries": []})
}

#[test]
fn an_unparsable_report_fails_every_view() {
    let project = project();
    write_report(&project, "{");
    let view = project.view();

    let stats = specforge_ops::stats::stats(&view).unwrap_err().to_string();
    let coverage = specforge_ops::coverage::coverage(&view, &Default::default())
        .unwrap_err()
        .to_string();
    let row = specforge_ops::coverage::row(&view, "b")
        .unwrap_err()
        .to_string();
    let plan = specforge_ops::plan::check(&view, &plan())
        .unwrap_err()
        .to_string();
    let inspect = specforge_ops::inspect::inspect(&view, "b").unwrap();
    let inspect = inspect.coverage.unwrap_err().to_string();

    for message in [stats, coverage, row, plan, inspect] {
        assert!(message.starts_with("invalid test results"), "{message}");
    }
}

#[test]
fn an_unreadable_report_is_unreadable() {
    let project = project();
    report_is_a_directory(&project);

    let error = project.view().test_report().unwrap_err();

    assert!(
        matches!(
            error,
            specforge_project::coverage::ReportError::Unreadable { missing: false, .. }
        ),
        "{error:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_locked_report_is_unreadable() {
    let project = project();
    if !lock_report(&project) {
        return;
    }

    let error = project.view().test_report().unwrap_err();

    assert!(
        matches!(
            error,
            specforge_project::coverage::ReportError::Unreadable { missing: false, .. }
        ),
        "{error:?}"
    );
}

#[test]
fn analyze_calls_a_missing_named_report_a_schema_mismatch() {
    let project = project();
    let options = AnalyzeOptions {
        report: ReportSource::File(project.dir.path().join("none.json")),
        ..Default::default()
    };

    let error = analyze(&project.view(), None, &options).unwrap_err();

    let AnalyzeError::UnusableReport(error) = error else {
        panic!("expected an unusable report: {error:?}");
    };
    assert_eq!(error.kind, OpErrorKind::SchemaMismatch);
    assert_eq!(error.code, "E045");
}

#[test]
fn plan_calls_every_report_failure_a_schema_mismatch() {
    let project = project();
    report_is_a_directory(&project);
    let report = project.view().test_report().unwrap_err();

    let error = specforge_ops::OpError::from(specforge_ops::plan::PlanError::Report(report));

    assert_eq!(error.kind, OpErrorKind::SchemaMismatch);
}

#[test]
fn migrate_compiles_the_path_it_was_given() {
    use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
    use specforge_migrate::CURRENT_FORMAT_VERSION;
    use specforge_ops::migrate::{Request, run_with_hooks};
    use specforge_wasm::testing::InProcessRuntime;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name": "p", "version": "0.1.0", "spec_root": "spec", "extensions": ["@t/x"]}"#,
    )
    .unwrap();
    std::fs::create_dir(root.join("spec")).unwrap();
    std::fs::write(
        root.join("spec/old.spec"),
        "// specforge-format: 0.1\nbehavior old_one \"Old\" {\n}\n",
    )
    .unwrap();
    let runtime = InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/x", "1.0.0"));
        c.migration_hook_handler("migrate_x", |_| Ok(()));
        c
    });
    let declared = |at: &Path| {
        // Start from a fresh copy of the old file each time.
        std::fs::write(
            root.join("spec/old.spec"),
            "// specforge-format: 0.1\nbehavior old_one \"Old\" {\n}\n",
        )
        .unwrap();
        let mut seen: Vec<String> = Vec::new();
        run_with_hooks(
            &Request {
                root: at,
                target: CURRENT_FORMAT_VERSION,
                dry_run: false,
                no_backup: true,
            },
            Some(&runtime),
            &mut |declarations, _| {
                seen = declarations.iter().map(|d| d.name().to_string()).collect();
                (Vec::new(), Vec::new())
            },
        );
        seen
    };

    let from_the_sub_path = declared(&root.join("spec"));
    let from_the_project = declared(root);

    assert!(from_the_sub_path.is_empty(), "{from_the_sub_path:?}");
    assert_eq!(from_the_project, ["@t/x"]);
}

#[test]
fn format_verdict_inputs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("specforge.json"), "{}").unwrap();
    std::fs::write(
        dir.path().join("a.spec"),
        "behavior a \"A\" {\ncontract \"The system MUST work\"\n}\n",
    )
    .unwrap();

    let checked = run(&Request {
        root: dir.path(),
        paths: &[],
        mode: Mode::Check,
    });
    assert!(checked.succeeded() && checked.complete() && !checked.clean());
    assert_eq!(checked.changes.len(), 1);

    let written = run(&Request {
        root: dir.path(),
        paths: &[],
        mode: Mode::Write,
    });
    assert!(written.succeeded() && written.complete() && !written.clean());
    assert!(written.changes.iter().all(|c| c.written));
}
