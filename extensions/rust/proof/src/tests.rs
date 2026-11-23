use super::*;
use serde_json::json;
use specforge_test::prelude::*;

/// The Kani reader's one-crate output: crate `demo`'s harness `proofs::check_add`, successful.
const ONE_CRATE: &str = include_str!("../../../code/kani/fixtures/one-crate.txt");
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
