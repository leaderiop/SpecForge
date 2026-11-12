use super::*;
use serde_json::json;
use specforge_test::prelude::*;

/// nextest 0.9's JUnit report of a toy crate (`toy-crate`): the lib's `tests::adds`, `tests::fails` (failing) and
/// `tests::nested::deep`, the bins `extra` and `toy-crate` and the integration test `tests`, its ignored test absent.
const TOY: &str = include_str!("../fixtures/toy.xml");

/// The input the host sends for `report`, these item facts served.
fn input(report: &str, items: Vec<Fact>) -> CollectInput {
    CollectInput {
        reports: vec![CollectReportFile {
            path: REPORT.to_string(),
            content: report.to_string(),
        }],
        facts: items,
        ..CollectInput::default()
    }
}

/// Each fact's `(kind, id, status)`.
fn summary(out: &CollectOutput) -> Vec<(&str, &str, &str)> {
    out.facts
        .iter()
        .map(|f| {
            (
                f.kind.as_str(),
                f.id.as_str(),
                f.fields["status"].as_str().unwrap(),
            )
        })
        .collect()
}

fn place(file: &str, line: usize) -> FactLocation {
    FactLocation {
        file: file.to_string(),
        start_line: line,
        start_col: 5,
        end_line: line,
        end_col: 30,
    }
}

/// The item `@rust/source` answers for a test function `id` with the runner id `test_id` and these `attrs`.
fn item(id: &str, test_id: &str, attrs: &[&str], file: &str, line: usize) -> Fact {
    Fact::new("item", id)
        .field("file", file)
        .field("attrs", attrs.to_vec())
        .field("test_id", test_id)
        .at(place(file, line))
}

#[specforge_test(
    behavior = "rt_declare_test_collector",
    verify = "rust-test declares nextest, its config, its install line and its peers"
)]
fn rust_test_declares_nextest_its_config_its_install_line_and_its_peers() {
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
            ("@code/core", "^1.1", false),
            ("@rust/source", "^1.0", false)
        ]
    );
    let [collector] = &declaration.collectors[..] else {
        panic!("one collector: {:?}", declaration.collectors);
    };
    assert_eq!(collector.name, COLLECTOR);
    assert_eq!(collector.answers, ["test", "test_run"]);
    assert_eq!(collector.inputs, [Input::facts(&["item"])]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(
        collector.install.as_deref(),
        Some("cargo install cargo-nextest --locked")
    );
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    assert_eq!(capabilities[0]["name"], COLLECTOR);
    assert_eq!(capabilities[0]["port"], "process");
    assert_eq!(capabilities[0]["call_kinds"], json!(["collect"]));
    assert_eq!(
        capabilities[0]["argv"],
        json!([
            "cargo-nextest",
            "nextest",
            "run",
            "--workspace",
            "--no-tests=warn",
            "--tool-config-file",
            "specforge:{config}",
            "--profile",
            "specforge"
        ]),
        "{capabilities}"
    );
    assert_eq!(
        capabilities[0]["config"],
        json!({"name": CONFIG, "template": TEMPLATE})
    );
}

#[specforge_test(
    behavior = "rt_declare_test_collector",
    verify = "the config's specforge profile writes JUnit to the report path"
)]
fn the_config_writes_junit_to_the_report_path() {
    let lines: Vec<&str> = TEMPLATE.lines().collect();
    let junit = lines
        .iter()
        .position(|l| *l == "[profile.specforge.junit]")
        .expect("the profile's junit table");
    assert!(
        lines[junit..].contains(&"path = \"{report}\""),
        "{TEMPLATE}"
    );
    assert!(
        lines.contains(&"[profile.specforge]") && lines.contains(&"fail-fast = false"),
        "every test runs: {TEMPLATE}"
    );
    assert_eq!(TEMPLATE.matches("{report}").count(), 1);
}

#[specforge_test(
    behavior = "rt_map_nextest_report",
    verify = "cases become tests named by their binary id and path"
)]
fn cases_become_tests_named_by_binary_and_path() {
    let out = collect(&input(TOY, Vec::new())).unwrap();
    assert_eq!(
        summary(&out),
        [
            ("test", "rust:toy-crate#tests::adds", "passed"),
            ("test_run", "run:rust:toy-crate#tests::adds", "passed"),
            ("test", "rust:toy-crate#tests::fails", "failed"),
            ("test_run", "run:rust:toy-crate#tests::fails", "failed"),
            ("test", "rust:toy-crate#tests::nested::deep", "passed"),
            (
                "test_run",
                "run:rust:toy-crate#tests::nested::deep",
                "passed"
            ),
            ("test", "rust:toy-crate::bin/extra#extra_test", "passed"),
            (
                "test_run",
                "run:rust:toy-crate::bin/extra#extra_test",
                "passed"
            ),
            (
                "test",
                "rust:toy-crate::bin/toy-crate#tests::adds",
                "passed"
            ),
            (
                "test_run",
                "run:rust:toy-crate::bin/toy-crate#tests::adds",
                "passed"
            ),
            ("test", "rust:toy-crate::tests#tests::adds", "passed"),
            (
                "test_run",
                "run:rust:toy-crate::tests#tests::adds",
                "passed"
            ),
        ]
    );
    assert_eq!(out.facts[0].fields["binary"], "toy-crate");
}

