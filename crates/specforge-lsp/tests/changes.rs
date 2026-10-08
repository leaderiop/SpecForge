//! What a change the client reports asks of the project session
//! (`specforge_lsp::changes`): the plan and what applying it publishes, with
//! no client, no debounce and no stdio.

use crate::recorder::{Sent, codes, last_codes, logs, publications};
use crate::served::Served;
use specforge_lsp::answers;
use specforge_lsp::changes::{Change, Plan};
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{FileChangeType, FileEvent, Url};

const A_ALPHA: &str = "type alpha \"A\" {}\n";
const A_OMEGA: &str = "type omega \"O\" {}\n";
const B_USES_ALPHA: &str = "behavior user \"U\" {\n  types [alpha]\n}\n";
const A_DANGLING: &str = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n";
const B_PLAIN: &str = "type other \"O\" {}\n";
const B_USES_OMEGA: &str = "behavior user \"U\" {\n  types [omega]\n}\n";

/// `a.spec` and `b.spec`, `b` using what `a` declares, both open.
fn two_files() -> Served {
    Served::new(&[("a.spec", A_ALPHA), ("b.spec", B_USES_ALPHA)]).open(&["a.spec", "b.spec"])
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

    let applied = served
        .apply(Change::Edited(vec![
            served.uri("a.spec"),
            served.uri("b.spec"),
        ]))
        .expect("the buffers are open");
    assert!(applied.changed);
    let sent = served.sent();
    let b = served.uri("b.spec");
    assert_eq!(publications(&sent, &b).len(), 1, "one publication");
    let codes = last_codes(&sent, &b).expect("b.spec is published");
    assert!(
        !codes.contains(&"E003".to_string()),
        "no state in which b.spec names a missing alpha: {codes:?}"
    );
    assert_eq!(
        names(answers::workspace_symbols(&served.state(), "omega")),
        ["omega"]
    );
}

#[spec(
    behavior = "document_open_close",
    verify = "closing a document compiles its file from disk again, dropping its unsaved edits"
)]
fn closing_an_unsaved_buffer_reads_its_file_from_disk() {
    let mut served = two_files();
    let b = served.uri("b.spec");
    served.edit("a.spec", A_OMEGA).expect("compiled");
    let codes = last_codes(&served.sent(), &b).expect("published");
    assert!(codes.contains(&"E003".to_string()), "{codes:?}");
    assert_eq!(
        names(answers::workspace_symbols(&served.state(), "omega")),
        ["omega"]
    );

    let applied = served.close("a.spec").expect("the close asks something");
    assert!(applied.changed);
    assert_eq!(
        names(answers::workspace_symbols(&served.state(), "alpha")),
        ["alpha"]
    );
    assert!(answers::workspace_symbols(&served.state(), "omega").is_none());
    let codes = last_codes(&served.sent(), &b).expect("published");
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
fn closing_an_unmodified_buffer_publishes_its_file_once() {
    let mut served = two_files();
    let a = served.uri("a.spec");
    let applied = served.close("a.spec").expect("the close is planned");
    assert!(!applied.changed);
    let sent = served.sent();
    assert_eq!(publications(&sent, &a), [vec![]], "a.spec has no errors");
    assert_eq!(
        names(answers::workspace_symbols(&served.state(), "alpha")),
        ["alpha"]
    );
}

#[spec(
    behavior = "document_open_close",
    verify = "closing a project source publishes what the project reports for its file"
)]
fn closing_a_clean_source_keeps_its_errors_published() {
    let mut served = Served::new(&[("a.spec", A_DANGLING), ("b.spec", B_PLAIN)]).open(&["a.spec"]);
    served.sent();
    let a = served.uri("a.spec");

    let applied = served.close("a.spec").expect("the close is planned");
    assert!(!applied.changed, "the disk text is the compiled text");
    let sent = served.sent();
    let published = publications(&sent, &a);
    assert_eq!(published.len(), 1, "published once: {sent:?}");
    assert_eq!(codes(&published[0]), ["E003"]);
    assert!(
        sent.iter().any(|s| matches!(
            s,
            Sent::Published { uri, version: None, .. } if *uri == a
        )),
        "the file is not open any more: {sent:?}"
    );
}

