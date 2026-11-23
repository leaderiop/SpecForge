use super::*;
use serde_json::{json, Value};
use specforge_javascript_workspace::PACKAGES;
use specforge_test::prelude::*;

/// jest 30.5.2 with jest-junit 17.0.0's recorded report through the pack's env file (see `fixtures/README.md`).
const RECORDED: &str = include_str!("../fixtures/jest-30.5.2-jest-junit-17.0.0.junit.xml");

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

#[specforge_test(requirement = "FEATURE-152")]
fn javascript_jest_declares_jest_through_node_and_its_env_file() {
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
    assert_eq!(collector.name, "javascript-jest");
    assert_eq!(collector.input_formats, ["junit-xml"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["jest.config.*".to_string()][..])
    );
    assert_eq!(collector.answers, ["test", "test_run"]);
    assert_eq!(collector.inputs, [packages_input()]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(collector.capture, None);
    assert_eq!(
        collector.install.as_deref(),
        Some("npm install --save-dev jest jest-junit")
    );
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    assert_eq!(capabilities[0]["name"], COLLECTOR);
    assert_eq!(capabilities[0]["port"], "process");
    assert_eq!(
        capabilities[0]["argv"],
        json!([
            "node",
            "--env-file={config}",
            "node_modules/jest/bin/jest.js",
            "--reporters=default",
            "--reporters=jest-junit"
        ]),
        "{capabilities}"
    );
    assert_eq!(
        capabilities[0]["config"],
        json!({"name": "jest-junit.env.toml", "template": TEMPLATE})
    );
    assert_eq!(capabilities[0]["filter"], json!(["-t", "{filter}"]));
    // No `JEST_*`: a variable already in the environment would override the env file's.
    assert_eq!(capabilities[0]["env"], json!(["NODE_*", "CI"]));
}

#[specforge_test(requirement = "FEATURE-152")]
fn the_env_file_sets_jest_junits_six_variables_and_only_report_is_the_hosts() {
    assert_eq!(
        TEMPLATE.lines().collect::<Vec<_>>(),
        [
            r#"JEST_JUNIT_OUTPUT_FILE = "{report}""#,
            r#"JEST_JUNIT_ADD_FILE_ATTRIBUTE = "true""#,
            r#"JEST_JUNIT_ANCESTOR_SEPARATOR = " > ""#,
            r#"JEST_JUNIT_SUITE_NAME = "{filepath}""#,
            r#"JEST_JUNIT_CLASSNAME = "{classname}""#,
            r#"JEST_JUNIT_TITLE = "{title}""#,
        ]
    );
    // The host's placeholders, `{report}` and `{config}`: only the first, once.
    assert_eq!(TEMPLATE.matches("{report}").count(), 1);
    assert!(!TEMPLATE.contains("{config}"));
}

#[specforge_test(requirement = "FEATURE-152")]
fn jests_recorded_report_names_its_tests_by_their_ancestors_then_title() {
    let out = collect(&input(&[RECORDED], "/project", BOARD)).unwrap();
    let file = "js:board/test/board.test.js";
    let expected = [
        ("board > inner > adds", "passed", 1),
        ("board > inner > fails", "failed", 1),
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
    let top = out
        .facts
        .iter()
        .find(|f| f.id == format!("{file}#top level"));
    assert_eq!(top.unwrap().fields["suite"], json!([]));
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
}

#[specforge_test(requirement = "FEATURE-152")]
fn a_case_without_a_file_says_jest_junits_file_attribute_is_off() {
    let report = r#"<testsuites><testsuite name="x"><testcase classname="board" name="adds"/></testsuite></testsuites>"#;
    assert_eq!(
        collect(&input(&[report], "/project", BOARD)).unwrap_err(),
        "jest-junit wrote no file for 'adds': its file attribute is off (the pack's config turns \
         JEST_JUNIT_ADD_FILE_ATTRIBUTE on)"
    );
}

#[specforge_test(requirement = "FEATURE-152")]
fn a_workspace_test_is_named_after_its_nearest_package() {
    let packages = [
        ("package.json", r#"{"name":"app"}"#),
        ("packages/core/package.json", r#"{"name":"@acme/core"}"#),
    ];
    let report = r#"<testsuites><testsuite name="x">
        <testcase classname="" name="adds" file="/work/app/packages/core/test/x.test.js"/>
    </testsuite></testsuites>"#;
    let out = collect(&input(&[report], "/work/app", &packages)).unwrap();
    assert_eq!(out.facts[0].id, "js:@acme/core/test/x.test.js#adds");
    assert_eq!(out.facts[0].fields["file"], "packages/core/test/x.test.js");
}

#[specforge_test(requirement = "FEATURE-152")]
fn a_test_no_named_package_owns_or_outside_the_project_is_the_collectors_error() {
    let why = collect(&input(&[RECORDED], "/project", &[])).unwrap_err();
    assert_eq!(
        why,
        "test/board.test.js: no package.json with a name at or above it; javascript-jest names a test \
         js:<package name>/<file>#<test>, so give the package a name"
    );
    let outside = r#"<testsuites><testcase name="adds" file="/elsewhere/a.test.js"/></testsuites>"#;
    assert_eq!(
        collect(&input(&[outside], "/w", BOARD)).unwrap_err(),
        "/elsewhere/a.test.js: the test file is outside the project /w"
    );
}

#[specforge_test(requirement = "FEATURE-152")]
fn zero_or_two_reports_is_the_collectors_error() {
    for reports in [&[][..], &[RECORDED, RECORDED][..]] {
        let why = collect(&input(reports, "/project", BOARD)).unwrap_err();
        assert_eq!(
            why,
            format!(
                "javascript-jest reads one report, {REPORT}; it was given {}",
                reports.len()
            )
        );
    }
}
