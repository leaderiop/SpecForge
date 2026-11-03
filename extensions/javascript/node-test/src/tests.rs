use super::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

/// The taskboard twin's recorded report (`gsd/parity/goldens/taskboard/junit.xml`, kept by the JUnit library):
/// three top-level tests, files relative to the project.
const TASKBOARD: &str = include_str!("../../../code/junit/fixtures/taskboard.xml");
/// node 26.12's JUnit report of one file under `/project`: a top-level test, `describe("board")` holding a passing
/// test and `describe("inner")` with a failing one, a skipped test and a `todo` (the JUnit library's fixture).
const NESTED: &str = include_str!("../../../code/junit/fixtures/node26-nested.xml");

/// The input the host sends for `report` read at `cwd`, with these `(path, text)` package files read.
fn input(report: &str, cwd: &str, packages: &[(&str, &str)]) -> CollectInput {
    let facts: Vec<Value> = packages
        .iter()
        .map(|(path, text)| json!({"path": path, "exists": true, "text": text}))
        .collect();
    serde_json::from_value(json!({
        "reports": [{"path": REPORT, "content": report}],
        "cwd": cwd,
        "files": {PACKAGES: {"facts": facts}},
    }))
    .unwrap()
}

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

/// The `test` facts' `(id, status)`.
fn tests(out: &CollectOutput) -> Vec<(String, String)> {
    summary(out)
        .into_iter()
        .filter(|(kind, _, _)| kind == "test")
        .map(|(_, id, status)| (id, status))
        .collect()
}

