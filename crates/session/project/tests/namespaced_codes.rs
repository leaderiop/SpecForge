//! A pass finding's code is its extension's namespace and its own local code (K10).

use specforge_extension_sdk::prelude::*;
use specforge_test::prelude::*;

fn findings(
    extension: &str,
    code: &str,
    severity: PassSeverity,
) -> Vec<specforge_common::Diagnostic> {
    let mut book = specforge_diagnostics::Codebook::default();
    // The extension catalogues the code it reports (a first-party one has none to catalogue).
    book.add(
        extension,
        vec![specforge_diagnostics::ExtensionCode {
            local: code.to_string(),
            title: "t".to_string(),
            explanation: "e".to_string(),
            docs: None,
        }],
    )
    .unwrap();
    let output = PassOutput {
        diagnostics: vec![PassDiagnostic::new(code, severity, "x")],
        summary: Default::default(),
    };
    specforge_project::passes::pass_findings(
        extension,
        "audit",
        output,
        &specforge_registry::EntitySnapshot::default(),
        &book,
    )
}

#[specforge_test(
    invariant = "diagnostic_code_uniqueness",
    verify = "a third-party extension's code is its namespace and its own local code"
)]
fn a_pass_finding_names_its_extensions_namespace() {
    let found = findings("@acme/x", "W950", PassSeverity::Warning);
    assert_eq!(found[0].code, "acme.x/W950");
}

#[specforge_test(
    invariant = "diagnostic_code_uniqueness",
    verify = "an extension's code never prints as a core code or another extension's"
)]
fn a_squatted_core_code_is_not_the_core_code() {
    let found = findings("@acme/squat", "E001", PassSeverity::Error);
    assert!(found.iter().all(|d| d.code != "E001"), "{found:?}");
}
