//! The hover of an extension's declared code shows its rule's description (ADR 0278).

use specforge_common::Diagnostic;
use specforge_lsp::hover::diagnostics;
use specforge_test_macros::test as spec;

/// An E903 diagnostic reported by `origin`.
fn reported_by(origin: &str) -> Diagnostic {
    let mut diag = Diagnostic::untyped(
        "E903",
        specforge_common::Severity::Error,
        "campaign 'spring' has no end",
    );
    diag.origin = Some(origin.to_string());
    diag
}

#[spec(
    behavior = "hover_diagnostic",
    verify = "an extension's declared code shows its rule's description in the hover"
)]
#[ignore = "K10 T6"]
fn an_extensions_declared_code_hover_shows_its_description() {
    // The description moved to the extension's `codes` catalog (ADR 0286): T6 reads it there.
    let ours = reported_by("@acme/launch");
    assert_eq!(
        diagnostics(&[&ours]).as_deref(),
        Some(
            "**E903** \u{b7} reported by '@acme/launch'\n\ncampaign 'spring' has no end\n\nA campaign that ends before it starts."
        )
    );
}

// Pins K10-D2: today an extension's code shows only the extension; T6 reads its catalog.
#[spec(
    behavior = "hover_diagnostic",
    verify = "a third-party code is titled, explained and linked from its own catalog"
)]
#[ignore = "K10 T6"]
fn hover_explains_a_third_party_code() {
    let mut diag = Diagnostic::untyped(
        "sdk.greet/E901",
        specforge_common::Severity::Error,
        "no greeting",
    );
    diag.origin = Some("@sdk/greet".to_string());
    // T6 passes the code book (`@sdk/greet` cataloguing E901 as "Greeting missing", "Add a greeting.",
    // https://greet.dev/codes#e901) where today's hover takes only the diagnostics.
    assert_eq!(
        diagnostics(&[&diag]).as_deref(),
        Some(
            "**sdk.greet/E901** \u{b7} Greeting missing\n\nno greeting\n\nAdd a greeting.\n\n[Documentation](https://greet.dev/codes#e901)"
        )
    );
}
