//! A project that loads no software extension still states its rules
//! (`invariant`, governance's) and checks its formal kinds (ADR 0280).

use specforge_common::Severity;
use specforge_project::testing::TestProject;

// The regulated-process example's files: the test reads the example itself,
// so the example and this pin cannot drift apart.
const RULES: &str = include_str!("../../../../examples/regulated-process/spec/rules.spec");
const LIFECYCLE: &str = include_str!("../../../../examples/regulated-process/spec/lifecycle.spec");
const PROCEDURE: &str = include_str!("../../../../examples/regulated-process/spec/procedure.spec");

/// The `(code, message)` of every diagnostic, sorted.
fn reported(project: TestProject) -> Vec<(String, String)> {
    let compiled = project.compile();
    let mut pairs: Vec<(String, String)> = compiled
        .diagnostics()
        .iter()
        .map(|d| (d.code.clone(), d.message.clone()))
        .collect();
    pairs.sort();
    pairs
}

#[specforge_test_macros::test(
    behavior = "ge_declare_manifest",
    verify = "a project that loads governance and not software declares invariants, and W003 reports the one nothing references"
)]
#[ignore = "flipped by T7"]
fn governance_alone_declares_invariant() {
    let pairs = reported(
        TestProject::new()
            .enabling(&["@specforge/product", "@specforge/governance"])
            .file(
                "main.spec",
                concat!(
                    "invariant launch_needs_legal \"No launch without legal sign-off\" {\n",
                    "  guarantee \"A campaign goes live only after legal has signed it off.\"\n",
                    "  risk high\n}\n",
                    "invariant orphan_rule \"A rule nothing names\" {\n  guarantee \"g\"\n}\n",
                    "constraint legal_review \"Legal review\" {\n",
                    "  description \"Legal reviews every campaign.\"\n",
                    "  protects [launch_needs_legal]\n}\n",
                ),
            ),
    );
    assert!(pairs.iter().all(|(code, _)| code != "E024"), "{pairs:?}");
    let w003: Vec<&str> = pairs
        .iter()
        .filter(|(code, _)| code == "W003")
        .map(|(_, message)| message.as_str())
        .collect();
    assert_eq!(w003.len(), 1, "{pairs:?}");
    assert!(w003[0].contains("'orphan_rule'"), "{w003:?}");
}

#[specforge_test_macros::test(
    behavior = "fa_declare_manifest",
    verify = "formal loads without software: a project of invariants, constraints, properties, protocols and processes has no E027, no enhancement I004 and no W123 or W128"
)]
#[specforge_test_macros::test(
    behavior = "ge_declare_manifest",
    verify = "governance and formal load together without software"
)]
#[ignore = "flipped by T7"]
fn rules_and_lifecycles_check_without_software() {
    let pairs = reported(
        TestProject::new()
            .enabling(&["@specforge/governance", "@specforge/formal"])
            .file("rules.spec", RULES)
            .file("lifecycle.spec", LIFECYCLE),
    );
    // The residual (plan 04 §7): a root process no event joins is W134.
    let codes: Vec<&str> = pairs.iter().map(|(code, _)| code.as_str()).collect();
    assert_eq!(codes, ["W134"], "{pairs:?}");
    assert!(pairs[0].1.contains("'batch_lifecycle'"), "{pairs:?}");
}

#[specforge_test_macros::test(
    behavior = "ge_declare_manifest",
    verify = "a project that loads software, governance and formal checks its invariants as before the move"
)]
fn a_software_project_with_governance_keeps_its_rules() {
    let compiled = TestProject::new()
        .enabling(&[
            "@specforge/software",
            "@specforge/governance",
            "@specforge/formal",
        ])
        .file("rules.spec", RULES)
        .file("lifecycle.spec", LIFECYCLE)
        .file("procedure.spec", PROCEDURE)
        .compile();
    let diagnostics = compiled.diagnostics();
    assert!(
        diagnostics
            .iter()
            .all(|d| !matches!(d.severity, Severity::Error | Severity::Warning)),
        "{diagnostics:?}"
    );
}
