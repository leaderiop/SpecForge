//! What the server answers a request, decided over the LSP state alone
//! (`specforge_lsp::answers`): no client, no debounce, no stdio.

use specforge_lsp::{ClientSupport, LspState, answers};
use specforge_project::ProjectSession;
use specforge_test_macros::test as spec;
use tempfile::TempDir;
use tower_lsp::lsp_types::{
    ClientCapabilities, CompletionClientCapabilities, CompletionItemCapability,
    DocumentSymbolClientCapabilities, GotoCapability, GotoDefinitionResponse, Position,
    TextDocumentClientCapabilities, Url,
};

const LINKED: &str = "type token \"T\" {}\nbehavior login \"L\" {\n  types [token]\n}\n";

/// A state serving the project at `dir` (one file, `main.spec`, holding
/// `text`) with `main.spec` open, and its URI.
fn serving(dir: &TempDir, text: &str) -> (LspState, Url) {
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("main.spec"), text).unwrap();
    let mut state = LspState::new();
    state.set_session(ProjectSession::open(dir.path()));
    let uri = Url::from_file_path(dir.path().join("main.spec")).unwrap();
    state.open_document(uri.as_str(), text);
    (state, uri)
}

fn declaring(definition: bool, hierarchical: bool, insert_replace: bool) -> ClientCapabilities {
    ClientCapabilities {
        text_document: Some(TextDocumentClientCapabilities {
            definition: Some(GotoCapability {
                link_support: Some(definition),
                ..Default::default()
            }),
            document_symbol: Some(DocumentSymbolClientCapabilities {
                hierarchical_document_symbol_support: Some(hierarchical),
                ..Default::default()
            }),
            completion: Some(CompletionClientCapabilities {
                completion_item: Some(CompletionItemCapability {
                    insert_replace_support: Some(insert_replace),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[spec(
    behavior = "go_to_definition",
    verify = "a definition is a location link for a client that declares linkSupport, else a location at the name"
)]
fn client_support_reads_what_the_client_declared() {
    assert_eq!(
        ClientSupport::of(&ClientCapabilities::default()),
        ClientSupport::default()
    );
    for (flags, expected) in [
        ((true, false, false), (true, false, false)),
        ((false, true, false), (false, true, false)),
        ((false, false, true), (false, false, true)),
    ] {
        let support = ClientSupport::of(&declaring(flags.0, flags.1, flags.2));
        assert_eq!(
            (
                support.definition_links,
                support.hierarchical_symbols,
                support.insert_replace
            ),
            expected
        );
    }

    let dir = TempDir::new().unwrap();
    let (mut state, uri) = serving(&dir, LINKED);
    let cursor = Position::new(2, 10);

    state.set_client(ClientSupport::of(&declaring(true, false, false)));
    let Some(GotoDefinitionResponse::Link(links)) = answers::definition(&state, &uri, cursor)
    else {
        panic!("a client declaring linkSupport gets a link");
    };
    let range = |r: tower_lsp::lsp_types::Range| {
        (r.start.line, r.start.character, r.end.line, r.end.character)
    };
    assert_eq!(links.len(), 1);
    assert_eq!(range(links[0].target_range), (0, 0, 0, 17));
    assert_eq!(range(links[0].target_selection_range), (0, 5, 0, 10));
    assert_eq!(
        links[0].origin_selection_range.map(range),
        Some((2, 9, 2, 14))
    );

    state.set_client(ClientSupport::of(&declaring(false, false, false)));
    let Some(GotoDefinitionResponse::Scalar(location)) = answers::definition(&state, &uri, cursor)
    else {
        panic!("any other client gets a location");
    };
    assert_eq!(location.uri, uri);
    assert_eq!(range(location.range), (0, 5, 0, 10));
}
