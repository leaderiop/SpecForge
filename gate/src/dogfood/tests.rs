#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use super::*;

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
