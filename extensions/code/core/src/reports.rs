//! The check pass `reports` (ADR 0388): what the facts a standard report's reader minted leave unmapped, one code per
//! class, and the catalogue of those codes.

use specforge_code_facts::{ItemKind, API_ITEM, COVERAGE};
use specforge_extension_sdk::prelude::*;

use crate::REPORT_KINDS;

/// W104 on each fact that is not pending and names no `item`: a `coverage`, and an `api_item` of a kind an item can
/// have (not a variant, re-export, extern crate nor `other`, and not a crate root, which no item is).
pub(crate) fn reports(input: &PassInput) -> Vec<PassDiagnostic> {
    input
        .entities
        .iter()
        .filter(|e| !e.pending && REPORT_KINDS.contains(&e.kind.as_str()))
        .filter(|e| !e.fields.contains_key("item"))
        .filter_map(|fact| {
            let field = |name: &str| fact.fields.get(name).map_or("", String::as_str);
            let message = if fact.kind == COVERAGE.kind {
                format!(
                    "the coverage fact '{}' names no item: no collected item holds its lines of `{}`",
                    fact.id,
                    field("file")
                )
            } else if fact.kind == API_ITEM.kind
                && ItemKind::NAMES.contains(&field("kind"))
                && !is_crate_root(&fact.id)
            {
                format!(
                    "the api_item '{}' names no item: no collected item has its path `{}`",
                    fact.id,
                    field("path")
                )
            } else {
                return None;
            };
            Some(PassDiagnostic::warning("W104", message).with_entity(&fact.id))
        })
        .collect()
}

/// Whether `id` is a Rust crate root's `api:rust:<crate>`: a crate is no item.
fn is_crate_root(id: &str) -> bool {
    id.strip_prefix("api:rust:")
        .is_some_and(|krate| !krate.contains("::"))
}

/// The codes the `reports` pass raises.
pub(crate) fn catalogue(c: &mut ContributionsBuilder) {
    c.code("W104", |d| {
        d.title("A report entry names no item").explanation(
            "A fact a report reader minted cites no item, because the report's file or line lies in no collected \
             item (a macro expansion, top-level statements, or a file no source pack collected), or because an API \
             snapshot's entry has a path no collected item has: a re-exported path whose definition path differs, \
             an item a macro generates, or a crate whose library is named apart from its package. The entry is \
             kept with what the report says of it, so nothing is lost. Collect the source pack for those files \
             (`@rust/source`, `@javascript/source`), then collect the report again.",
        );
    });
}