#[specforge_test(
    behavior = "rt_map_nextest_report",
    verify = "a test takes its places, file and names from its item"
)]
fn a_test_takes_its_location_file_and_names_from_its_item() {
    let adds = item(
        "rust:toy-crate::tests::adds",
        "rust:toy-crate#tests::adds",
        &["test"],
        "src/lib.rs",
        3,
    )
    .name("names", "tc_add", place("src/lib.rs", 2));
    let out = collect(&input(TOY, vec![adds])).unwrap();
    let test = &out.facts[0];
    assert_eq!(test.id, "rust:toy-crate#tests::adds");
    assert_eq!(test.fields["file"], "src/lib.rs");
    assert_eq!(test.fields["names"], json!(["tc_add"]));
    assert_eq!(test.at, [place("src/lib.rs", 3)]);
    assert_eq!(test.named_at["names"]["tc_add"], [place("src/lib.rs", 2)]);
    assert!(
        !out.facts[2].fields.contains_key("file"),
        "a test with no item has no file"
    );
}

#[specforge_test(
    behavior = "rt_fill_ignored_tests",
    verify = "an ignored test the report omits is a skipped test without a run"
)]
fn an_ignored_test_the_report_omits_is_a_skipped_test_without_a_run() {
    let skipped = item(
        "rust:toy-crate/test/tests::skipped",
        "rust:toy-crate::tests#skipped",
        &["ignore", "test"],
        "tests/tests.rs",
        3,
    );
    let plain = item(
        "rust:toy-crate::not_run",
        "rust:toy-crate#not_run",
        &["test"],
        "src/lib.rs",
        9,
    );
    let out = collect(&input(TOY, vec![skipped, plain])).unwrap();
    let facts: Vec<(&str, &str, &str)> = summary(&out)
        .into_iter()
        .filter(|(_, id, _)| id.contains("skipped") || id.contains("not_run"))
        .collect();
    assert_eq!(
        facts,
        [("test", "rust:toy-crate::tests#skipped", "skipped")],
        "no test_run, and a test item that is not ignored is not invented"
    );
    let skipped = out
        .facts
        .iter()
        .find(|f| f.id == "rust:toy-crate::tests#skipped")
        .unwrap();
    assert_eq!(skipped.fields["file"], "tests/tests.rs");
}

#[specforge_test(
    behavior = "rt_fill_ignored_tests",
    verify = "an ignored test the report holds keeps the status it ran with"
)]
fn an_ignored_test_the_report_holds_keeps_its_status() {
    let deep = item(
        "rust:toy-crate::tests::nested::deep",
        "rust:toy-crate#tests::nested::deep",
        &["ignore", "test"],
        "src/lib.rs",
        6,
    );
    let out = collect(&input(TOY, vec![deep])).unwrap();
    let deep: Vec<(&str, &str, &str)> = summary(&out)
        .into_iter()
        .filter(|(_, id, _)| id.ends_with("#tests::nested::deep"))
        .collect();
    assert_eq!(
        deep,
        [
            ("test", "rust:toy-crate#tests::nested::deep", "passed"),
            (
                "test_run",
                "run:rust:toy-crate#tests::nested::deep",
                "passed"
            ),
        ]
    );
}

#[specforge_test(
    behavior = "rt_map_nextest_report",
    verify = "a case outside exactly one testsuite is the collector's error"
)]
fn a_case_outside_one_binary_suite_is_the_collectors_error() {
    for report in [
        r#"<testsuites><testcase name="x"/></testsuites>"#,
        r#"<testsuites><testsuite name="a"><testsuite name="b"><testcase name="x"/></testsuite></testsuite></testsuites>"#,
    ] {
        let why = collect(&input(report, Vec::new())).unwrap_err();
        assert!(why.starts_with(&format!("{REPORT}: ")), "{why}");
        assert!(why.contains("not a nextest JUnit report"), "{why}");
    }
    let why = collect(&input("<html>", Vec::new())).unwrap_err();
    assert!(why.starts_with(&format!("{REPORT}: ")), "{why}");
}

#[specforge_test(
    behavior = "rt_map_nextest_report",
    verify = "an empty report has no test"
)]
fn an_empty_report_has_no_test() {
    for report in [
        "",
        r#"<testsuites name="nextest-run" tests="0"></testsuites>"#,
    ] {
        let out = collect(&input(report, Vec::new())).unwrap();
        assert_eq!(out.facts, Vec::<Fact>::new(), "{report:?}");
    }
}
