use super::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

/// The LCOV reader's c8 12.0.0 recording: `src/board.js` of the package `board`, relative `SF`.
const C8_12: &str = include_str!("../../../code/lcov/fixtures/c8-12.0.0.info");
/// The LCOV reader's served facts for that recording, as JSON.
const ITEMS: &str = include_str!("../../../code/lcov/fixtures/c8-12.0.0.items.json");
/// The LCOV reader's accepted snapshot of the facts `C8_12` mints with `ITEMS` served.
const C8_12_FACTS: &str =
    include_str!("../../../code/lcov/src/snapshots/specforge_code_lcov__tests__c8_12.snap");

/// The input the host sends for these reports at `/project`, the reader's items served.
fn input(reports: &[&str]) -> CollectInput {
    CollectInput {
        reports: reports
            .iter()
            .map(|content| CollectReportFile {
                path: REPORT.to_string(),
                content: content.to_string(),
            })
            .collect(),
        cwd: Some("/project".to_string()),
        facts: serde_json::from_str(ITEMS).expect("the served facts parse"),
        ..CollectInput::default()
    }
}

/// The JSON body of an insta snapshot, after its `---` header.
fn snapshot(text: &str) -> Value {
    let body = text.splitn(3, "---\n").nth(2).expect("a snapshot body");
    serde_json::from_str(body).expect("the snapshot parses")
}

#[specforge_test(requirement = "FEATURE-144")]
fn javascript_coverage_declares_c8_around_npm_test_and_its_peers() {
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
    assert_eq!(collector.name, COLLECTOR);
    assert_eq!(collector.input_formats, ["lcov"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["package.json".to_string()][..])
    );
    assert_eq!(collector.answers, ["coverage"]);
    assert_eq!(collector.inputs, [Input::facts(&["item"])]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(
        collector.install.as_deref(),
        Some("npm install --save-dev c8")
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
            "c8",
            "--reporter=lcovonly",
            "--reports-dir=.specforge/reports/javascript-coverage",
            "npm",
            "test"
        ]),
        "{capabilities}"
    );
    assert_eq!(
        capabilities[0]["env"],
        json!(["NODE_*", "NPM_CONFIG_*", "npm_config_*", "CI"])
    );
    assert!(capabilities[0].get("filter").is_none_or(Value::is_null));
    // c8 writes `lcov.info` in the directory the argv names.
    assert!(REPORT.ends_with("/lcov.info"));
    assert!(capabilities[0]["argv"]
        .as_array()
        .unwrap()
        .contains(&json!(format!(
            "--reports-dir={}",
            REPORT.trim_end_matches("/lcov.info")
        ))));
}

#[specforge_test(requirement = "FEATURE-144")]
fn a_c8_report_mints_the_readers_coverage_facts_and_no_warning() {
    let out = collect(&input(&[C8_12])).unwrap();
    assert_eq!(
        serde_json::to_value(&out.facts).unwrap(),
        snapshot(C8_12_FACTS)
    );
    assert_eq!(out.warnings, Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-144")]
fn a_file_outside_the_project_is_skipped_with_one_warning() {
    let report = format!("{C8_12}SF:../shared/util.js\nDA:1,1\nend_of_record\n");
    let out = collect(&input(&[&report])).unwrap();
    assert_eq!(
        serde_json::to_value(&out.facts).unwrap(),
        snapshot(C8_12_FACTS)
    );
    assert_eq!(
        out.warnings,
        [format!(
            "{REPORT}: skipped 1 file(s) outside the project: ../shared/util.js"
        )]
    );
}

#[specforge_test(requirement = "FEATURE-144")]
fn zero_or_two_reports_or_a_bad_one_is_the_collectors_error() {
    for reports in [&[][..], &[C8_12, C8_12][..]] {
        let why = collect(&input(reports)).unwrap_err();
        assert!(why.contains(COLLECTOR) && why.contains(REPORT), "{why}");
    }
    let why = collect(&input(&["garbage"])).unwrap_err();
    assert!(why.starts_with(&format!("{REPORT}: line 1:")), "{why}");
}
