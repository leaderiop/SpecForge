//! The code book of one registry build: each non-first-party extension's
//! `codes` catalog under its code namespace (ADR 0286). Two extensions that
//! share a namespace are E039; the first in load order owns it.

use specforge_common::{Diagnostic, codes};
use specforge_diagnostics::{Codebook, ExtensionCode};
use specforge_protocol_types::ExtensionDeclaration;

/// `declaration`'s `codes` as the code book reads them.
pub fn extension_codes(declaration: &ExtensionDeclaration) -> Vec<ExtensionCode> {
    declaration
        .codes
        .iter()
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
