//! A path is read one segment at a time, and the editor shows the full path (ADR 0284, plan K2).

use specforge_lsp::answers;
use specforge_project::testing::{SCOPED_EXTENSION, scoped_kinds};
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{
    CompletionResponse, GotoDefinitionResponse, Position, PrepareRenameResponse,
};

use crate::served::Served;

const SOURCE: &str = "\
milestone v1.0 \"One\" {
}

phase 03 \"Three\" {
  milestone v1.0
}

phase 04 \"Four\" {
  milestone v1.0
}

decision D-01 \"Choice\" {
  phase v1.0/03
}

decision D-01 \"Other choice\" {
  phase v1.0/04
}

plan 04-01 \"Plan\" {
  phase v1.0/04
  decisions [03/D-01]
}
";

fn served() -> Served {
    Served::new(&[("main.spec", SOURCE)])
        .extension(SCOPED_EXTENSION, scoped_kinds)
        .open(&["main.spec"])
}

/// The position `offset` characters into the first occurrence of `needle` on the line holding `line_has`.
fn at(line_has: &str, needle: &str, offset: usize) -> Position {
    let (n, line) = SOURCE
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains(line_has))
        .unwrap_or_else(|| panic!("no line holds {line_has}"));
    Position::new(n as u32, (line.find(needle).unwrap() + offset) as u32)
}

#[spec(
    behavior = "go_to_definition",
    verify = "go-to-definition, prepare rename and rename take a path one segment at a time"
)]
#[ignore = "K2 T9"]
fn navigation_takes_a_path_one_segment_at_a_time() {
    let served = served();
    let uri = served.uri("main.spec");
    let state = served.state();
    let line_of = |at: Position| match answers::definition(&state, &uri, at) {
        Some(GotoDefinitionResponse::Scalar(location)) => location.range.start.line,
        other => panic!("{at:?}: {other:?}"),
    };
    // `03` of `03/D-01` is the phase; `D-01` is the decision of that phase.
    assert_eq!(line_of(at("decisions [03/D-01]", "03/D-01", 1)), 3);
    assert_eq!(line_of(at("decisions [03/D-01]", "D-01", 1)), 11);
    let range = |position: Position| match answers::prepare_rename(&state, &uri, position) {
        Ok(Some(PrepareRenameResponse::Range(r))) => (r.start.character, r.end.character),
        other => panic!("{position:?}: {other:?}"),
    };
    let start = SOURCE
        .lines()
        .find(|l| l.contains("decisions [03/D-01]"))
        .unwrap()
        .find("03/")
        .unwrap() as u32;
    assert_eq!(
        range(at("decisions [03/D-01]", "03/D-01", 1)),
        (start, start + 2)
    );
    assert_eq!(
        range(at("decisions [03/D-01]", "D-01", 1)),
        (start + 3, start + 7)
    );
}

#[spec(
    behavior = "autocomplete_entity_ids",
    verify = "completion offers an entity by its name, shows its path and inserts the shortest spelling from the cursor's block"
)]
#[ignore = "K2 T9"]
fn completion_inserts_the_shortest_spelling() {
    let text = SOURCE.replace("decisions [03/D-01]", "decisions []");
    let served = Served::new(&[("main.spec", &text)])
        .extension(SCOPED_EXTENSION, scoped_kinds)
        .open(&["main.spec"]);
    let uri = served.uri("main.spec");
    let line = text
        .lines()
        .position(|l| l.contains("decisions []"))
        .unwrap() as u32;
    let col = text.lines().nth(line as usize).unwrap().find('[').unwrap() as u32 + 1;
    let Some(CompletionResponse::Array(items)) =
        answers::completion(&served.state(), &uri, Position::new(line, col))
    else {
        panic!("an array of items");
    };
    let in_phase_three = items
        .iter()
        .find(|i| {
            i.detail
                .as_deref()
                .is_some_and(|d| d.contains("v1.0/03/D-01"))
        })
        .unwrap_or_else(|| panic!("{items:?}"));
    assert_eq!(in_phase_three.label, "D-01");
    let inserted = |item: &tower_lsp::lsp_types::CompletionItem| {
        item.insert_text.clone().or_else(|| match &item.text_edit {
            Some(tower_lsp::lsp_types::CompletionTextEdit::Edit(e)) => Some(e.new_text.clone()),
            _ => None,
        })
    };
    // The cursor is in phase 04's plan: its own D-01 is `D-01`, phase 03's is `03/D-01`.
    assert_eq!(inserted(in_phase_three).as_deref(), Some("03/D-01"));
}

#[spec(
    behavior = "hover_information",
    verify = "hover and workspace symbols show an entity's full path"
)]
#[ignore = "K2 T9"]
fn hover_and_symbols_show_the_path() {
    let served = served();
    let hover = served.hover("main.spec", 11, 10).expect("a hover");
    assert!(hover.contains("v1.0/03/D-01"), "{hover}");
    let symbols = answers::workspace_symbols(&served.state(), "D-01").expect("symbols");
    let mut names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["v1.0/03/D-01", "v1.0/04/D-01"]);
}
