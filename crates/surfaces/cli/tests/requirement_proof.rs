//! A test that names a requirement proves it (ADR 0374 D3, D5): the gsd fixture `xtask/tests/fixtures/requirement-links`
//! collected through the chain the gate's `dogfood` runs. `rust-cargo` reads `Cargo.toml`, `rust-source` collects the
//! tests' `requirement = "…"` attributes as `names` (two stacked on one test name both), `rust-test` reads a nextest
//! JUnit report (merged from two partitions), and `@gsd/core`'s computed `proof` of each requirement answers.

use std::collections::BTreeMap;

use specforge_test::prelude::*;

use crate::support::{Cli, TestProject};

/// A copy of the fixture, `check`ed with no error and no warning.
fn tracer() -> TestProject {
    let fixture = specforge_repo_root::repo_root().join("xtask/tests/fixtures/requirement-links");
    let project = TestProject::copy_of(fixture);
    let check = Cli::on(&project).run(&["check", "--format", "json"]);
    assert_eq!(check.code, 0, "{}{}", check.stdout, check.stderr);
    let reported: Vec<String> = check
        .json()
        .as_array()
        .expect("check's diagnostics")
        .iter()
        .filter(|d| matches!(d["severity"].as_str(), Some("Error" | "Warning")))
        .map(|d| d["code"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(reported.is_empty(), "{reported:?}");
    project
}

/// The chain `dogfood` runs: collect `rust-cargo`, `rust-source`, `rust-test` over the copy's `junit.xml`, then each
/// requirement's `proof` from `export --format graph --kinds requirement`.
fn proofs(project: &TestProject) -> BTreeMap<String, String> {
    let cli = Cli::on(project);
    let report = project.root().join("junit.xml");
    let report = report.to_str().expect("a UTF-8 temp path");
    let online = ["--yes", "--effects", "online", "--format", "json"];
    for runner in [
        &["--runner", "rust-cargo"][..],
        &["--runner", "rust-source"],
        &["--runner", "rust-test", "--report", report],
    ] {
        let args: Vec<&str> = ["collect"]
            .into_iter()
            .chain(runner.iter().copied())
            .chain(online)
            .collect();
        let collect = cli.run(&args);
        assert_eq!(
            collect.code, 0,
            "{args:?}\n{}{}",
            collect.stdout, collect.stderr
        );
    }
    let export = cli.run(&["export", "--format", "graph", "--kinds", "requirement"]);
    assert_eq!(export.code, 0, "{}", export.stderr);
    export.json()["nodes"]
        .as_array()
        .expect("the export's nodes")
        .iter()
        .filter(|n| n["kind"] == "requirement")
        .map(|n| {
            let id = n["id"].as_str().unwrap_or("").to_string();
            (
                id,
                n["fields"]["proof"].as_str().unwrap_or("none").to_string(),
            )
        })
        .collect()
}

fn expected(states: &[(&str, &str)]) -> BTreeMap<String, String> {
    states
        .iter()
        .map(|(id, state)| (id.to_string(), state.to_string()))
        .collect()
}

#[specforge_test(requirement = "FEATURE-128")]
fn two_stacked_requirement_attributes_prove_both_requirements() {
    let project = tracer();
    assert_eq!(
        proofs(&project),
        expected(&[
            ("TRACER-01", "proven"),
            ("TRACER-02", "proven"),
            ("TRACER-03", "no_test"),
        ])
    );
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_failed_test_fails_the_requirement_it_names() {
    let project = tracer();
    let report = project.root().join("junit.xml");
    let passed = std::fs::read_to_string(&report).unwrap();
    let subtracts = r#"<testcase name="tests::subtracts" classname="tracer" timestamp="2026-11-17T10:00:00.003+00:00" time="0.007"/>"#;
    let failed = r#"<testcase name="tests::subtracts" classname="tracer" timestamp="2026-11-17T10:00:00.003+00:00" time="0.007">
            <failure message="thread &apos;tests::subtracts&apos; panicked at src/lib.rs:26:9" type="test failure with exit code 101">thread &apos;tests::subtracts&apos; panicked at src/lib.rs:26:9:
assertion `left == right` failed</failure>
        </testcase>"#;
    assert!(passed.contains(subtracts), "{passed}");
    std::fs::write(&report, passed.replace(subtracts, failed)).unwrap();
    assert_eq!(
        proofs(&project),
        expected(&[
            ("TRACER-01", "proven"),
            ("TRACER-02", "failed"),
            ("TRACER-03", "no_test"),
        ])
    );
}
