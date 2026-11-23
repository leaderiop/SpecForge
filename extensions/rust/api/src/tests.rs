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

/// The input of a captured run printing `stdout`, the host serving `read` as the accepted files.
fn against(stdout: &str, read: Value) -> CollectInput {
    CollectInput {
        files: serde_json::from_value(json!({ ACCEPTED: read })).expect("the read parses"),
        ..input(Some(stdout), &[])
    }
}

/// A committed accepted file the host served with its text.
fn served(path: &str, text: &str) -> Value {
    json!({"path": path, "exists": true, "text": text})
}

/// Each fact's `change`, absent as `None`.
fn changes(out: &CollectOutput) -> Vec<Option<&str>> {
    out.facts
        .iter()
        .map(|f| f.fields.get("change").and_then(Value::as_str))
        .collect()
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
            ("@code/core", "^1.4", false),
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
    assert_eq!(
        collector.inputs,
        [
            Input::facts(&["item", "package"]),
            Input::files(
                ACCEPTED,
                FileReadSpec::glob("**/public-api.txt").text_up_to(4_194_304)
            )
        ]
    );
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

#[specforge_test(requirement = "FEATURE-150")]
fn no_api_item_read_is_no_fact_and_one_warning() {
    // A missing nightly toolchain: cargo public-api's error goes to stderr and stdout holds nothing.
    for given in [input(Some(""), &[]), input(None, &[""])] {
        let out = collect(&given).unwrap();
        assert_eq!(out.facts, Vec::<Fact>::new());
        assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
        assert!(
            out.warnings[0].contains("no public API item was read"),
            "{:?}",
            out.warnings
        );
    }
}

#[specforge_test(requirement = "FEATURE-150")]
fn no_accepted_file_served_is_todays_facts() {
    let out = collect(&against(SNAPSHOT, json!({"facts": []}))).unwrap();
    assert_eq!(
        serde_json::to_value(&out.facts).unwrap(),
        snapshot(SNAPSHOT_FACTS)
    );
    assert_eq!(out.warnings, Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-150")]
fn an_accepted_file_equal_to_the_run_marks_every_item_unchanged() {
    let read = json!({"facts": [served("tests/public-api.txt", SNAPSHOT)]});
    let out = collect(&against(SNAPSHOT, read)).unwrap();
    assert!(!out.facts.is_empty());
    assert!(
        changes(&out).iter().all(|c| *c == Some("unchanged")),
        "{:?}",
        changes(&out)
    );
    assert_eq!(out.warnings, Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-150")]
fn an_accepted_function_the_run_lacks_is_removed() {
    let gone = "pub const fn example_api::function(x: u32) -> u32";
    let stdout: String = SNAPSHOT
        .lines()
        .filter(|line| *line != gone)
        .map(|line| format!("{line}\n"))
        .collect();
    let read = json!({"facts": [served("tests/public-api.txt", SNAPSHOT)]});
    let out = collect(&against(&stdout, read)).unwrap();
    let removed: Vec<&Fact> = out
        .facts
        .iter()
        .filter(|f| f.fields.get("change") == Some(&json!("removed")))
        .collect();
    let [fact] = &removed[..] else {
        panic!("one removed item: {removed:?}");
    };
    assert_eq!(fact.fields["accepted_signature"], json!(gone));
    assert_eq!(
        changes(&out)
            .iter()
            .filter(|c| **c == Some("unchanged"))
            .count(),
        out.facts.len() - 1
    );
}

#[specforge_test(requirement = "FEATURE-150")]
fn an_unreadable_accepted_file_is_the_collectors_error_naming_it() {
    let read = json!({"facts": [served("tests/public-api.txt", "pub mod demo\ngarbage\n")]});
    let why = collect(&against(SNAPSHOT, read)).unwrap_err();
    assert!(
        why.starts_with("tests/public-api.txt: ") && why.contains("line 2"),
        "{why}"
    );
}

#[specforge_test(requirement = "FEATURE-150")]
fn an_accepted_file_not_served_or_of_another_crate_is_one_warning_and_no_change() {
    let unserved = json!({"path": "tests/public-api.txt", "exists": true, "bytes": 5_000_000, "error": "too_large"});
    let foreign = served(
        "other/tests/public-api.txt",
        "pub mod other_api\npub fn other_api::f()\n",
    );
    for (file, says) in [
        (unserved, "the accepted API was not read (too_large)"),
        (foreign, "names no crate this run read"),
    ] {
        let path = file["path"].as_str().unwrap().to_string();
        let out = collect(&against(SNAPSHOT, json!({ "facts": [file] }))).unwrap();
        assert_eq!(
            serde_json::to_value(&out.facts).unwrap(),
            snapshot(SNAPSHOT_FACTS)
        );
        assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
        assert!(
            out.warnings[0].starts_with(&format!("{path}: ")) && out.warnings[0].contains(says),
            "{:?}",
            out.warnings
        );
    }
}

#[specforge_test(requirement = "FEATURE-150")]
fn a_truncated_accepted_read_is_one_warning() {
    let read = json!({"facts": [served("tests/public-api.txt", SNAPSHOT)], "truncated": true});
    let out = collect(&against(SNAPSHOT, read)).unwrap();
    assert!(changes(&out).iter().all(|c| *c == Some("unchanged")));
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(
        out.warnings[0].contains("more files matched than were read"),
        "{:?}",
        out.warnings
    );
}
