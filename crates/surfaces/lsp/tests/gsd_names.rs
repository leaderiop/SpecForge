//! A name holding a dash or a dot, an all-digit name and a number in a reference field, as the editor reads them
//! (ADR 0283, plan K1).

use specforge_extension_sdk::prelude::FieldType;
use specforge_lsp::answers;
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{GotoDefinitionResponse, Position, PrepareRenameResponse};

use crate::served::{Served, buffers, uri_of_path};

fn prepared(text: &str, at: Position) -> Option<(Position, Position)> {
    let state = buffers(&[("/p/gsd.spec", text)]);
    match answers::prepare_rename(&state, &uri_of_path("/p/gsd.spec"), at) {
        Ok(Some(PrepareRenameResponse::Range(r))) => Some((r.start, r.end)),
        _ => None,
    }
}

#[spec(
    behavior = "prepare_rename",
    verify = "prepare rename ranges a name holding a dash or a dot whole"
)]
fn prepare_rename_ranges_a_gsd_name_whole() {
    for (name, end) in [("AUTH-01", 12), ("v1.0", 9), ("03-02", 10)] {
        let text = format!("type {name} \"T\" {{\n}}\n");
        assert_eq!(
            prepared(&text, Position::new(0, 7)),
            Some((Position::new(0, 5), Position::new(0, end))),
            "{name}"
        );
    }
}

#[spec(
    behavior = "prepare_rename",
    verify = "an entity named with digits only is a name to the editor"
)]
fn an_all_digit_entity_name_is_a_name_to_the_editor() {
    assert_eq!(
        prepared("type 03 \"T\" {\n}\n", Position::new(0, 6)),
        Some((Position::new(0, 5), Position::new(0, 7)))
    );
}

#[spec(
    behavior = "go_to_definition",
    verify = "go-to-definition follows a number written in a field that names entities"
)]
#[ignore = "K1 T6"]
fn a_number_in_a_reference_field_navigates_to_its_entity() {
    let text = "ticket 10 \"A\" {\n}\nticket t \"B\" {\n  parent 10\n  deps [10]\n}\n";
    let served = Served::new(&[("main.spec", text)])
        .extension("@t/tickets", |c| {
            c.kind("ticket", |k| {
                k.field("parent", |f| {
                    f.field_type(FieldType::Reference);
                });
                k.field("deps", |f| {
                    f.field_type(FieldType::ReferenceList);
                });
            });
        })
        .open(&["main.spec"]);
    let uri = served.uri("main.spec");
    for at in [Position::new(3, 10), Position::new(4, 9)] {
        let found = answers::definition(&served.state(), &uri, at);
        let Some(GotoDefinitionResponse::Scalar(location)) = found else {
            panic!("{at:?}: {found:?}");
        };
        assert_eq!(location.range.start.line, 0, "{at:?}");
    }
}

#[spec(
    behavior = "hover_information",
    verify = "hover shows an integer field's value beside the word it was written as when they differ"
)]
#[ignore = "K1 T6"]
fn hover_shows_an_integer_field_value_beside_its_written_word() {
    let text = "ticket t \"B\" {\n  points 2-1\n  more 5\n}\n";
    let served = Served::new(&[("main.spec", text)])
        .extension("@t/tickets", |c| {
            c.kind("ticket", |k| {
                k.field("points", |f| {
                    f.field_type(FieldType::Integer);
                });
                k.field("more", |f| {
                    f.field_type(FieldType::Integer);
                });
            });
        })
        .open(&["main.spec"]);
    let hover = served.hover_on("main.spec", "t").expect("a hover");
    assert!(hover.contains("`points` = 1 (written 2-1)"), "{hover}");
    assert!(
        hover.contains("`more` = 5\n") || hover.ends_with("`more` = 5"),
        "{hover}"
    );
}
