//! The runner extensions' collectors, read through the protocol from
//! their real Wasm blobs and dispatched by the host's collect flow.

use specforge_ops::collect::{ReportFile, collectors, dispatch};
use specforge_registry::ManifestV2;
use specforge_wasm::protocol::{declaration_to_manifest, load_declaration};

/// A Wasm runtime for a temp project enabling `ext_names`.
fn wasm_runtime_for(ext_names: &[&str]) -> specforge_component::ComponentRuntime {
    let dir = tempfile::TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "test-project",
        "version": "0.1.0",
        "extensions": ext_names,
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    specforge_component::project_runtime(dir.path())
}

/// An extension's manifest, loaded through the protocol.
fn load_via_protocol(ext_name: &str) -> ManifestV2 {
    let runtime = wasm_runtime_for(&[ext_name]);
    declaration_to_manifest(&load_declaration(&runtime, ext_name).unwrap().declaration)
}

#[specforge_test_macros::test(
    behavior = "vt_declare_vitest_collector",
    verify = "vitest declares its collector"
)]
fn vitest_declares_its_collector() {
    let manifest = load_via_protocol("@specforge/vitest");
    assert!(
        manifest
            .peer_dependencies
            .iter()
            .any(|p| p.name == "@specforge/testing" && !p.optional),
        "testing is a required peer"
    );
    let collectors = collectors(std::slice::from_ref(&manifest));
    assert_eq!(collectors.len(), 1);
    let c = &collectors[0];
    assert_eq!(c.name, "vitest");
    assert_eq!(c.export, "collect__vitest");
    assert_eq!(c.detect, ["vitest.config.*", "vitest.workspace.*"]);
    assert_eq!(&c.run[..3], ["npx", "--no", "vitest"]);
    assert!(c.run.iter().any(|a| a == "--outputFile.json={report}"));
    assert_eq!(c.report, ".specforge/reports/vitest.json");
}

#[specforge_test_macros::test(
    behavior = "ct_declare_cargo_collector",
    verify = "cargo-test declares its collector"
)]
fn cargo_test_declares_its_collector() {
    let manifest = load_via_protocol("@specforge/cargo-test");
    assert!(
        manifest
            .peer_dependencies
            .iter()
            .any(|p| p.name == "@specforge/testing" && !p.optional),
        "testing is a required peer"
    );
    let collectors = collectors(std::slice::from_ref(&manifest));
    assert_eq!(collectors.len(), 1);
    let c = &collectors[0];
    assert_eq!(c.name, "cargo-test");
    assert_eq!(c.export, "collect__cargo_test");
    assert_eq!(c.detect, ["Cargo.toml"]);
    assert_eq!(c.run, ["cargo", "test", "--workspace", "--no-fail-fast"]);
    assert_eq!(c.report, "target/specforge");
    assert_eq!(
        c.capture.as_deref(),
        Some("stdout"),
        "plain tests only appear in libtest's output"
    );
}

#[specforge_test_macros::test(
    behavior = "ct_report_unlinked_tests",
    verify = "tests the attribute did not record are reported as unlinked"
)]
fn cargo_test_reports_plain_tests_as_unlinked() {
    let runtime = wasm_runtime_for(&["@specforge/cargo-test"]);
    let manifest = load_via_protocol("@specforge/cargo-test");
    let collector = &collectors(std::slice::from_ref(&manifest))[0];
    let stdout = include_str!("../../../extensions/cargo-test/tests/fixtures/libtest-stdout.txt");
    let report = ReportFile {
        path: "target/specforge/shop_lib.json".into(),
        content: r#"{"entries":[{"entity_id":"cart","test_name":"panics",
            "module_path":"shop_lib::tests","file":"src/lib.rs","status":"pass"}]}"#
            .into(),
    };
    let out = dispatch(&runtime, collector, &[report], Some(stdout)).unwrap();
    assert_eq!(out.entity_results.len(), 1, "the attribute's own result");
    let unlinked: Vec<(&str, &str)> = out
        .unlinked
        .iter()
        .map(|t| (t.name.as_str(), t.status.as_str()))
        .collect();
    assert_eq!(
        unlinked,
        [
            ("tests::plain_ignored", "skipped"),
            ("tests::slow_one", "skipped"),
            ("tests::add_item::rejects_a_duplicate_item", "passed"),
            ("tests::add_item__rejects_an_empty_name", "passed"),
            ("tests::fails", "failed"),
            ("cart__is_empty_at_first", "passed"),
        ],
        "doc tests, the attribute's test and replayed failure output are left out"
    );
    assert_eq!(
        out.unlinked[2].path,
        ["tests", "add_item", "rejects_a_duplicate_item"]
    );
}
