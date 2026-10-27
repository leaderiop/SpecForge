//! Hover, completion, navigation and semantic tokens read a block member by its path (ADR 0285, plan K3).

use specforge_lsp::{Document, MOD_REFERENCE, answers};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{Assembled, TYPED_EXTENSION, typed_kinds};
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{CompletionResponse, GotoDefinitionResponse, Position};

use crate::served::Served;

const SOURCE: &str = "\
milestone v1.0 \"One\" {
}

phase 02 \"Two\" {
  milestone v1.0
}

phase 03 \"Three\" {
  milestone v1.0
}

plan 03-02 \"Plan\" {
  phase v1.0/03
  files_modified [\"src/a.ts\"]
}

summary 03-02-S \"S\" {
  plan v1.0/03/03-02
  requires [
    { phase 02 provides \"x\" }
    { phase 03 }
  ]
}

refactor_proposal r1 \"R\" {
  reasons [threshold, ]
}
";

fn served(text: &str) -> Served {
    Served::new(&[("main.spec", text)])
        .extension(TYPED_EXTENSION, typed_kinds)
        .open(&["main.spec"])
}

/// The position `offset` characters into the first `needle` on the first line holding `line_has`.
fn at(line_has: &str, needle: &str, offset: usize) -> Position {
    let (n, line) = SOURCE
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains(line_has))
        .unwrap_or_else(|| panic!("no line holds {line_has}"));
    Position::new(n as u32, (line.find(needle).unwrap() + offset) as u32)
}

fn labels(served: &Served, at: Position) -> Vec<String> {
    let uri = served.uri("main.spec");
    match answers::completion(&served.state(), &uri, at) {
        Some(CompletionResponse::Array(items)) => items.into_iter().map(|i| i.label).collect(),
        other => panic!("{at:?}: {other:?}"),
    }
}

#[spec(
    behavior = "autocomplete_entity_ids",
    verify = "completion inside a typed block offers its unwritten members, a member's values and an enum list's unwritten words"
)]
#[ignore = "K3 T11"]
fn completion_and_hover_inside_a_typed_block() {
    let served = served(SOURCE);
    // After `{ phase 03 ` the member left to write is `provides`.
    let members = labels(&served, at("{ phase 03 }", "}", 0));
    assert!(
        members.contains(&"provides".to_string()) && !members.contains(&"phase".to_string()),
        "{members:?}"
    );
    // A reference member completes the ids of its kind.
    let phases = labels(&served, at("{ phase 03 }", "03", 0));
    assert!(
        phases.contains(&"02".to_string()) && !phases.contains(&"03-02".to_string()),
        "{phases:?}"
    );
    // An enum list completes the words it has not got.
    let words = labels(&served, at("reasons [threshold, ]", "]", 0));
    assert_eq!(words, ["jump"]);

    let hover = served
        .hover(
            "main.spec",
            at("provides", "provides", 1).line,
            at("provides", "provides", 1).character,
        )
        .expect("a hover");
    assert!(
        hover.contains("provides") && hover.contains("requires"),
        "{hover}"
    );
}

#[spec(
    behavior = "go_to_definition",
    verify = "go-to-definition and the semantic token of a reference inside a block reach the entity it names"
)]
#[ignore = "K3 T11"]
fn navigation_and_tokens_reach_a_block_member_reference() {
    let served = served(SOURCE);
    let uri = served.uri("main.spec");
    let state = served.state();
    match answers::definition(&state, &uri, at("{ phase 02 ", "02", 0)) {
        Some(GotoDefinitionResponse::Scalar(location)) => assert_eq!(location.range.start.line, 3),
        other => panic!("{other:?}"),
    }

    let project = Assembled::typed().sources(&[("main.spec", SOURCE)]);
    let view = ProjectView::of(project.compiled());
    let tokens = Document::new(SOURCE.into()).tokens(&view);
    let token = tokens
        .iter()
        .find(|t| t.line == at("{ phase 02 ", "02", 0).line && t.text == "02")
        .expect("a token for `02`");
    assert!(token.modifiers & MOD_REFERENCE != 0, "{token:?}");
}
