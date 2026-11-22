//! The kinds of the facts a standard report's reader mints (ADR 0388), and their edges.

use specforge_code_facts::COVERAGE;
use specforge_extension_sdk::prelude::*;

use crate::{referencing, ITEM_KIND};

/// Declares the report kinds and the edges their references draw.
pub(crate) fn report_kinds(c: &mut ContributionsBuilder) {
    c.kind(COVERAGE.kind, |k| {
        k.description(
            "What a coverage report (LCOV) says of one item, written by a coverage pack's collector (ADR 0388): \
             cov:<item id> for the innermost item holding the report's lines, cov:@<file> for what no item of a file \
             holds (W104)",
        )
        .collected(&[COVERAGE.scheme])
        .field("item", |f| {
            referencing(f, COVERAGE.kind, "item")
                .field_type(FieldType::Reference)
                .description("The item holding the lines; absent when no collected item does");
        })
        .field("file", |f| {
            f.field_type(FieldType::String)
                .description("The report's file, project-relative, with / separators");
        });
        let counts = [
            ("functions_found", "The functions the report lists in it"),
            ("functions_hit", "Those of its functions that ran"),
            ("lines_found", "The lines the report measured in it"),
            ("lines_hit", "Those of its lines that ran"),
            ("branches_found", "The branches the report measured in it"),
            ("branches_hit", "Those of its branches that were taken"),
        ];
        for (name, description) in counts {
            k.field(name, |f| {
                f.field_type(FieldType::Integer).description(description);
            });
        }
    });
    c.edge("covers", |e| {
        e.description("The item a coverage fact reports on")
            .source_kind(COVERAGE.kind)
            .target_kind(ITEM_KIND);
    });
}
