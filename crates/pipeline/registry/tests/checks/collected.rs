//! E115 and the E014 exemption: a kind a collector writes (ADR 0327) has its entities from fact stores only.

use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_test_macros::test as spec;

use crate::support::written::checked;
use crate::support::{build, coded_in, declare};

/// `@acme/code`: the kind `test`, written by a collector under the scheme `js`.
fn code() -> ExtensionDeclaration {
    declare("@acme/code", |c| {
        c.kind("test", |k| {
            k.collected(&["js"]).field("file", |f| {
                f.field_type(FieldType::String);
            });
        });
    })
}

#[spec(
    behavior = "check_collected_kinds",
    verify = "an entity of a kind a collector writes, written in a spec file, is E115"
)]
fn an_entity_of_a_collected_kind_in_a_spec_file_is_e115() {
    let diags = checked(
        &build([code()]),
        &[("main.spec", "test t1 \"T\" {\n  file \"a.js\"\n}\n")],
    );

    let e115 = coded_in(&diags, "E115");
    assert_eq!(e115.len(), 1, "{diags:?}");
    assert_eq!(
        e115[0].message,
        "test 't1' is a fact a collector writes (@acme/code), and is not written in a spec file"
    );
    assert_eq!(
        e115[0].suggestion.as_deref(),
        Some("delete the block: `specforge collect` gives its facts")
    );
    assert_eq!(e115[0].span.as_ref().unwrap().file.as_str(), "main.spec");
}

#[spec(
    behavior = "check_entities_in_one_order",
    verify = "a fact's id is exempt from the identifier length contract"
)]
fn a_collected_id_over_sixty_characters_is_no_e014() {
    let id = "a".repeat(63);
    let text = format!("test {id} \"T\" {{\n}}\n");

    let stored = checked(
        &build([code()]),
        &[(".specforge/facts/worktree/node.json", &text)],
    );
    let written = checked(&build([code()]), &[("main.spec", &text)]);

    assert!(stored.is_empty(), "{stored:?}");
    // The exemption is the kind's: the same id in a spec file is E115, and still no E014.
    assert!(coded_in(&written, "E014").is_empty(), "{written:?}");
    assert_eq!(coded_in(&written, "E115").len(), 1, "{written:?}");
}
