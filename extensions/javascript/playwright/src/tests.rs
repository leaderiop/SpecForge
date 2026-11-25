use super::*;
use serde_json::{json, Value};
use specforge_javascript_workspace::PACKAGES;
use specforge_test::prelude::*;

/// Playwright 1.65.1's recorded standard output (see `fixtures/README.md`): one file, `board.spec.js`, relative to
/// the config's `testDir`.
const RECORDED: &str = include_str!("../fixtures/playwright-1.65.1.junit.xml");

/// The input the host sends at `/project`: `stdout` as captured, these report files, these `(path, text)` package
/// files read.
fn input(stdout: Option<&str>, reports: &[&str], packages: &[(&str, &str)]) -> CollectInput {
    let facts: Vec<Value> = packages
        .iter()
        .map(|(path, text)| json!({"path": path, "exists": true, "text": text}))
        .collect();
    let reports: Vec<Value> = reports
        .iter()
        .map(|content| json!({"path": "saved/playwright.xml", "content": content}))
        .collect();
    serde_json::from_value(json!({
        "stdout": stdout,
        "reports": reports,
        "cwd": "/project",
        "files": {PACKAGES: {"facts": facts}},
    }))
    .unwrap()
}

/// The root package `board`.
const BOARD: &[(&str, &str)] = &[("package.json", r#"{"name":"board"}"#)];

/// Each fact's `(kind, id, status)`.
fn summary(out: &CollectOutput) -> Vec<(String, String, String)> {
    out.facts
        .iter()
        .map(|f| {
            (
                f.kind.clone(),
                f.id.clone(),
                f.fields["status"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[specforge_test(requirement = "FEATURE-153")]
fn javascript_playwright_declares_playwright_with_its_stdout_captured() {
    let declaration = crate::specforge_extension_build().declaration();
    let peers: Vec<(&str, &str, bool)> = declaration
        .handshake
        .peer_dependencies
        .iter()
        .map(|p| (p.name.as_str(), p.version.as_str(), p.optional))
        .collect();
    assert_eq!(
        peers,
        [
            ("@code/core", "^1.2", false),
            ("@javascript/source", "^1.0", false)
        ]
    );
    let [collector] = &declaration.collectors[..] else {
        panic!("one collector: {:?}", declaration.collectors);
    };
    assert_eq!(collector.name, "javascript-playwright");
    assert_eq!(collector.input_formats, ["junit-xml"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["playwright.config.*".to_string()][..])
    );
    assert_eq!(collector.answers, ["test", "test_run"]);
    assert_eq!(collector.inputs, [packages_input()]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(collector.capture.as_deref(), Some("stdout"));
    assert_eq!(
        collector.install.as_deref(),
        Some("npm install --save-dev @playwright/test")
    );
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    assert_eq!(capabilities[0]["name"], COLLECTOR);
    assert_eq!(capabilities[0]["port"], "process");
    assert_eq!(
        capabilities[0]["argv"],
        json!([
            "npx",
            "--no",
            "--",
            "playwright",
            "test",
            "--reporter=junit",
            "--output=.specforge/reports/javascript-playwright-results"
        ]),
        "{capabilities}"
    );
    assert_eq!(capabilities[0]["filter"], json!(["--grep", "{filter}"]));
    // Not `PLAYWRIGHT_*`: `PLAYWRIGHT_JUNIT_OUTPUT_NAME` would move the report off standard output.
    assert_eq!(
        capabilities[0]["env"],
        json!(["NODE_*", "PLAYWRIGHT_BROWSERS_PATH", "CI"])
    );
}

#[specforge_test(requirement = "FEATURE-153")]
fn playwrights_captured_report_names_its_tests_by_their_title_path() {
    let out = collect(&input(Some(RECORDED), &[], BOARD)).unwrap();
    let file = "js:board/board.spec.js";
    let expected = [
        ("board > inner > adds", "passed"),
        ("board > inner > fails", "failed"),
        ("board > skipped", "skipped"),
        ("top level", "passed"),
    ];
    assert_eq!(
        summary(&out),
        expected
            .iter()
            .flat_map(|(name, status)| [
                (
                    "test".to_string(),
                    format!("{file}#{name}"),
                    status.to_string()
                ),
                (
                    "test_run".to_string(),
                    format!("run:{file}#{name}"),
                    status.to_string()
                ),
            ])
            .collect::<Vec<_>>()
    );
    let adds = &out.facts[0];
    assert_eq!(adds.fields["suite"], json!(["board", "inner"]));
    assert_eq!(adds.fields["name"], "adds");
    // The file as Playwright writes it: relative to its testDir, not the project (ADR 0389 D9).
    assert_eq!(adds.fields["file"], "board.spec.js");
    assert_eq!(out.facts[1].fields["duration_ms"], 2);
    // The skipped case carries no time attribute: its run has no duration.
    let skipped = &out.facts[5];
    assert_eq!(skipped.kind, "test_run");
    assert!(!skipped.fields.contains_key("duration_ms"), "{skipped:?}");
}

#[specforge_test(requirement = "FEATURE-153")]
fn one_saved_report_reads_as_the_captured_stdout() {
    let captured = collect(&input(Some(RECORDED), &[], BOARD)).unwrap();
    let saved = collect(&input(None, &[RECORDED], BOARD)).unwrap();
    assert_eq!(summary(&saved), summary(&captured));
}

#[specforge_test(requirement = "FEATURE-153")]
fn no_capture_and_not_one_report_is_the_collectors_error() {
    for reports in [&[][..], &[RECORDED, RECORDED][..]] {
        let why = collect(&input(None, reports, BOARD)).unwrap_err();
        assert_eq!(
            why,
            format!(
                "javascript-playwright: Playwright's standard output was not captured, and it reads one report \
                 in its place; it was given {}",
                reports.len()
            )
        );
    }
    let why = collect(&input(Some("Error: no tests found"), &[], BOARD)).unwrap_err();
    assert!(why.starts_with(&format!("{CAPTURE}: ")), "{why}");
}

#[specforge_test(requirement = "FEATURE-153")]
fn a_test_no_named_package_owns_is_the_collectors_error() {
    let why = collect(&input(Some(RECORDED), &[], &[])).unwrap_err();
    assert_eq!(
        why,
        "board.spec.js: no package.json with a name at or above it; javascript-playwright names a test \
         js:<package name>/<file>#<test>, so give the package a name"
    );
}
