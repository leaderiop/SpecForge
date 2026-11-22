//! The kinds of the facts a standard report's reader mints (ADR 0388), and their edges.

use specforge_code_facts::{API_ITEM, COVERAGE, DEPENDENCY, STATIC_FINDING};
use specforge_extension_sdk::prelude::*;

use crate::{referencing, API_KINDS, ITEM_KIND, LEVELS, PACKAGE_KIND, SCOPES, VISIBILITY};

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
    c.kind(STATIC_FINDING.kind, |k| {
        k.description(
            "One result of a SARIF run (a lint or static analysis finding), written by a lint pack's collector \
             (ADR 0388): sarif:<tool>/<rule>#<file>:<line>, #<file> with no line, neither with no location, ~2 on \
             a repeat in document order",
        )
        .collected(&[STATIC_FINDING.scheme])
        .field("tool", |f| {
            f.field_type(FieldType::String)
                .description("The run's tool (tool.driver.name)");
        })
        .field("rule", |f| {
            f.field_type(FieldType::String)
                .description("The rule id the result states; unknown when it states none (W105)");
        })
        .field("level", |f| {
            f.vocabulary(&LEVELS).description(
                "The level the result or its rule's defaultConfiguration states; unknown when neither does (W105), \
                 never SARIF's own warning default",
            );
        })
        .field("message", |f| {
            f.field_type(FieldType::String)
                .description("The result's message.text");
        })
        .field("file", |f| {
            f.field_type(FieldType::String)
                .description("The file of its first location, project-relative, with / separators");
        })
        .field("line", |f| {
            f.field_type(FieldType::Integer)
                .description("The 1-based start line of its first location");
        })
        .field("item", |f| {
            referencing(f, STATIC_FINDING.kind, "item")
                .field_type(FieldType::Reference)
                .description("The innermost item holding its line; absent when no collected item does");
        })
        .field("package", |f| {
            referencing(f, STATIC_FINDING.kind, "package")
                .field_type(FieldType::Reference)
                .description(
                    "The package holding its file, when no item holds its line; absent with neither (W104)",
                );
        });
    });
    c.edge("found_in", |e| {
        e.description("The item a static finding lies in")
            .source_kind(STATIC_FINDING.kind)
            .target_kind(ITEM_KIND);
    });
    c.edge("found_in_package", |e| {
        e.description("The package a static finding lies in, when no item holds it")
            .source_kind(STATIC_FINDING.kind)
            .target_kind(PACKAGE_KIND);
    });
    c.kind(DEPENDENCY.kind, |k| {
        k.description(
            "One component of a CycloneDX bill of materials, written by an SBOM pack's collector (ADR 0388): \
             dep:<package id>/<purl without qualifiers and subpath>, or dep:<package id>/@<[group/]name[@version]> \
             for a component with no purl (W106); one fact per id, a purl listed twice merged",
        )
        .collected(&[DEPENDENCY.scheme])
        .field("package", |f| {
            referencing(f, DEPENDENCY.kind, "package")
                .field_type(FieldType::Reference)
                .description("The project package the BOM describes (its metadata.component)");
        })
        .field("purl", |f| {
            f.field_type(FieldType::String)
                .description("The component's package URL as written; absent when it has none (W106)");
        })
        .field("name", |f| {
            f.field_type(FieldType::String)
                .description("The component's name, group/name when it states a group");
        })
        .field("version", |f| {
            f.field_type(FieldType::String)
                .description("The component's version, when it states one");
        })
        .field("scope", |f| {
            f.vocabulary(&SCOPES).description(
                "The component's scope, required when the BOM states none (the spec's default); the widest of a \
                 purl listed twice",
            );
        })
        .field("direct", |f| {
            f.field_type(FieldType::Bool).description(
                "Whether the project package depends on it directly; present only when the BOM states a \
                 dependency graph for its root, never false for a graph it does not state",
            );
        });
    });
    c.edge("dependency_of", |e| {
        e.description("The project package a dependency is a component of the BOM of")
            .source_kind(DEPENDENCY.kind)
            .target_kind(PACKAGE_KIND);
    });
}
