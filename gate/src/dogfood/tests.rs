#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::sync::OnceLock;

use super::*;
use crate::change_set::ChangeSet;
use crate::repo::Repo;

fn write(root: &Path, file: &str) {
    let path = root.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "<testsuites/>").unwrap();
}

#[test]
fn no_report_directory_finds_no_report() {
    let dir = tempfile::tempdir().unwrap();
    assert!(reports(&dir.path().join("target/nextest/gate")).is_empty());
    assert!(
        reports(dir.path()).is_empty(),
        "an empty directory holds none"
    );
}

#[test]
fn a_local_run_finds_its_one_report() {
    let dir = tempfile::tempdir().unwrap();
    let at = report_dir(dir.path(), "gate");
    write(&at, "junit.xml");
    assert_eq!(reports(&at), [at.join("junit.xml")]);
}

#[test]
fn clear_reports_removes_an_earlier_runs_reports_and_accepts_a_missing_directory() {
    let dir = tempfile::tempdir().unwrap();
    let at = report_dir(dir.path(), "gate").join(GUESTS);
    write(&at, "x/junit.xml");
    clear_reports(&at).unwrap();
    assert!(!at.exists(), "{} is still there", at.display());
    clear_reports(&at).unwrap();
}

/// CI downloads each test partition's report into its own directory: every one is found, in path order, and an XML
/// file that is no nextest report is left out.
#[test]
fn the_partitions_reports_are_found_in_their_directories_and_other_xml_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let at = report_dir(dir.path(), "ci");
    for file in [
        "junit-tests-2/junit.xml",
        "junit-tests-1/junit.xml",
        "junit-tests-1/other.xml",
        "notes.txt",
    ] {
        write(&at, file);
    }
    assert_eq!(
        reports(&at),
        [
            at.join("junit-tests-1/junit.xml"),
            at.join("junit-tests-2/junit.xml")
        ]
    );
}

fn workflow() -> String {
    std::fs::read_to_string(specforge_repo_root::repo_root().join(".github/workflows/ci.yml"))
        .unwrap()
}

/// The CI upload step whose artifact is named `name` keeps `report` (from the repository root, under the `ci`
/// profile's report dir) and fails when it is not there.
fn assert_uploads_or_fails(name: &str, report: &Path) {
    let workflow = workflow();
    let step: Vec<&str> = workflow
        .lines()
        .map(str::trim)
        .skip_while(|line| *line != format!("name: {name}"))
        .take_while(|line| !line.is_empty() && !line.starts_with("- "))
        .collect();
    let report = report_dir(Path::new(""), "ci").join(report);
    let report = report.to_string_lossy().replace('\\', "/");
    assert!(
        step.contains(&format!("path: {report}").as_str()),
        "ci.yml: the {name} upload does not keep {report}: {step:?}"
    );
    assert!(
        step.contains(&"if-no-files-found: error"),
        "ci.yml: the {name} upload passes with no report: {step:?}"
    );
}

/// CI keeps each test partition's report for this check, from where nextest writes it under the `ci` profile. A
/// partition whose report is not there fails its upload: the verdict needs one proven requirement, so a lost
/// partition would otherwise read as `no_test` on what only it proves and pass.
#[test]
fn ci_uploads_each_partitions_report_or_fails() {
    assert_uploads_or_fails("junit-tests-${{ matrix.part }}", Path::new("junit.xml"));
}

/// The specs shard keeps the guests' reports under the artifact named as their directory (ADR 0393 D2).
#[test]
fn ci_uploads_the_guests_reports_or_fails() {
    assert_uploads_or_fails(GUESTS, Path::new(GUESTS));
}

/// The gsd shard keeps the `@gsd` workspace's report under the artifact named as its directory (ADR 0393 D2).
#[test]
fn ci_uploads_the_gsd_report_or_fails() {
    assert_uploads_or_fails(GSD, Path::new(GSD));
}

/// The dogfood job downloads every `junit-*` artifact, not only the test partitions': a download limited to them
/// would leave the guests' and the gsd set out, and `report_sets` would refuse.
#[test]
fn the_dogfood_job_downloads_every_junit_artifact() {
    let workflow = workflow();
    let patterns: Vec<&str> = workflow
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pattern: junit"))
        .collect();
    assert_eq!(patterns, ["pattern: junit-*"]);
}