/// A one-test report of `file`.
fn one(file: &str) -> String {
    format!(r#"<testsuites><testcase name="adds" classname="test" file="{file}"/></testsuites>"#)
}

#[specforge_test(
    behavior = "nt_map_node_test_report",
    verify = "the twin's tests are named after its package"
)]
fn the_twins_tests_are_named_after_its_package() {
    let out = collect(&input(
        TASKBOARD,
        "/work/taskboard",
        &[("package.json", r#"{"name":"taskboard"}"#)],
    ))
    .unwrap();
    let file = "js:taskboard/test/tasks.test.js";
    assert_eq!(
        summary(&out),
        [
            ("test", format!("{file}#a task is added with the next id")),
            (
                "test_run",
                format!("run:{file}#a task is added with the next id")
            ),
            ("test", format!("{file}#a task is completed by its id")),
            (
                "test_run",
                format!("run:{file}#a task is completed by its id")
            ),
            ("test", format!("{file}#completing an unknown id throws")),
            (
                "test_run",
                format!("run:{file}#completing an unknown id throws")
            ),
        ]
        .map(|(kind, id)| (kind.to_string(), id, "passed".to_string()))
    );
    assert!(
        out.entity_results.is_empty() && out.unlinked.is_empty(),
        "facts only"
    );
}

#[specforge_test(
    behavior = "nt_map_node_test_report",
    verify = "a workspace test is named after its nearest package"
)]
fn a_workspace_test_is_named_after_its_nearest_package() {
    let packages = [
        (
            "package.json",
            r#"{"name":"app","workspaces":["packages/*"]}"#,
        ),
        ("packages/core/package.json", r#"{"name":"@acme/core"}"#),
        ("packages/cli/package.json", r#"{"private":true}"#),
    ];
    let report = r#"<testsuites>
        <testcase name="adds" file="/work/app/packages/core/test/x.test.js"/>
        <testcase name="runs" file="/work/app/packages/cli/test/y.test.js"/>
        <testcase name="boots" file="/work/app/test/z.test.js"/>
    </testsuites>"#;
    let out = collect(&input(report, "/work/app", &packages)).unwrap();
    assert_eq!(
        tests(&out),
        [
            "js:@acme/core/test/x.test.js#adds",
            "js:app/packages/cli/test/y.test.js#runs",
            "js:app/test/z.test.js#boots",
        ]
        .map(|id| (id.to_string(), "passed".to_string()))
    );
}

#[specforge_test(
    behavior = "nt_map_node_test_report",
    verify = "an absolute file is made relative to the project root"
)]
fn an_absolute_file_is_made_relative_to_cwd() {
    let out = collect(&input(
        &one("/work/proj/test/a.test.js"),
        "/work/proj",
        &[("package.json", r#"{"name":"p"}"#)],
    ))
    .unwrap();
    assert_eq!(tests(&out)[0].0, "js:p/test/a.test.js#adds");
    assert_eq!(out.facts[0].fields["file"], "test/a.test.js");
}

#[specforge_test(
    behavior = "nt_map_node_test_report",
    verify = "a Windows file is made relative to a verbatim cwd"
)]
fn a_windows_file_is_made_relative_to_a_verbatim_cwd() {
    let out = collect(&input(
        &one(r"C:\proj\test\a.test.js"),
        r"\\?\C:\proj",
        &[("package.json", r#"{"name":"p"}"#)],
    ))
    .unwrap();
    assert_eq!(tests(&out)[0].0, "js:p/test/a.test.js#adds");
}

#[specforge_test(
    behavior = "nt_map_node_test_report",
    verify = "a failing and a skipped test keep their status"
)]
fn a_failing_and_a_skipped_test_keep_their_status() {
    let out = collect(&input(
        NESTED,
        "/project",
        &[("package.json", r#"{"name":"probe"}"#)],
    ))
    .unwrap();
    let file = "js:probe/test/a.test.js";
    let expected = [
        ("board > adds", "passed"),
        ("board > inner > adds", "failed"),
        ("skipped", "skipped"),
        ("todo one", "skipped"),
        ("top", "passed"),
    ]
    .map(|(name, status)| (format!("{file}#{name}"), status.to_string()));
    assert_eq!(tests(&out), expected);
    let runs: Vec<(String, String)> = summary(&out)
        .into_iter()
        .filter(|(kind, _, _)| kind == "test_run")
        .map(|(_, id, status)| (id, status))
        .collect();
    assert_eq!(
        runs,
        expected.map(|(id, status)| (format!("run:{id}"), status))
    );
}

#[specforge_test(
    behavior = "nt_map_node_test_report",
    verify = "a test with no named package is the collector's error"
)]
fn a_test_with_no_named_package_is_the_collectors_error() {
    for packages in [
        &[][..],
        &[("package.json", r#"{"private":true}"#)][..],
        &[("package.json", "not json")][..],
        &[("other/package.json", r#"{"name":"other"}"#)][..],
    ] {
        let why = collect(&input(&one("/w/test/a.test.js"), "/w", packages)).unwrap_err();
        assert_eq!(
            why,
            "test/a.test.js: no package.json with a name at or above it; node-test names a test \
             js:<package name>/<file>#<test>, so give the package a name",
            "{packages:?}"
        );
    }
}

#[specforge_test(
    behavior = "nt_map_node_test_report",
    verify = "a case without a file is the collector's error"
)]
fn a_case_without_a_file_is_the_collectors_error() {
    let report = r#"<testsuites><testcase name="adds" classname="test"/></testsuites>"#;
    let why = collect(&input(report, "/w", &[("package.json", r#"{"name":"p"}"#)])).unwrap_err();
    assert_eq!(
        why,
        "node's JUnit reporter wrote no file for 'adds'; node 20.10 and later do"
    );
}

#[specforge_test(
    behavior = "nt_map_node_test_report",
    verify = "a file outside the project, absolute or through .., is the collector's error"
)]
fn a_file_outside_the_project_is_the_collectors_error() {
    for file in [
        "/elsewhere/a.test.js",
        "../a.test.js",
        "/w/test/../../a.test.js",
    ] {
        let why = collect(&input(
            &one(file),
            "/w",
            &[("package.json", r#"{"name":"p"}"#)],
        ))
        .unwrap_err();
        assert_eq!(
            why,
            format!("{file}: the test file is outside the project /w")
        );
    }
}

#[specforge_test(
    behavior = "nt_map_node_test_report",
    verify = "a report that is not JUnit is the collector's error"
)]
fn a_report_that_is_not_junit_is_the_collectors_error() {
    let why = collect(&input(r#"{"testResults":[]}"#, "/w", &[])).unwrap_err();
    assert!(why.starts_with(&format!("{REPORT}: ")), "{why}");
    assert!(why.contains("not a JUnit report"), "{why}");
}
