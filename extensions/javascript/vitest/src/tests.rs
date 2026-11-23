use super::*;
use serde_json::{json, Value};
use specforge_javascript_workspace::PACKAGES;
use specforge_test::prelude::*;

/// vitest 5.1.1's recorded report (see `fixtures/README.md`): one file, `test/board.test.js`.
const RECORDED: &str = include_str!("../fixtures/vitest-5.1.1.junit.xml");

/// The input the host sends for these reports read at `cwd`, with these `(path, text)` package files read.
fn input(reports: &[&str], cwd: &str, packages: &[(&str, &str)]) -> CollectInput {
    let facts: Vec<Value> = packages
        .iter()
        .map(|(path, text)| json!({"path": path, "exists": true, "text": text}))
        .collect();
    let reports: Vec<Value> = reports
        .iter()
        .map(|content| json!({"path": REPORT, "content": content}))
        .collect();
    serde_json::from_value(json!({
        "reports": reports,
        "cwd": cwd,
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

#[specforge_test(requirement = "FEATURE-151")]
fn javascript_vitest_declares_vitest_with_its_junit_reporter_and_its_peers() {
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
    assert_eq!(collector.name, "javascript-vitest");
    assert_eq!(collector.input_formats, ["junit-xml"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(
            &[
                "vitest.config.*".to_string(),
                "vitest.workspace.*".to_string()
            ][..]
        )
    );
    assert_eq!(collector.answers, ["test", "test_run"]);
    assert_eq!(collector.inputs, [packages_input()]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(collector.capture, None);
    assert_eq!(
        collector.install.as_deref(),
        Some("npm install --save-dev vitest")
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
            "vitest",
            "run",
            "--reporter=default",
            "--reporter=junit",
            "--outputFile.junit={report}"
        ]),
        "{capabilities}"
    );
    assert_eq!(capabilities[0]["filter"], json!(["-t", "{filter}"]));
    assert_eq!(capabilities[0]["env"], json!(["NODE_*", "VITEST_*", "CI"]));
}

#[specforge_test(requirement = "FEATURE-151")]
fn vitests_recorded_report_names_its_tests_as_node_test_does() {
    let out = collect(&input(&[RECORDED], "/project", BOARD)).unwrap();
    let file = "js:board/test/board.test.js";
    let expected = [
        ("board > inner > adds", "passed", 1),
        ("board > inner > fails", "failed", 3),
        ("board > skipped", "skipped", 0),
        ("top level", "passed", 0),
    ];
    assert_eq!(
        summary(&out),
        expected
            .iter()
            .flat_map(|(name, status, _)| [
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
    for fact in out.facts.iter().filter(|f| f.kind == "test") {
        assert_eq!(fact.fields["file"], "test/board.test.js", "{}", fact.id);
    }
    let adds = &out.facts[0];
    assert_eq!(adds.fields["suite"], json!(["board", "inner"]));
    assert_eq!(adds.fields["name"], "adds");
    let durations: Vec<&Value> = out
        .facts
        .iter()
        .filter(|f| f.kind == "test_run")
        .map(|f| &f.fields["duration_ms"])
        .collect();
    assert_eq!(
        durations,
        expected
            .map(|(_, _, ms)| json!(ms))
            .iter()
            .collect::<Vec<_>>()
    );
    assert!(
        out.entity_results.is_empty() && out.unlinked.is_empty(),
        "facts only"
    );
}

#[specforge_test(requirement = "FEATURE-151")]
fn a_workspace_test_is_named_after_its_nearest_package() {
    let packages = [
        ("package.json", r#"{"name":"app"}"#),
        ("packages/core/package.json", r#"{"name":"@acme/core"}"#),
    ];
    let report = r#"<testsuites><testsuite name="packages/core/test/x.test.ts">
        <testcase classname="packages/core/test/x.test.ts" name="adds"/>
    </testsuite></testsuites>"#;
    let out = collect(&input(&[report], "/work/app", &packages)).unwrap();
    assert_eq!(out.facts[0].id, "js:@acme/core/test/x.test.ts#adds");
    assert_eq!(out.facts[0].fields["file"], "packages/core/test/x.test.ts");
}

#[specforge_test(requirement = "FEATURE-151")]
fn a_test_no_named_package_owns_is_the_collectors_error() {
    for packages in [&[][..], &[("package.json", r#"{"private":true}"#)][..]] {
        let why = collect(&input(&[RECORDED], "/project", packages)).unwrap_err();
        assert_eq!(
            why,
            "test/board.test.js: no package.json with a name at or above it; javascript-vitest names a test \
             js:<package name>/<file>#<test>, so give the package a name",
            "{packages:?}"
        );
    }
}

#[specforge_test(requirement = "FEATURE-151")]
fn a_file_outside_the_project_or_no_file_is_the_collectors_error() {
    let outside = r#"<testsuites><testcase classname="../a.test.js" name="adds"/></testsuites>"#;
    assert_eq!(
        collect(&input(&[outside], "/w", BOARD)).unwrap_err(),
        "../a.test.js: the test file is outside the project /w"
    );
    let none = r#"<testsuites><testcase name="adds"/></testsuites>"#;
    assert_eq!(
        collect(&input(&[none], "/w", BOARD)).unwrap_err(),
        "vitest's JUnit reporter wrote no classname (the test file) for 'adds'"
    );
}

#[specforge_test(requirement = "FEATURE-151")]
fn zero_or_two_reports_or_not_junit_is_the_collectors_error() {
    for reports in [&[][..], &[RECORDED, RECORDED][..]] {
        let why = collect(&input(reports, "/project", BOARD)).unwrap_err();
        assert_eq!(
            why,
            format!(
                "javascript-vitest reads one report, {REPORT}; it was given {}",
                reports.len()
            )
        );
    }
    let why = collect(&input(&[r#"{"testResults":[]}"#], "/project", BOARD)).unwrap_err();
    assert!(why.starts_with(&format!("{REPORT}: ")), "{why}");
    assert!(why.contains("not a JUnit report"), "{why}");
}