#[spec(
    behavior = "document_open_close",
    verify = "closing a document outside a project drops its file from the project"
)]
fn closing_a_buffer_outside_a_project_drops_it() {
    let mut served = Served::detached();
    let a = Url::parse("file:///test/a.spec").unwrap();
    let b = Url::parse("file:///test/b.spec").unwrap();
    served.open_document(&a, A_ALPHA);
    served.open_document(&b, B_USES_ALPHA);
    served.sent();
    assert!(served.state().graph().node("alpha").is_some());

    let applied = served.close_uri(&a).expect("the close is planned");
    assert!(applied.changed);
    assert!(
        served.state().graph().node("alpha").is_none(),
        "alpha left the project"
    );
    let codes = last_codes(&served.sent(), &b).expect("published");
    assert!(
        codes.contains(&"E003".to_string()),
        "b.spec now names a missing alpha: {codes:?}"
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
    served.sent();
    let applied = served
        .apply(Change::Watched(vec![event]))
        .expect("specforge.json is an input");
    assert!(applied.environment);
    assert!(served.state().graph().node("omega").is_some());
    assert!(served.state().graph().node("alpha").is_none());
    let sent = served.sent();
    for file in ["a.spec", "b.spec"] {
        let codes = last_codes(&sent, &served.uri(file)).expect("every buffer's file");
        assert!(!codes.contains(&"E003".to_string()), "{file}: {codes:?}");
    }
    assert!(
        logs(&sent).contains(
            &"specforge-lsp: extension environment changed, reloaded 0 extension(s)".to_string()
        ),
        "the reload is announced: {sent:?}"
    );
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
    assert!(Plan::of(Change::Watched(vec![change.clone()]), &served.state()).is_none());
    served.sent();
    assert!(served.apply(Change::Watched(vec![change])).is_none());
    assert!(
        !served
            .sent()
            .iter()
            .any(|sent| matches!(sent, Sent::Published { .. })),
        "nothing is published"
    );
    let deleted = changed(&served, "a.spec", FileChangeType::DELETED);
    assert!(Plan::of(Change::Watched(vec![deleted]), &served.state()).is_some());

    // A file that is not open is the disk's.
    let change = changed(&served, "c.spec", FileChangeType::CHANGED);
    served.write("c.spec", "type sigma \"S\" {}\n");
    let applied = served
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

#[spec(
    behavior = "bring_session_up_to_date",
    verify = "the LSP's catch-up keeps an open buffer"
)]
fn a_catch_up_reads_the_disk_except_for_an_open_buffer() {
    let mut served = Served::new(&[("a.spec", A_ALPHA), ("c.spec", A_OMEGA)]).open(&["a.spec"]);
    served.edit("a.spec", "type zeta \"Z\" {}\n");
    assert!(
        Plan::of(Change::CatchUp, &served.state()).is_none(),
        "nothing changed on disk"
    );

    // Both files are rewritten on disk while the client's watchers moved.
    served.write("a.spec", "type other \"O\" {}\n");
    served.write("c.spec", "type sigma \"S\" {}\n");
    let applied = served.apply(Change::CatchUp).expect("c.spec changed");
    assert!(applied.changed);
    let state = served.state();
    assert!(state.graph().node("zeta").is_some(), "the buffer stays");
    assert!(
        state.graph().node("other").is_none(),
        "its file is not read"
    );
    assert!(
        state.graph().node("sigma").is_some(),
        "c.spec is the disk's"
    );
    assert!(state.graph().node("omega").is_none());
    assert!(
        Plan::of(Change::CatchUp, &served.state()).is_none(),
        "a catch-up leaves nothing stale"
    );
}

#[spec(
    behavior = "bring_session_up_to_date",
    verify = "the LSP's catch-up keeps an open buffer"
)]
fn a_catch_up_sees_the_deletion_of_an_open_documents_file() {
    let mut served = Served::new(&[("a.spec", A_ALPHA), ("c.spec", A_OMEGA)]).open(&["a.spec"]);
    served.edit("a.spec", "type zeta \"Z\" {}\n");

    std::fs::remove_file(served.root().join("a.spec")).unwrap();
    assert!(Plan::of(Change::CatchUp, &served.state()).is_some());
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
