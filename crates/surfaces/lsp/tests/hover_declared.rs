//! The hover of an extension's declared code shows its rule's description (ADR 0278).

use specforge_common::Diagnostic;
use specforge_extension_sdk::prelude::*;
use specforge_lsp::hover::diagnostics;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_test_macros::test as spec;

/// `@acme/launch` declaring E903 on `campaign`, described when `description` is.
fn launch(description: Option<&str>) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/launch", "0.1.0"));
    c.rule("E903", |r| {
        r.check(CheckKind::MissingRequiredField)
            .target_kind("campaign")
            .field("ends")
            .severity(ValidationSeverity::Error)
            .message_template("campaign '{id}' has no end");
        if let Some(text) = description {
            r.description(text);
        }
    });
    c.declaration()
}

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
fn an_extensions_declared_code_hover_shows_its_description() {
    let described = [launch(Some("A campaign that ends before it starts."))];
    let ours = reported_by("@acme/launch");
    assert_eq!(
        diagnostics(&[&ours], &described).as_deref(),
        Some(
            "**E903** \u{b7} reported by '@acme/launch'\n\ncampaign 'spring' has no end\n\nA campaign that ends before it starts."
        )
    );
    let theirs = reported_by("@acme/other");
    assert_eq!(
        diagnostics(&[&theirs], &described).as_deref(),
        Some("**E903** \u{b7} reported by '@acme/other'\n\ncampaign 'spring' has no end")
    );
    assert_eq!(
        diagnostics(&[&ours], &[launch(None)]).as_deref(),
        Some("**E903** \u{b7} reported by '@acme/launch'\n\ncampaign 'spring' has no end")
    );
}
