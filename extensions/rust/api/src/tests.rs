use super::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

/// The API reader's `cargo public-api` snapshot (hand-written to the tool's lines): crate `example_api`.
const SNAPSHOT: &str = include_str!("../../../code/api/fixtures/rust.public-api.txt");
/// The API reader's served items for it, as JSON.
const ITEMS: &str = include_str!("../../../code/api/fixtures/items.json");
/// The API reader's accepted snapshot of the facts `SNAPSHOT` mints with `ITEMS` served.
const SNAPSHOT_FACTS: &str =
    include_str!("../../../code/api/src/snapshots/specforge_code_api__tests__rust.snap");

/// The input the host sends: `stdout` as captured, these report files, the reader's items served.
fn input(stdout: Option<&str>, reports: &[&str]) -> CollectInput {
    CollectInput {
        stdout: stdout.map(ToString::to_string),
        reports: reports
            .iter()
            .map(|content| CollectReportFile {
                path: "saved/public-api.txt".to_string(),
                content: content.to_string(),
            })
            .collect(),
        facts: serde_json::from_str(ITEMS).expect("the served facts parse"),
        ..CollectInput::default()
    }
}

/// The JSON body of an insta snapshot, after its `---` header.
fn snapshot(text: &str) -> Value {
    let body = text.splitn(3, "---\n").nth(2).expect("a snapshot body");
    serde_json::from_str(body).expect("the snapshot parses")
}

#[specforge_test(requirement = "FEATURE-150")]
fn rust_api_declares_cargo_public_api_with_its_stdout_captured_its_filter_and_its_peers() {
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
    assert_eq!(collector.input_formats, ["public-api-text"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["Cargo.toml".to_string()][..])
    );
    assert_eq!(collector.answers, ["api_item"]);
    assert_eq!(collector.inputs, [Input::facts(&["item", "package"])]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(collector.capture.as_deref(), Some("stdout"));
    let install = collector.install.as_deref().expect("an install line");
    assert_eq!(
        install,
        "cargo install cargo-public-api --locked (and: rustup toolchain install nightly --profile minimal)"
    );
    assert!(
        specforge_protocol_types::is_install_line(install),
        "{install}"
    );
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    assert_eq!(capabilities[0]["name"], COLLECTOR);
    assert_eq!(capabilities[0]["port"], "process");
    assert_eq!(
        capabilities[0]["argv"],
        json!(["cargo-public-api", "--color", "never", "-ss"]),
        "{capabilities}"
    );
    assert_eq!(capabilities[0]["filter"], json!(["--package", "{filter}"]));
    assert_eq!(
        capabilities[0]["env"],
        json!([
            "CARGO_*",
            "RUSTUP_*",
            "RUSTC",
            "RUSTC_*",
            "RUSTFLAGS",
            "RUSTDOCFLAGS",
            "RUST_LOG"
        ])
    );
}

#[specforge_test(requirement = "FEATURE-150")]
fn a_captured_public_api_run_or_a_saved_one_mints_the_readers_facts() {
    for given in [input(Some(SNAPSHOT), &[]), input(None, &[SNAPSHOT])] {
        let out = collect(&given).unwrap();
        assert_eq!(
            serde_json::to_value(&out.facts).unwrap(),
            snapshot(SNAPSHOT_FACTS)
        );
        assert_eq!(out.warnings, Vec::<String>::new());
    }
}

#[specforge_test(requirement = "FEATURE-150")]
fn no_capture_and_no_single_report_or_a_bad_line_is_the_collectors_error() {
    for reports in [&[][..], &[SNAPSHOT, SNAPSHOT][..]] {
        let why = collect(&input(None, reports)).unwrap_err();
        assert!(
            why.contains(COLLECTOR) && why.contains("standard output was not captured"),
            "{why}"
        );
    }
    let why = collect(&input(Some("pub mod demo\ngarbage\n"), &[])).unwrap_err();
    assert!(
        why.starts_with(&format!("{CAPTURE}: ")) && why.contains("line 2"),
        "{why}"
    );
}
