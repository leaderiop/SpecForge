//! What a change the client reports asks of the project session
//! (`specforge_lsp::changes`): the plan and what applying it publishes, with
//! no client, no debounce and no stdio.

use crate::served::{Served, apply_change};
use specforge_lsp::changes::{Change, Plan};
use specforge_lsp::publish::Publication;
use specforge_lsp::{LspState, answers};
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{FileChangeType, FileEvent, Url};

const A_ALPHA: &str = "type alpha \"A\" {}\n";
const A_OMEGA: &str = "type omega \"O\" {}\n";
const B_USES_ALPHA: &str = "behavior user \"U\" {\n  types [alpha]\n}\n";
const B_USES_OMEGA: &str = "behavior user \"U\" {\n  types [omega]\n}\n";

/// `a.spec` and `b.spec`, `b` using what `a` declares, both open.
fn two_files() -> Served {
    Served::new(&[("a.spec", A_ALPHA), ("b.spec", B_USES_ALPHA)]).open(&["a.spec", "b.spec"])
}

/// The codes published for `file`, or none when the publication does not
/// name it.
fn published(served: &Served, publication: &Publication, file: &str) -> Option<Vec<String>> {
    publication.files.get(&served.uri(file)).map(|file| {
        file.diagnostics
            .iter()
            .filter_map(|d| match d.code.as_ref()? {
                tower_lsp::lsp_types::NumberOrString::String(code) => Some(code.clone()),
                tower_lsp::lsp_types::NumberOrString::Number(_) => None,
            })
            .collect()
    })
}

fn names(symbols: Option<Vec<tower_lsp::lsp_types::SymbolInformation>>) -> Vec<String> {
    symbols
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.name)
        .collect()
}

fn changed(served: &Served, file: &str, typ: FileChangeType) -> FileEvent {
    FileEvent {
        uri: served.uri(file),
        typ,
    }
}

#[spec(
    behavior = "handle_text_document_change",
    verify = "edits to several documents in one debounce window are one update and one publication"
)]
fn a_batch_of_edits_is_one_update_and_one_publication() {
    let mut served = two_files();
    // A rename's edits: the client applies both before the debounce fires.
    served.type_text("a.spec", A_OMEGA);
    served.type_text("b.spec", B_USES_OMEGA);

    let (applied, publication) = served
        .apply(Change::Edited(vec![
            served.uri("a.spec"),
            served.uri("b.spec"),
        ]))
        .expect("the buffers are open");
    assert!(applied.changed);
    let publication = publication.expect("one publication");
    let codes = published(&served, &publication, "b.spec").expect("b.spec is published");
    assert!(
        !codes.contains(&"E003".to_string()),
        "no state in which b.spec names a missing alpha: {codes:?}"
    );
    assert_eq!(
        names(answers::workspace_symbols(served.state(), "omega")),
        ["omega"]
    );
}

#[spec(
    behavior = "document_open_close",
    verify = "closing a document compiles its file from disk again, dropping its unsaved edits"
)]
fn closing_an_unsaved_buffer_reads_its_file_from_disk() {
    let mut served = two_files();
    let edit = served.edit("a.spec", A_OMEGA).expect("compiled");
    let codes = published(&served, &edit.1.expect("published"), "b.spec").unwrap();
    assert!(codes.contains(&"E003".to_string()), "{codes:?}");
    assert_eq!(
        names(answers::workspace_symbols(served.state(), "omega")),
        ["omega"]
    );

    let (applied, publication) = served.close("a.spec").expect("the close asks something");
    assert!(applied.changed);
    assert_eq!(
        names(answers::workspace_symbols(served.state(), "alpha")),
        ["alpha"]
    );
    assert!(answers::workspace_symbols(served.state(), "omega").is_none());
    let codes = published(&served, &publication.expect("published"), "b.spec").unwrap();
    assert!(!codes.contains(&"E003".to_string()), "{codes:?}");
    assert_eq!(
        std::fs::read_to_string(served.root().join("a.spec")).unwrap(),
        A_ALPHA,
        "the disk is untouched"
    );
}

#[spec(
    behavior = "document_open_close",
    verify = "closing a document compiles its file from disk again, dropping its unsaved edits"
)]
fn closing_an_unmodified_buffer_changes_nothing() {
    let mut served = two_files();
    let (applied, publication) = served.close("a.spec").expect("the close is planned");
    assert!(!applied.changed);
    assert!(publication.is_none());
    assert_eq!(
        names(answers::workspace_symbols(served.state(), "alpha")),
        ["alpha"]
    );
}

