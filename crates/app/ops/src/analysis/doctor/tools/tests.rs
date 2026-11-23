use super::*;
use crate::analysis::doctor::{DoctorReport, Part, diagnose};
use crate::testing::{ScriptedSolver, Viewed};
use specforge_effects::testing::FakeProcess;
use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
use specforge_project::testing::Assembled;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_test_macros::test as specforge_test;

/// `extension` with one collector `name` detected on `Cargo.toml`, running `run`, answering `answers`.
fn runner(
    extension: &str,
    name: &str,
    run: &[&str],
    answers: &[&str],
    install: Option<&str>,
) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(extension, "1.0.0"));
    c.collector(name, |collector| {
        collector
            .detect_files(&["Cargo.toml"])
            .run(run)
            .answers(answers);
        if let Some(line) = install {
            collector.install(line);
        }
        collector.collect(|_| Ok(Default::default()));
    });
    c.declaration()
}

/// `@acme/core`, cataloguing codes judging collected kinds: W104 coverage, W108 proof.
fn judge() -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/core", "1.0.0"));
    c.code("W104", |d| {
        d.title("Coverage on no item")
            .explanation("Serve the item.")
            .facts(&["coverage"]);
    });
    c.code("W108", |d| {
        d.title("Proof on no item")
            .explanation("Serve the item.")
            .facts(&["proof"]);
    });
    c.declaration()
}

fn cov() -> ExtensionDeclaration {
    runner(
        "@acme/cov",
        "cov",
        &["cargo-llvm-cov", "llvm-cov"],
        &["coverage"],
        Some("cargo install cargo-llvm-cov --locked"),
    )
}

/// The report of a project loading `declarations`, with a `Cargo.toml` when `cargo`, diagnosed under `process`.
fn diagnosed(
    declarations: Vec<ExtensionDeclaration>,
    cargo: bool,
    process: &FakeProcess,
) -> DoctorReport {
    let fixture = Assembled::new().declarations(declarations);
    if cargo {
        fixture.write("Cargo.toml", "");
    }
    let view = fixture.view();
    diagnose(&view, &ScriptedSolver::ready()).with_tools(&view, process)
}

fn tools(report: &DoctorReport) -> Vec<&Finding> {
    report.about(Part::Tools).collect()
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_missing_program_is_one_info_finding_naming_its_collectors_and_codes() {
    let report = diagnosed(vec![cov(), judge()], true, &FakeProcess::new());

    let found = tools(&report);
    assert_eq!(found.len(), 1, "{found:?}");
    let finding = found[0];
    assert_eq!(finding.status, FindingStatus::Info);
    assert_eq!(finding.code, "tool_missing");
    let About::Tools {
        program,
        collectors,
        codes,
    } = &finding.about
    else {
        panic!("{:?}", finding.about);
    };
    assert_eq!(program, "cargo-llvm-cov");
    assert_eq!(collectors, &["cov (@acme/cov)"]);
    assert_eq!(codes, &["acme/W104"]);
    assert!(
        finding.check.contains("`cargo-llvm-cov`"),
        "{}",
        finding.check
    );
    assert!(finding.check.contains(&codes[0]), "{}", finding.check);
    assert_eq!(finding.remediation, "cargo install cargo-llvm-cov --locked");
    assert!(report.ok(), "a missing tool never fails doctor");
    assert!(report.z3_available(), "a tool note is no toolchain finding");
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_program_on_path_says_nothing() {
    let process = FakeProcess::new().on_path("cargo-llvm-cov", "/usr/bin/cargo-llvm-cov");

    let report = diagnosed(vec![cov(), judge()], true, &process);

    assert!(tools(&report).is_empty(), "{:?}", tools(&report));
    assert!(process.requests().is_empty(), "doctor runs nothing");
}

#[specforge_test(requirement = "FEATURE-021")]
fn a_collector_not_detected_here_is_not_probed() {
    let report = diagnosed(vec![cov(), judge()], false, &FakeProcess::new());

    assert!(tools(&report).is_empty(), "{:?}", tools(&report));
}

#[specforge_test(requirement = "FEATURE-021")]
fn the_document_carries_the_tool_finding_and_its_fields() {
    let report = diagnosed(vec![cov(), judge()], true, &FakeProcess::new());

    let document = serde_json::to_value(report.document()).unwrap();
    let finding = document["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["about"] == "tools")
        .expect("a tools finding");
    assert_eq!(finding["program"], "cargo-llvm-cov");
    assert_eq!(
        finding["collectors"],
        serde_json::json!(["cov (@acme/cov)"])
    );
    assert_eq!(finding["codes"].as_array().unwrap().len(), 1);
    assert_eq!(finding["status"], "info");
}
