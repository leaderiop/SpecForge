use super::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

/// The SARIF reader's ESLint 10.15.0 recording: `src/board view.js` of the package `board` at `/project`.
const ESLINT_10_15: &str = include_str!("../../../code/sarif/fixtures/eslint-10.15.0.sarif");
/// The SARIF reader's served facts for that recording, as JSON.
const ITEMS: &str = include_str!("../../../code/sarif/fixtures/eslint-10.15.0.items.json");
/// The SARIF reader's accepted snapshot of the facts `ESLINT_10_15` mints with `ITEMS` served.
const ESLINT_10_15_FACTS: &str = include_str!(
    "../../../code/sarif/src/snapshots/specforge_code_sarif__tests__eslint_10_15.snap"
);

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

#[specforge_test(requirement = "FEATURE-145")]
fn javascript_lint_declares_eslint_with_the_sarif_formatter_and_its_peers() {
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
    assert_eq!(collector.input_formats, ["sarif"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["eslint.config.*".to_string()][..])
    );
    assert_eq!(collector.answers, ["static_finding"]);
    assert_eq!(collector.inputs, [Input::facts(&["item", "package"])]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(
        collector.install.as_deref(),
        Some("npm install --save-dev eslint @microsoft/eslint-formatter-sarif")
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
            "eslint",
            "--format",
            "@microsoft/eslint-formatter-sarif",
            "--output-file",
            "{report}",
            "."
        ]),
        "{capabilities}"
    );
    assert_eq!(capabilities[0]["env"], json!(["NODE_*", "ESLINT_*", "CI"]));
    assert!(capabilities[0].get("filter").is_none_or(Value::is_null));
}

#[specforge_test(requirement = "FEATURE-145")]
fn an_eslint_report_mints_the_readers_static_findings_and_no_warning() {
    let out = collect(&input(&[ESLINT_10_15])).unwrap();
    assert_eq!(
        serde_json::to_value(&out.facts).unwrap(),
        snapshot(ESLINT_10_15_FACTS)
    );
    assert_eq!(out.warnings, Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-145")]
fn a_result_outside_the_project_is_skipped_with_one_warning() {
    let outside = ESLINT_10_15.replacen(
        "file:///project/src/board%20view.js\",\n                  \"index\": 1",
        "file:///elsewhere/vendor.js\",\n                  \"index\": 1",
        1,
    );
    assert_ne!(outside, ESLINT_10_15, "the recording names its file by uri");
    let out = collect(&input(&[&outside])).unwrap();
    let rules: Vec<&str> = out
        .facts
        .iter()
        .map(|f| f.fields["rule"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(rules, ["no-undef", "no-console", "eqeqeq"]);
    assert_eq!(
        out.warnings,
        [format!(
            "{REPORT}: skipped 1 file(s) outside the project: /elsewhere/vendor.js"
        )]
    );
}

#[specforge_test(requirement = "FEATURE-145")]
fn a_result_naming_no_file_is_skipped_with_its_own_warning() {
    let rooted = ESLINT_10_15.replacen(
        "file:///project/src/board%20view.js\",\n                  \"index\": 1",
        "file:///project\",\n                  \"index\": 1",
        1,
    );
    assert_ne!(rooted, ESLINT_10_15, "the recording names its file by uri");
    let out = collect(&input(&[&rooted])).unwrap();
    assert_eq!(out.facts.len(), 3, "{:?}", out.facts);
    assert_eq!(
        out.warnings,
        [format!(
            "{REPORT}: skipped 1 path(s) naming no file: /project"
        )]
    );
}

#[specforge_test(requirement = "FEATURE-145")]
fn zero_or_two_reports_or_a_bad_one_is_the_collectors_error() {
    for reports in [&[][..], &[ESLINT_10_15, ESLINT_10_15][..]] {
        let why = collect(&input(reports)).unwrap_err();
        assert!(why.contains(COLLECTOR) && why.contains(REPORT), "{why}");
    }
    let why = collect(&input(&["garbage"])).unwrap_err();
    assert!(why.starts_with(&format!("{REPORT}: no JSON")), "{why}");
    let no_cwd = CollectInput {
        cwd: None,
        ..input(&[ESLINT_10_15])
    };
    assert!(collect(&no_cwd).unwrap_err().contains("no cwd"));
}
