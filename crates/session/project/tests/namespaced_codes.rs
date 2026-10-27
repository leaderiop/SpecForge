//! A pass finding's code is its extension's namespace and its own local code (K10).

use specforge_extension_sdk::prelude::*;
use specforge_test::prelude::*;

fn findings(
    extension: &str,
    code: &str,
    severity: PassSeverity,
) -> Vec<specforge_common::Diagnostic> {
    let output = PassOutput {
        diagnostics: vec![PassDiagnostic::new(code, severity, "x")],
        summary: Default::default(),
    };
    specforge_project::passes::pass_findings(
        extension,
        "audit",
        output,
        &specforge_registry::EntitySnapshot::default(),
    )
}

// Pins K10-D1: today the finding prints as `W950`; T5 un-ignores it.
#[specforge_test(
    invariant = "diagnostic_code_uniqueness",
    verify = "a third-party extension's code is its namespace and its own local code"
)]
#[ignore = "K10 T5"]
fn a_pass_finding_names_its_extensions_namespace() {
    let found = findings("@acme/x", "W950", PassSeverity::Warning);
    assert_eq!(found[0].code, "acme.x/W950");
}

// Pins K10-D4: today the squatter's finding prints as core's `E001`; T5 un-ignores it.
#[specforge_test(
    invariant = "diagnostic_code_uniqueness",
    verify = "an extension's code never prints as a core code or another extension's"
)]
#[ignore = "K10 T5"]
fn a_squatted_core_code_is_not_the_core_code() {
    let found = findings("@acme/squat", "E001", PassSeverity::Error);
    assert!(found.iter().all(|d| d.code != "E001"), "{found:?}");
}
