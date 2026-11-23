use super::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

/// The LCOV reader's hand-written `cargo llvm-cov --lcov` tracefile: crate `demo` at `/project`.
const LLVM_COV: &str = include_str!("../../../code/lcov/fixtures/llvm-cov.info");
/// The LCOV reader's served `item` and `package` facts for that crate, as JSON.
const ITEMS: &str = include_str!("../../../code/lcov/fixtures/items.json");
/// The LCOV reader's accepted snapshot of the facts `LLVM_COV` mints with `ITEMS` served.
const LLVM_COV_FACTS: &str =
    include_str!("../../../code/lcov/src/snapshots/specforge_code_lcov__tests__llvm_cov.snap");
/// Hand-written (see `fixtures/README.md`): one section in the project, two outside it.
const OUTSIDE: &str = include_str!("../fixtures/outside.info");

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

#[specforge_test(requirement = "FEATURE-143")]
fn rust_coverage_declares_llvm_cov_its_install_line_and_its_peers() {
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
            ("@rust/source", "^1.0", false)
        ]
    );
    let [collector] = &declaration.collectors[..] else {
        panic!("one collector: {:?}", declaration.collectors);
    };
    assert_eq!(collector.name, COLLECTOR);
    assert_eq!(collector.input_formats, ["lcov"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["Cargo.toml".to_string()][..])
    );
    assert_eq!(collector.answers, ["coverage"]);
    assert_eq!(collector.inputs, [Input::facts(&["item"])]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(
        collector.install.as_deref(),
        Some("cargo install cargo-llvm-cov cargo-nextest --locked")
    );
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    assert_eq!(capabilities[0]["name"], COLLECTOR);
    assert_eq!(capabilities[0]["port"], "process");
    assert_eq!(
        capabilities[0]["argv"],
        json!([
            "cargo-llvm-cov",
            "llvm-cov",
            "nextest",
            "--workspace",
            "--ignore-run-fail",
            "--lcov",
            "--output-path",
            "{report}"
        ]),
        "{capabilities}"
    );
    assert_eq!(
        capabilities[0]["env"],
        json!([
            "CARGO_*",
            "RUSTUP_*",
            "RUSTC",
            "RUSTC_*",
            "RUSTFLAGS",
            "RUSTDOCFLAGS",
            "LLVM_*"
        ])
    );
    assert!(capabilities[0].get("filter").is_none_or(Value::is_null));
}

#[specforge_test(requirement = "FEATURE-143")]
fn an_llvm_cov_report_mints_the_readers_coverage_facts_and_no_warning() {
    let out = collect(&input(&[LLVM_COV])).unwrap();
    assert_eq!(
        serde_json::to_value(&out.facts).unwrap(),
        snapshot(LLVM_COV_FACTS)
    );
    assert_eq!(out.warnings, Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-143")]
fn a_file_outside_the_project_is_skipped_with_one_warning() {
    let out = collect(&input(&[OUTSIDE])).unwrap();
    let ids: Vec<&str> = out.facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["cov:rust:demo::add"]);
    assert_eq!(out.facts[0].fields["lines_found"], 3);
    assert_eq!(
        out.warnings,
        [format!(
            "{REPORT}: skipped 2 file(s) outside the project: /elsewhere/a.rs, ../../etc/passwd"
        )]
    );
}

#[specforge_test(requirement = "FEATURE-143")]
fn zero_or_two_reports_or_a_bad_one_is_the_collectors_error() {
    for reports in [&[][..], &[LLVM_COV, LLVM_COV][..]] {
        let why = collect(&input(reports)).unwrap_err();
        assert!(why.contains(COLLECTOR) && why.contains(REPORT), "{why}");
    }
    let why = collect(&input(&["garbage"])).unwrap_err();
    assert!(why.starts_with(&format!("{REPORT}: line 1:")), "{why}");
    let no_cwd = CollectInput {
        cwd: None,
        ..input(&[LLVM_COV])
    };
    assert!(collect(&no_cwd).unwrap_err().contains("no cwd"));
}
