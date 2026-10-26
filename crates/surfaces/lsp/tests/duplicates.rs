//! A duplicate declaration (E002) as the LSP serves it: its references are tokens like any other (ADR 0176).
use specforge_lsp::answers;
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{Position, PrepareRenameResponse};

use crate::defaults::served;

const DUPLICATE: &str = "behavior a \"A\" {\n  ports [Repo]\n}\n\nbehavior a \"Again\" {\n  ports [Repo]\n}\n\n\
                         port Repo \"Repo\" {\n}\n";

#[spec(
    behavior = "prepare_rename",
    verify = "prepare rename answers on a reference in a duplicate declaration"
)]
fn prepare_rename_on_a_reference_in_a_duplicate_declaration_has_a_rename_range() {
    let served = served(DUPLICATE);
    let (state, uri) = (served.state(), served.uri("main.spec"));
    let answer = answers::prepare_rename(&state, &uri, Position::new(5, 10)).unwrap();
    let Some(PrepareRenameResponse::Range(range)) = answer else {
        panic!("{answer:?}")
    };
    assert_eq!(
        (range.start, range.end),
        (Position::new(5, 9), Position::new(5, 13))
    );
    // The duplicate's own name is not what a rename of `a` edits (the winner's is): it stays unavailable.
    assert!(
        answers::prepare_rename(&state, &uri, Position::new(4, 9))
            .unwrap()
            .is_none()
    );
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover on an entity lists the diagnostics of its duplicate declarations"
)]
fn hover_on_the_first_declaration_lists_its_duplicates_diagnostics() {
    let served = served(DUPLICATE);
    // `a` of the first declaration, line 0; the duplicate's E002 is at lines 4-6.
    let text = served.hover("main.spec", 0, 9).expect("a hover");
    assert!(text.contains("**Diagnostics** *(1)*"), "{text}");
    assert!(text.contains("**E002**"), "{text}");
}
