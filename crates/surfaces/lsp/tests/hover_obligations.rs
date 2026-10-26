//! The hover lists an entity's `verify` statements apart from its fields (ADR 0094).

use specforge_test_macros::test as spec;

use crate::hover::coverage_project;

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover lists the entity's obligations apart from its fields"
)]
fn hover_lists_obligations_apart_from_fields() {
    let project = coverage_project(
        "type Payload \"P\" {\n  verify string @optional\n  verify unit \"payload is valid\"\n}\n",
    );
    let hover = project.hover_on("main.spec", "Payload").unwrap();
    assert!(hover.contains("- `verify` = `string`"), "{hover}");
    assert!(
        hover.contains("**Obligations**\n\n| # | Kind | Obligation |\n|---|---|---|\n| 1 | unit | payload is valid |"),
        "{hover}"
    );
    assert!(!hover.contains("`verify` = unit"), "{hover}");
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover lists each obligation once, its kind apart from its text, whatever the text holds"
)]
fn hover_lists_each_obligation_once_whatever_its_text() {
    let project = coverage_project(
        "behavior b5 \"B5\" {\n  contract \"c\"\n  verify unit \"x\"\n  verify \"unit: x\"\n  verify unit \"first line\\nsecond\"\n}\n",
    );
    let hover = project.hover_on("main.spec", "b5").unwrap();
    let want = "**Obligations**\n\n| # | Kind | Obligation |\n|---|---|---|\n| 1 | unit | x |\n| 2 | — | unit: x |\n| 3 | unit | first line second |";
    assert!(hover.contains(want), "{hover}");
}
