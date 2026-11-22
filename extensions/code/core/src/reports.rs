//! The check pass `reports` (ADR 0388): what the facts a standard report's reader minted leave unmapped, one code per
//! class, and the catalogue of those codes.

use specforge_code_facts::COVERAGE;
use specforge_extension_sdk::prelude::*;

use crate::REPORT_KINDS;

/// W104 on each `coverage` fact that is not pending and names no `item`.
pub(crate) fn reports(input: &PassInput) -> Vec<PassDiagnostic> {
    input
        .entities
        .iter()
        .filter(|e| !e.pending && REPORT_KINDS.contains(&e.kind.as_str()))
        .filter(|e| e.kind == COVERAGE.kind && !e.fields.contains_key("item"))
        .map(|fact| {
            let file = fact.fields.get("file").map_or("", String::as_str);
            PassDiagnostic::warning(
                "W104",
                format!(
                    "the coverage fact '{}' names no item: no collected item holds its lines of `{file}`",
                    fact.id
                ),
            )
            .with_entity(&fact.id)
        })
        .collect()
}

/// The codes the `reports` pass raises.
pub(crate) fn catalogue(c: &mut ContributionsBuilder) {
    c.code("W104", |d| {
        d.title("A report entry names no item").explanation(
            "A fact a report reader minted cites no item, because the report's file or line lies in no collected \
             item: a macro expansion, top-level statements, or a file no source pack collected. The entry is kept \
             with its counts, so nothing the report says is lost. Collect the source pack for those files \
             (`@rust/source`, `@javascript/source`), then collect the report again.",
        );
    });
}
