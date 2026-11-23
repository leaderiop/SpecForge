use super::*;
use serde_json::json;
use specforge_test::prelude::*;

/// The Kani reader's one-crate output: crate `demo`'s harness `proofs::check_add`, successful.
const ONE_CRATE: &str = include_str!("../../../code/kani/fixtures/one-crate.txt");
/// Two runs: crate `ledger`'s two harnesses, then crate `vault`'s one.
const WORKSPACE: &str = include_str!("../../../code/kani/fixtures/workspace.txt");
/// Crate `demo`: `proofs::check_add` successful, `proofs::check_bound` failed.
const FAILED: &str = include_str!("../../../code/kani/fixtures/failed.txt");
/// No harness found.
const NONE: &str = include_str!("../../../code/kani/fixtures/none.txt");
/// The served items of the crates the Kani outputs name.
const ITEMS: &str = include_str!("../fixtures/items.json");

/// The input the host sends: `stdout` as captured, these report files, the fixture's items served.
fn input(stdout: Option<&str>, reports: &[&str]) -> CollectInput {
    CollectInput {
        stdout: stdout.map(ToString::to_string),
        reports: reports
            .iter()
            .map(|content| CollectReportFile {
                path: "saved/kani.txt".to_string(),
                content: content.to_string(),
            })
            .collect(),
        facts: serde_json::from_str(ITEMS).expect("the served facts parse"),
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

#[specforge_test(requirement = "FEATURE-154")]
fn rust_proof_declares_kani_with_its_stdout_captured_its_filter_and_its_peers() {
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
    assert_eq!(collector.input_formats, ["kani-regular"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["Cargo.toml".to_string()][..])
    );
    assert_eq!(collector.answers, ["test", "test_run"]);
    assert_eq!(collector.inputs, [Input::facts(&["item", "package"])]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(collector.capture.as_deref(), Some("stdout"));
    let install = collector.install.as_deref().expect("an install line");
    assert_eq!(
        install,
        "cargo install --locked kani-verifier && cargo kani setup"
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
        json!(["cargo-kani", "--workspace", "--output-format", "regular"]),
        "{capabilities}"
    );
    assert_eq!(capabilities[0]["filter"], json!(["--harness", "{filter}"]));
    assert_eq!(
        capabilities[0]["env"],
        json!([
            "CARGO_*",
            "RUSTUP_*",
            "RUSTC",
            "RUSTC_*",
            "RUSTFLAGS",
            "KANI_*"
        ])
    );
}

#[specforge_test(requirement = "FEATURE-154")]
fn a_harness_becomes_a_test_placed_at_its_item() {
    for given in [input(Some(ONE_CRATE), &[]), input(None, &[ONE_CRATE])] {
        let out = collect(&given).unwrap();
        assert_eq!(
            summary(&out),
            [
                ("test", "rust:demo#proofs::check_add", "passed"),
                ("test_run", "run:rust:demo#proofs::check_add", "passed"),
            ]
        );
        let test = &out.facts[0];
        assert_eq!(test.fields["file"], "src/lib.rs");
        assert_eq!(test.fields["binary"], "demo");
        assert_eq!(
            test.at,
            [FactLocation {
                file: "src/lib.rs".to_string(),
                start_line: 8,
                start_col: 5,
                end_line: 12,
                end_col: 6,
            }]
        );
        assert_eq!(out.facts[1].fields["duration_ms"], 390);
        assert_eq!(out.warnings, Vec::<String>::new());
    }
}

/// Kani's output of one run holding these successful harnesses.
fn run_of(harnesses: &[&str]) -> String {
    let mut text = String::new();
    for name in harnesses {
        text.push_str(&format!(
            "Checking harness {name}...\nVERIFICATION:- SUCCESSFUL\n"
        ));
    }
    let n = harnesses.len();
    text.push_str(&format!(
        "Complete - {n} successfully verified harnesses, 0 failures, {n} total.\n"
    ));
    text
}

/// A served item `id` (no place).
fn item(id: &str) -> Fact {
    Fact::new("item", id).field("file", "src/lib.rs")
}

/// The input of `stdout` with only `items` served.
fn served(stdout: &str, items: &[&str]) -> CollectInput {
    CollectInput {
        stdout: Some(stdout.to_string()),
        facts: items.iter().map(|id| item(id)).collect(),
        ..CollectInput::default()
    }
}

#[specforge_test(requirement = "FEATURE-154")]
fn each_run_of_a_workspace_is_named_by_the_crate_holding_its_harnesses() {
    let out = collect(&input(Some(WORKSPACE), &[])).unwrap();
    let tests: Vec<(&str, &str, &str)> = summary(&out)
        .into_iter()
        .filter(|(kind, _, _)| *kind == "test")
        .collect();
    assert_eq!(
        tests,
        [
            (
                "test",
                "rust:ledger#proofs::balance_never_negative",
                "passed"
            ),
            ("test", "rust:ledger#proofs::transfer_conserves", "passed"),
            ("test", "rust:vault#checks::seal_roundtrip", "passed"),
        ]
    );
    let vault = out
        .facts
        .iter()
        .find(|f| f.id == "rust:vault#checks::seal_roundtrip")
        .unwrap();
    assert_eq!(vault.fields["file"], "crates/vault/src/lib.rs");
    assert_eq!(vault.at[0].start_line, 19);
}

#[specforge_test(requirement = "FEATURE-154")]
fn the_crate_must_hold_every_harness_of_the_run() {
    let out = collect(&served(
        &run_of(&["proofs::p", "proofs::q"]),
        &[
            "rust:a::proofs::p",
            "rust:b::proofs::p",
            "rust:a::proofs::q",
        ],
    ))
    .unwrap();
    let ids: Vec<&str> = summary(&out).into_iter().map(|(_, id, _)| id).collect();
    assert_eq!(
        ids,
        [
            "rust:a#proofs::p",
            "run:rust:a#proofs::p",
            "rust:a#proofs::q",
            "run:rust:a#proofs::q"
        ]
    );
}

#[specforge_test(requirement = "FEATURE-154")]
fn a_run_no_crate_or_two_crates_hold_wholly_is_the_collectors_error() {
    let why = collect(&served(
        &run_of(&["proofs::p", "proofs::q"]),
        &["rust:a::proofs::p", "rust:b::proofs::q"],
    ))
    .unwrap_err();
    assert!(
        why.starts_with(COLLECTOR)
            && why.contains("'proofs::p', 'proofs::q'")
            && why.contains("collect @rust/source first"),
        "{why}"
    );
    let why = collect(&served(
        &run_of(&["proofs::p"]),
        &["rust:a::proofs::p", "rust:b::proofs::p"],
    ))
    .unwrap_err();
    assert!(
        why.starts_with(COLLECTOR) && why.contains("a and b") && why.contains("'proofs::p'"),
        "{why}"
    );
    let why = collect(&served(&run_of(&["proofs::p"]), &[])).unwrap_err();
    assert!(why.contains("collect @rust/source first"), "{why}");
}

#[specforge_test(requirement = "FEATURE-154")]
fn a_harness_named_with_its_crate_keeps_the_path_after_it() {
    let out = collect(&served(
        &run_of(&["demo::proofs::p"]),
        &["rust:demo::proofs::p"],
    ))
    .unwrap();
    assert_eq!(out.facts[0].id, "rust:demo#proofs::p");
    assert_eq!(out.facts[0].fields["file"], "src/lib.rs");
}

#[specforge_test(requirement = "FEATURE-154")]
fn a_failed_harness_is_a_failed_test() {
    let out = collect(&input(Some(FAILED), &[])).unwrap();
    assert_eq!(
        summary(&out),
        [
            ("test", "rust:demo#proofs::check_add", "passed"),
            ("test_run", "run:rust:demo#proofs::check_add", "passed"),
            ("test", "rust:demo#proofs::check_bound", "failed"),
            ("test_run", "run:rust:demo#proofs::check_bound", "failed"),
        ]
    );
}

#[specforge_test(requirement = "FEATURE-154")]
fn no_capture_is_the_collectors_error_and_no_harness_is_no_fact() {
    for reports in [&[][..], &[ONE_CRATE, ONE_CRATE][..]] {
        let why = collect(&input(None, reports)).unwrap_err();
        assert!(
            why.contains(COLLECTOR) && why.contains("standard output was not captured"),
            "{why}"
        );
    }
    let why = collect(&input(Some("Checking harness p...\n"), &[])).unwrap_err();
    assert!(why.starts_with(&format!("{CAPTURE}: ")), "{why}");
    for text in [NONE, ""] {
        let out = collect(&input(Some(text), &[])).unwrap();
        assert_eq!(out.facts, Vec::<Fact>::new(), "{text:?}");
    }
}