#[spec(
    behavior = "document_open_close",
    verify = "closing a document outside a project drops its file from the project"
)]
fn closing_a_buffer_outside_a_project_drops_it() {
    let mut state = LspState::new();
    let a = Url::parse("file:///test/a.spec").unwrap();
    let b = Url::parse("file:///test/b.spec").unwrap();
    state.open_document(a.as_str(), A_ALPHA);
    state.open_document(b.as_str(), B_USES_ALPHA);
    apply_change(&mut state, Change::Edited(vec![a.clone(), b.clone()]), None)
        .expect("both are open");
    assert!(state.graph().node("alpha").is_some());

    state.close_document(a.as_str());
    let (applied, publication) =
        apply_change(&mut state, Change::Closed(a), None).expect("the close is planned");
    assert!(applied.changed);
    assert!(
        state.graph().node("alpha").is_none(),
        "alpha left the project"
    );
    let file = &publication.expect("published").files[&b];
    assert!(
        file.placed.iter().any(|d| d.code == "E003"),
        "b.spec now names a missing alpha"
    );
}

#[spec(
    behavior = "shared_incremental_pipeline",
    verify = "a reload applies every open buffer again, in one update"
)]
fn a_reload_applies_every_open_buffer_in_one_update() {
    let mut served = two_files();
    // Unsaved text in both buffers, compiled.
    served.edit("a.spec", A_OMEGA);
    served.edit("b.spec", B_USES_OMEGA);

    // The configuration changes on disk: the environment reloads from disk,
    // and the buffers are still the truth for their files.
    served.write(
        "specforge.json",
        r#"{"name":"t","version":"0.2.0","extensions":[]}"#,
    );
    let event = changed(&served, "specforge.json", FileChangeType::CHANGED);
    let (applied, publication) = served
        .apply(Change::Watched(vec![event]))
        .expect("specforge.json is an input");
    assert!(applied.environment);
    assert!(served.state().graph().node("omega").is_some());
    assert!(served.state().graph().node("alpha").is_none());
    let publication = publication.expect("published");
    for file in ["a.spec", "b.spec"] {
        let codes = published(&served, &publication, file).expect("every buffer's file");
        assert!(!codes.contains(&"E003".to_string()), "{file}: {codes:?}");
    }
}

#[spec(
    behavior = "document_open_close",
    verify = "only open documents participate in incremental compilation"
)]
fn a_disk_change_to_an_open_document_is_ignored() {
    let mut served = Served::new(&[("a.spec", A_ALPHA), ("c.spec", A_OMEGA)]).open(&["a.spec"]);
    served.edit("a.spec", "type zeta \"Z\" {}\n");

    // The buffer is the truth for its file: its change on disk is nothing,
    // its deletion is not.
    served.write("a.spec", "type other \"O\" {}\n");
    let change = changed(&served, "a.spec", FileChangeType::CHANGED);
    assert!(Plan::of(Change::Watched(vec![change]), served.state()).is_none());
    let deleted = changed(&served, "a.spec", FileChangeType::DELETED);
    assert!(Plan::of(Change::Watched(vec![deleted]), served.state()).is_some());

    // A file that is not open is the disk's.
    let change = changed(&served, "c.spec", FileChangeType::CHANGED);
    served.write("c.spec", "type sigma \"S\" {}\n");
    let (applied, _) = served
        .apply(Change::Watched(vec![change]))
        .expect("c.spec is a source");
    assert!(applied.changed);
    assert!(served.state().graph().node("sigma").is_some());
}

#[test]
fn readers_keep_the_last_graph_while_the_session_is_out() {
    let mut served = two_files();
    let (rebuilding, found) = served.rebuilding(|state| {
        (
            state.rebuilding(),
            names(answers::workspace_symbols(state, "alpha")),
        )
    });
    assert!(rebuilding);
    assert_eq!(found, ["alpha"]);
    assert!(!served.state().rebuilding());
}

#[test]
fn a_served_extension_declares_the_kinds_the_project_reads() {
    let served = Served::new(&[("main.spec", "gadget widget \"W\" {}\n")])
        .extension("@test/gadgets", |c| {
            c.kind("gadget", |k| {
                k.description("a thing that is served");
            });
        })
        .open(&["main.spec"]);
    let hover = served.hover_on("main.spec", "widget").expect("a hover");
    assert!(hover.starts_with("**gadget** `widget`"), "{hover}");
}
