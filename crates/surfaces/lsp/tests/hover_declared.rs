//! The hover of a third-party code shows what its extension's own catalog says of it (ADR 0286).

use specforge_common::{Codebook, Diagnostic};
use specforge_diagnostics::ExtensionCode;
use specforge_lsp::hover::diagnostics;
use specforge_test_macros::test as spec;

#[spec(
    behavior = "hover_diagnostic",
    verify = "a third-party code is titled, explained and linked from its own catalog"
)]
fn hover_explains_a_third_party_code() {
    let mut codes = Codebook::default();
    codes
        .add(
            "@sdk/greet",
            vec![ExtensionCode {
                local: "E901".into(),
                title: "Greeting missing".into(),
                explanation: "Add a greeting.".into(),
                docs: Some("https://greet.dev/codes#e901".into()),
            }],
        )
        .unwrap();
    let mut diag = Diagnostic::untyped(
        "sdk.greet/E901",
        specforge_common::Severity::Error,
        "no greeting",
    );
    diag.origin = Some("@sdk/greet".to_string());
    assert_eq!(
        diagnostics(&[&diag], &codes).as_deref(),
        Some(
            "**sdk.greet/E901** \u{b7} Greeting missing\n\nno greeting\n\nAdd a greeting.\n\n[Documentation](https://greet.dev/codes#e901)"
        )
    );
    // The same code from another extension, or no catalog entry: the extension is named instead.
    diag.origin = Some("@acme/squat".to_string());
    assert_eq!(
        diagnostics(&[&diag], &codes).as_deref(),
        Some("**sdk.greet/E901** \u{b7} reported by '@acme/squat'\n\nno greeting")
    );
}