/// The `@gsd` workspace's nextest store is the repository root's, and its `gate` and `ci` runs write their report in
/// the directory `dogfood` reads: a store left in `gsd/target` would hide every `@gsd` test from it.
#[test]
fn the_gsd_workspace_writes_its_junit_under_the_root_report_dir() {
    let path = specforge_repo_root::repo_root().join("gsd/.config/nextest.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = config.lines().map(str::trim).collect();
    let report = format!("path = \"{GSD}/junit.xml\"");
    for line in [
        "dir = \"../target/nextest\"",
        "[profile.gate.junit]",
        "[profile.ci.junit]",
        report.as_str(),
    ] {
        assert!(lines.contains(&line), "{}: no `{line}`", path.display());
    }
}

/// The root tests', the guests' and the `@gsd` workspace's reports are each required, in the local and the CI layout:
/// a missing set is an error naming it and the run that writes it (ADR 0393 D3).
#[test]
fn dogfood_refuses_without_each_report_set() {
    let local = [
        "junit.xml",
        "junit-guests/a/junit.xml",
        "junit-gsd/junit.xml",
    ];
    let ci = [
        "junit-tests-1/junit.xml",
        "junit-guests/a/junit.xml",
        "junit-gsd/junit.xml",
    ];
    for layout in [local, ci] {
        let dir = tempfile::tempdir().unwrap();
        for file in layout {
            write(dir.path(), file);
        }
        report_sets(dir.path(), &reports(dir.path())).unwrap();
    }
    for (missing, set) in [(0, "root tests"), (1, "guests"), (2, "gsd")] {
        let dir = tempfile::tempdir().unwrap();
        for (_, file) in local.iter().enumerate().filter(|(i, _)| *i != missing) {
            write(dir.path(), file);
        }
        let refused = report_sets(dir.path(), &reports(dir.path())).unwrap_err();
        assert!(
            refused.starts_with(&format!("no {set} JUnit report under ")),
            "{refused}"
        );
        assert!(
            refused.contains("cargo gate full --only tests,guests,gsd"),
            "{refused}"
        );
    }
}

#[test]
fn a_failed_requirement_verdict_still_logs_the_x2_error() {
    let lines = ["2 of 3 proved".to_string(), "R-01 has no test".to_string()];
    let x2_error = || Err("X2 ledger: collecting with rust-test: boom".to_string());
    let (logged, result) = outcome(true, &lines, x2_error());
    assert_eq!(
        logged.as_deref(),
        Some("X2 ledger: collecting with rust-test: boom")
    );
    assert_eq!(result, Err("R-01 has no test".to_string()));
    let (logged, result) = outcome(true, &lines, Ok(()));
    assert_eq!(
        (logged, result),
        (None, Err("R-01 has no test".to_string()))
    );
    // Requirements pass: X2's own outcome is the check's, nothing extra logged.
    let (logged, result) = outcome(false, &lines, x2_error());
    assert_eq!(logged, None);
    assert_eq!(result, x2_error());
    assert_eq!(outcome(false, &lines, Ok(())), (None, Ok(())));
}

fn x2_of(present: usize, missing: &[&str]) -> Result<x2::X2, String> {
    Ok(x2::X2 {
        present,
        total: present + missing.len(),
        missing: missing.iter().map(|id| id.to_string()).collect(),
    })
}

/// The check's X2 verdict: every project at the floor passes; one below it, at `0/0` or whose collect failed fails the
/// whole, each failure on its own line in project order.
#[test]
fn judge_x2_fails_when_any_project_fails_and_names_each_failure() {
    assert_eq!(
        judge_x2([("a", x2_of(3, &[])), ("b", x2_of(6, &[]))]),
        Ok(())
    );
    assert_eq!(
        judge_x2([
            ("a", x2_of(3, &[])),
            ("ledger", x2_of(5, &["i"])),
            (
                "c",
                Err("X2 c: collecting with rust-test: boom".to_string())
            ),
            ("d", x2_of(0, &[])),
        ]),
        Err("X2 ledger: 5/6 (83.3%): missing i\n\
             X2 c: collecting with rust-test: boom\n\
             X2 d: no item has a test_id"
            .to_string())
    );
}

/// A fixture's count of items with a test id is pinned: fewer (a partial extractor collapse that still reads `n/n`) or
/// more fails, naming the project and both counts.
#[test]
fn census_holds_a_fixture_to_its_pinned_count_of_items_with_a_test_id() {
    assert_eq!(census("vault", x2_of(3, &[]).unwrap(), 3).unwrap().total, 3);
    assert_eq!(
        census("taskboard", x2_of(1, &[]).unwrap(), 3).err(),
        Some(
            "X2 taskboard: 1 items carry a test id, expected 3: an extractor stopped or started minting test ids"
                .to_string()
        )
    );
    assert!(census("ledger", x2_of(6, &["x"]).unwrap(), 6).is_err());
}

/// A fixture collect passes with Info only; a Warning or an Error fails it, named by code and message, and so does an
/// answer whose shape lost its `diagnostics` array (CONTEXT D-10).
#[test]
fn no_warning_or_error_passes_info_and_names_each_warning_or_error() {
    let with = |severity: &str, code: &str| {
        serde_json::json!({"diagnostics": [
            {"severity": "Info", "code": "I214", "message": "m"},
            {"severity": severity, "code": code, "message": "bad"},
        ]})
    };
    assert_eq!(no_warning_or_error(&with("Info", "I209")), Ok(()));
    assert_eq!(
        no_warning_or_error(&serde_json::json!({"diagnostics": []})),
        Ok(())
    );
    assert_eq!(
        no_warning_or_error(&with("Warning", "W203")),
        Err("W203: bad".to_string())
    );
    assert_eq!(
        no_warning_or_error(&with("Error", "E104")),
        Err("E104: bad".to_string())
    );
    let refused = no_warning_or_error(&serde_json::json!({"facts": 3})).unwrap_err();
    assert!(refused.contains("no diagnostics array"), "{refused}");
}

/// The ledger fixture collected by `rust-cargo`, `rust-source` and `rust-test` over `report` (absolute, or from the
/// repository root), with the real row's extensions and census; leaked for the table's `'static` type.
fn ledger_reading(report: String) -> Fixture {
    let ledger = FIXTURES.iter().find(|f| f.name == "ledger").unwrap();
    let report: &'static str = Box::leak(report.into_boxed_str());
    Fixture {
        name: ledger.name,
        dir: ledger.dir,
        extensions: ledger.extensions,
        collects: Box::leak(
            vec![
                ("rust-cargo", None),
                ("rust-source", None),
                ("rust-test", Some(report)),
            ]
            .into_boxed_slice(),
        ),
        tests: ledger.tests,
    }
}

/// The floor's end-to-end negative control (ADR 0393 D8, CONTEXT D-14): the real `specforge` over a ledger copy whose
/// report lacks one case reads `5/6` and the floor fails naming it; over the golden report the floor passes. The
/// dogfood check runs it first; its log is in a tempdir, so the failing line never reaches dogfood.log (D-09).
#[test]
#[ignore = "needs the built specforge in SPECFORGE_BIN; the dogfood check runs it (ADR 0393 D8)"]
fn x2_floor_fails_a_ledger_copy_missing_one_case() {
    let bin = std::env::var_os("SPECFORGE_BIN").expect(
        "SPECFORGE_BIN must name the built specforge: the dogfood check sets it (cargo gate full --only dogfood)",
    );
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo {
        target_dir: dir.path().to_path_buf(),
        ..crate::repo::this_repo()
    };
    let (specforge, changes) = (OnceLock::new(), ChangeSet::default());
    specforge.set(Ok(PathBuf::from(bin))).unwrap();
    let ctx = Ctx::for_tests(&repo, &changes, &specforge, &dir.path().join("dogfood.log"));
    let specforge = ctx.specforge().unwrap();
    let tracked = steps::tracked_paths(&ctx).unwrap();

    let golden = "gsd/parity/goldens/ledger/rust-test.xml";
    let dropped = "a_memo_with_a_tab_is_refused";
    let text = std::fs::read_to_string(repo.root.join(golden)).unwrap();
    let case = format!("<testcase name=\"{dropped}\"");
    let kept: Vec<&str> = text.lines().filter(|line| !line.contains(&case)).collect();
    assert_eq!(
        kept.len() + 1,
        text.lines().count(),
        "{golden}: one self-closing line holds {case}"
    );
    let doctored = dir.path().join("rust-test.xml");
    std::fs::write(&doctored, kept.join("\n")).unwrap();

    // Judged as `run` judges: through `judge_x2` and `outcome`, so a check that only printed X2 fails here.
    let fixture = ledger_reading(doctored.to_string_lossy().into_owned());
    let x2 = fixture_x2(&ctx, specforge, &fixture, &tracked);
    let (_, result) = outcome(false, &[], judge_x2([("ledger", x2)]));
    let failed = result.unwrap_err();
    assert!(failed.contains("X2 ledger: 5/6"), "{failed}");
    assert!(failed.contains(dropped), "{failed}");

    let x2 = fixture_x2(
        &ctx,
        specforge,
        &ledger_reading(golden.to_string()),
        &tracked,
    );
    assert_eq!(
        outcome(false, &[], judge_x2([("ledger", x2)])),
        (None, Ok(()))
    );
}
