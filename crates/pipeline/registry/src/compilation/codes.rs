//! The code book of one registry build: each non-first-party extension's
//! `codes` catalog under its code namespace (ADR 0286). Two extensions that
//! share a namespace are E039; the first in load order owns it.

use specforge_common::{Diagnostic, codes};
use specforge_diagnostics::{Codebook, ExtensionCode, is_local_code};
use specforge_protocol_types::ExtensionDeclaration;

/// Whether `docs` is an absolute `http(s)` URL: the only link a surface may print.
pub(crate) fn is_http_url(docs: &str) -> bool {
    ["https://", "http://"].iter().any(|scheme| {
        docs.strip_prefix(scheme)
            .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
    })
}

/// `declaration`'s `codes` as the code book reads them: an entry E030 refuses (not a local
/// code, an empty title or explanation, a `docs` that is no http(s) URL, a code already
/// catalogued: the first wins) never reaches it, so no surface prints what the load refused.
pub fn extension_codes(declaration: &ExtensionDeclaration) -> Vec<ExtensionCode> {
    let mut seen = std::collections::HashSet::new();
    declaration
        .codes
        .iter()
        .filter(|entry| {
            is_local_code(&entry.code)
                && !entry.title.trim().is_empty()
                && !entry.explanation.trim().is_empty()
                && entry.docs.as_deref().is_none_or(is_http_url)
                && seen.insert(entry.code.to_uppercase())
        })
        .map(|entry| ExtensionCode {
            local: entry.code.clone(),
            title: entry.title.clone(),
            explanation: entry.explanation.clone(),
            docs: entry.docs.clone(),
        })
        .collect()
}

/// The code book of `declarations` (load order), and one E039 per extension
/// whose namespace an earlier extension owns.
pub(crate) fn build(declarations: &[ExtensionDeclaration]) -> (Codebook, Vec<Diagnostic>) {
    let mut book = Codebook::default();
    let mut diagnostics = Vec::new();
    for declaration in declarations {
        if let Err(shared) = book.add(declaration.name(), extension_codes(declaration)) {
            diagnostics.push(
                Diagnostic::new(
                    codes::E039,
                    format!(
                        "extensions '{}' and '{}' share the code namespace '{}'; '{}' owns it, and the codes of '{}' are described by nobody",
                        shared.owner, shared.other, shared.namespace, shared.owner, shared.other
                    ),
                )
                .with_suggestion(
                    "a code namespace is the package name's scope and name (`@gsd/core` is `gsd`, \
                     `@a/b.c` is `a.b.c`): rename one of the packages, or load only one of them",
                ),
            );
        }
    }
    (book, diagnostics)
}
