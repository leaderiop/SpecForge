//! The kinds of the facts a standard report's reader mints (ADR 0388), and their edges.

use specforge_code_facts::{API_ITEM, COVERAGE};
use specforge_extension_sdk::prelude::*;

use crate::{referencing, API_KINDS, ITEM_KIND, VISIBILITY};

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
    c.kind(API_ITEM.kind, |k| {
        k.description(
            "One public entry of an API snapshot, written by an api pack's collector (ADR 0388): \
             api:rust:<crate>::<path> (cargo public-api, a trait-impl member ...::<Type as Trait>::m), \
             api:js:<package>::<name path> (api-extractor), the id holding no line, order nor source file",
        )
        .collected(&[API_ITEM.scheme])
        .field("item", |f| {
            referencing(f, API_ITEM.kind, "item")
                .field_type(FieldType::Reference)
                .description(
                    "The item the entry is the API of; absent when no collected item has its path (W104 for a kind \
                     an item can have)",
                );
        })
        .field("path", |f| {
            f.field_type(FieldType::String)
                .description("The entry's qualified path as the snapshot wrote it");
        })
        .field("kind", |f| {
            f.vocabulary(&API_KINDS).description(
                "What the entry is: an item kind, or variant, reexport, extern_crate, other, which no item is",
            );
        })
        .field("visibility", |f| {
            f.vocabulary(&VISIBILITY)
                .description("Who may name it, as the snapshot wrote it");
        })
        .field("signature", |f| {
            f.field_type(FieldType::String)
                .description("The entry exactly as the snapshot wrote it, attributes included");
        })
        .field("tag", |f| {
            f.field_type(FieldType::String)
                .description("The api-extractor release tag (Public, Beta, Alpha, Internal)");
        });
    });
    c.edge("api_of", |e| {
        e.description("The item an API snapshot's entry is the API of")
            .source_kind(API_ITEM.kind)
            .target_kind(ITEM_KIND);
    });
}
