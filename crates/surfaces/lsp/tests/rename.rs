//! The LSP's rename (`specforge_lsp::answers::rename`): the shared plan
//! (`specforge_ops::authoring::rename`) as a workspace edit, every position from the
//! parser.

use specforge_test_macros::test as spec;

use crate::served::{buffers, uri_of_path};
use specforge_lsp::{LspState, answers};
use tower_lsp::lsp_types::{Position, PrepareRenameResponse};

/// The edits of the rename of the entity declared at `line`:`character` of
/// the open buffer `path` to `new`, as `"file line:start-end"` (0-based
/// lines, UTF-16 columns), in file order.
fn renamed_edits(
    state: &LspState,
    path: &str,
    (line, character): (u32, u32),
    new: &str,
) -> Result<Vec<String>, tower_lsp::jsonrpc::Error> {
    let edit = answers::rename(
        state,
        &uri_of_path(path),
        Position::new(line, character),
        new,
    )?
    .expect("the cursor names an entity");
    let mut edits: Vec<String> = edit
        .changes
        .expect("changes")
        .into_iter()
        .flat_map(|(uri, edits)| {
            edits.into_iter().map(move |e| {
                format!(
                    "{} {}:{}-{}",
                    uri.path(),
                    e.range.start.line,
                    e.range.start.character,
                    e.range.end.character
                )
            })
        })
        .collect();
    edits.sort();
    Ok(edits)
}

const TYPES: &str = "type auth_token \"Token\" {\n}\n";
const TYPES_BELOW: &str = "type\n  auth_token \"Token\" {\n}\n";
const AUTH: &str = "behavior user_login \"Login\" {\n  types [auth_token]\n}\n";

// -- rename_entity_id ---------------------------------------------------------

#[spec(
    behavior = "rename_entity_id",
    verify = "rename updates declaration and all references"
)]
fn rename_updates_all_sites() {
    let state = buffers(&[("/p/types.spec", TYPES), ("/p/auth.spec", AUTH)]);
    let edits = renamed_edits(&state, "/p/types.spec", (0, 6), "session_token").unwrap();
    // The declaration's name, and the reference from user_login.
    assert_eq!(edits, ["/p/auth.spec 1:9-19", "/p/types.spec 0:5-15"]);
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename is atomic — all or nothing"
)]
#[tokio::test]
async fn rename_is_atomic() {
    use crate::session::{Session, uri_of};
    use serde_json::json;

    let dir = tempfile::TempDir::new().unwrap();
    let limit = dir.path().join("limit.spec");
    let login = dir.path().join("login.spec");
    let login_text = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n";
    std::fs::write(&limit, "invariant session_limit \"Limit\" {\n}\n").unwrap();
    std::fs::write(&login, login_text).unwrap();
    let (mut session, _) = Session::start(Some(dir.path())).await;
    let login_uri = uri_of(&login);
    session.open(&login_uri, login_text).await;
    session.diagnostics(&login_uri).await;

    let rename = json!({
        "textDocument": {"uri": login_uri},
        "position": {"line": 1, "character": 16},
        "newName": "session_cap",
    });
    let edit = |line: u32, start: u32| {
        json!([{
            "range": {
                "start": {"line": line, "character": start},
                "end": {"line": line, "character": start + 13},
            },
            "newText": "session_cap",
        }])
    };

    // All: one workspace edit renames the declaration and the reference.
    let all = session.request("textDocument/rename", rename.clone()).await;
    assert_eq!(
        all["result"]["changes"],
        json!({ uri_of(&limit): edit(0, 10), login_uri.clone(): edit(1, 14) })
    );

    // Nothing: when the declaration's file can no longer be read, the
    // rename is refused rather than applied to the reference alone.
    std::fs::remove_file(&limit).unwrap();
    let nothing = session.request("textDocument/rename", rename).await;
    assert!(nothing["result"].is_null(), "{nothing}");
    // Said as the protocol's ContentModified: what the edit would apply to
    // is not what the project was compiled from.
    assert_eq!(nothing["error"]["code"], -32801, "{nothing}");
}

/// The edits of a rename are positions in the text the project was compiled
/// from, and apply to the text the editor has: a file whose text changed
/// since the compile (here, on disk, with no change event yet) is not
/// renamed from stale positions.
#[spec(
    behavior = "rename_entity_id",
    verify = "rename is refused as content modified when a file it edits changed since the compile"
)]
#[tokio::test]
async fn rename_waits_for_the_compile_of_a_changed_file() {
    use crate::session::{Session, uri_of};
    use serde_json::json;

    let dir = tempfile::TempDir::new().unwrap();
    let limit = dir.path().join("limit.spec");
    let login = dir.path().join("login.spec");
    let login_text = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n";
    std::fs::write(&limit, "invariant session_limit \"Limit\" {\n}\n").unwrap();
    std::fs::write(&login, login_text).unwrap();
    let (mut session, _) = Session::start(Some(dir.path())).await;
    let login_uri = uri_of(&login);
    session.open(&login_uri, login_text).await;
    session.diagnostics(&login_uri).await;
    let rename = json!({
        "textDocument": {"uri": login_uri},
        "position": {"line": 1, "character": 16},
        "newName": "session_cap",
    });

    // A line added above the declaration, as another tool would.
    std::fs::write(&limit, "// moved\ninvariant session_limit \"Limit\" {\n}\n").unwrap();
    let refused = session.request("textDocument/rename", rename).await;
    assert!(refused["result"].is_null(), "{refused}");
    assert_eq!(refused["error"]["code"], -32801, "{refused}");
}

/// The new name follows the shared entity-ID rule: the editor gets an
/// error saying why, and no edit.
#[spec(
    behavior = "rename_entity_id",
    verify = "rename to an illegal entity ID is refused with why"
)]
#[tokio::test]
async fn rename_to_an_illegal_id_is_refused_with_why() {
    use crate::session::{Session, uri_of};
    use serde_json::json;

    let dir = tempfile::TempDir::new().unwrap();
    let login = dir.path().join("login.spec");
    let text = "behavior login \"Login\" {\n}\n";
    std::fs::write(&login, text).unwrap();
    let (mut session, _) = Session::start(Some(dir.path())).await;
    let uri = uri_of(&login);
    session.open(&uri, text).await;
    session.diagnostics(&uri).await;

    let refused = session
        .request(
            "textDocument/rename",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 10},
                "newName": "log--in",
            }),
        )
        .await;
    assert!(refused["result"].is_null(), "{refused}");
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("invalid entity ID 'log--in'"), "{refused}");
}

/// A reserved word is no entity ID either: the editor gets the reason, and no edit.
#[spec(
    behavior = "rename_entity_id",
    verify = "rename to a reserved word is refused with why"
)]
#[tokio::test]
async fn rename_to_a_reserved_word_is_refused_with_why() {
    use crate::session::{Session, uri_of};
    use serde_json::json;

    let dir = tempfile::TempDir::new().unwrap();
    let login = dir.path().join("login.spec");
    let text = "behavior login \"Login\" {\n}\n";
    std::fs::write(&login, text).unwrap();
    let (mut session, _) = Session::start(Some(dir.path())).await;
    let uri = uri_of(&login);
    session.open(&uri, text).await;
    session.diagnostics(&uri).await;

    let refused = session
        .request(
            "textDocument/rename",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 10},
                "newName": "defaults",
            }),
        )
        .await;
    assert!(refused["result"].is_null(), "{refused}");
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("invalid entity ID 'defaults'"),
        "{refused}"
    );
    assert!(message.contains("reserved"), "{refused}");
}

#[spec(behavior = "rename_entity_id", verify = "rename across multiple files")]
fn rename_across_files() {
    let state = buffers(&[
        ("/p/a.spec", "type tok \"T\" {\n}\n"),
        ("/p/b.spec", "behavior b1 \"B1\" {\n  types [tok]\n}\n"),
        ("/p/c.spec", "behavior b2 \"B2\" {\n  types [tok]\n}\n"),
    ]);
    let edits = renamed_edits(&state, "/p/a.spec", (0, 6), "token").unwrap();
    assert_eq!(
        edits,
        ["/p/a.spec 0:5-8", "/p/b.spec 1:9-12", "/p/c.spec 1:9-12"]
    );
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename rejects new name that duplicates existing entity ID"
)]
fn rename_rejects_duplicate() {
    let state = buffers(&[("/p/types.spec", TYPES), ("/p/auth.spec", AUTH)]);
    let refused = renamed_edits(&state, "/p/types.spec", (0, 6), "user_login").unwrap_err();
    assert_eq!(refused.code, tower_lsp::jsonrpc::ErrorCode::InvalidParams);
    assert_eq!(
        refused.message,
        "cannot rename 'auth_token': 'user_login' exists"
    );
}

/// All or nothing: a file the compile holds no text of refuses the whole rename.
#[test]
fn a_rename_without_a_files_text_is_refused_whole() {
    let state = buffers(&[("/p/types.spec", TYPES), ("/p/auth.spec", AUTH)]);
    let compiled = state.session().unwrap().project();
    let without_auth = compiled.with_source_texts(
        compiled
            .source_texts()
            .into_iter()
            .filter(|(key, _)| key != "/p/auth.spec"),
    );
    let blind = specforge_ops::read::navigate::Navigator::new(
        specforge_ops::view::ProjectView::of(&without_auth),
    );

    let refused = specforge_ops::authoring::rename::plan(&blind, "auth_token", "session_token");

    assert_eq!(
        refused.unwrap_err().error.code,
        specforge_ops::FILE_UNREADABLE
    );
}

// -- a name written below its kind keyword -----------------------------------

#[spec(
    behavior = "prepare_rename",
    verify = "prepare rename answers on a name written below its kind"
)]
fn prepare_rename_on_a_name_written_below_its_kind_has_a_rename_range() {
    let state = buffers(&[("/p/types.spec", TYPES_BELOW), ("/p/auth.spec", AUTH)]);
    let prepared =
        answers::prepare_rename(&state, &uri_of_path("/p/types.spec"), Position::new(1, 4));
    let Some(PrepareRenameResponse::Range(r)) = prepared.unwrap() else {
        panic!("no rename range");
    };
    assert_eq!(
        (r.start, r.end),
        (Position::new(1, 2), Position::new(1, 12))
    );
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename edits a name written below its kind and its references"
)]
fn rename_of_a_name_written_below_its_kind_edits_it_and_its_references() {
    let state = buffers(&[("/p/types.spec", TYPES_BELOW), ("/p/auth.spec", AUTH)]);
    let want = ["/p/auth.spec 1:9-19", "/p/types.spec 1:2-12"];
    let from_reference = renamed_edits(&state, "/p/auth.spec", (1, 12), "session_token");
    assert_eq!(from_reference.unwrap(), want);
    let from_declaration = renamed_edits(&state, "/p/types.spec", (1, 4), "session_token");
    assert_eq!(from_declaration.unwrap(), want);
}

// -- the ledger of a rename (ADR 0289) -----------------------------------------------------------

mod ledger {
    use specforge_extension_sdk::DeclaredCategory;
    use specforge_lsp::changes::Change;
    use specforge_project::testing::gsd::{gsd_history_passes, gsd_history_wire};
    use specforge_project::testing::{GSD_EXTENSION, revision_of};
    use specforge_test_macros::test as spec;
    use tower_lsp::lsp_types::{FileChangeType, FileEvent, Position};

    use crate::recorder::Sent;
    use crate::served::Served;
    use specforge_lsp::answers;

    const M: &str = "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n";
    const U: &str = "uat_test U1 \"One\" {\n  phase 03\n  result \"pass\"\n}\n";

    const HEADER: &str = "# specforge-harness 1\n";

    /// A ledger line: `path key digest`, the digest of `path\0key\0value`.
    fn line(path: &str, key: &str, value: &str) -> String {
        let digest = revision_of(format!("{path}\0{key}\0{value}").as_bytes());
        format!("{path} {key} {}\n", digest.trim_start_matches("sha256:"))
    }

    fn served() -> Served {
        let ledger = format!("{HEADER}{}", line("v1.0/03/U1", "result", "\"pass\""));
        Served::new(&[("m.spec", M), ("u.spec", U), ("specforge-harness", &ledger)])
            .extension(GSD_EXTENSION, |c| {
                c.raw_category(DeclaredCategory::Entities, gsd_history_wire());
                c.raw_category(DeclaredCategory::Passes, gsd_history_passes());
            })
            .open(&["m.spec", "u.spec"])
    }

    fn e083(sent: &[Sent]) -> bool {
        sent.iter().any(|s| {
            matches!(s, Sent::Published { diagnostics, .. }
                if diagnostics.iter().any(|d| matches!(&d.code,
                    Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "E083")))
        })
    }

    fn ledger_on_disk(served: &Served) -> String {
        std::fs::read_to_string(served.root().join("specforge-harness")).unwrap_or_default()
    }

    /// The rename of phase `03` to `05` as the editor applies it to its buffers, then the editor's save.
    fn rename_and_type(served: &mut Served) -> (Vec<Sent>, tower_lsp::lsp_types::WorkspaceEdit) {
        let uri = served.uri("m.spec");
        let at = Position::new(2, 7);
        let edit = answers::rename(&served.state(), &uri, at, "05")
            .unwrap()
            .expect("the cursor names an entity");
        served.edit("m.spec", &M.replace("phase 03", "phase 05"));
        served.edit("u.spec", &U.replace("phase 03", "phase 05"));
        (served.sent(), edit)
    }

    fn save(served: &mut Served) {
        for (file, text) in [("m.spec", M), ("u.spec", U)] {
            served.write(file, &text.replace("phase 03", "phase 05"));
        }
        let events = ["m.spec", "u.spec"]
            .map(|file| FileEvent {
                uri: served.uri(file),
                typ: FileChangeType::CHANGED,
            })
            .to_vec();
        served.apply(Change::Watched(events));
    }

    #[ignore = "K5 T4"]
    #[spec(
        behavior = "land_rename_ledger",
        verify = "the LSP rename writes its ledger change once the rename lands on disk"
    )]
    fn the_lsp_rename_writes_its_ledger_once_it_lands() {
        let mut served = served();
        let (sent, edit) = rename_and_type(&mut served);
        let changes = edit.changes.expect("changes");
        assert!(
            changes
                .keys()
                .all(|uri| !uri.path().ends_with("specforge-harness")),
            "{changes:?}"
        );
        // Before the save, the buffers hold the rename and the ledger does not yet: no E083.
        assert!(!e083(&sent), "{sent:?}");
        assert_eq!(
            ledger_on_disk(&served),
            line("v1.0/03/U1", "result", "\"pass\"")
        );
        save(&mut served);
        let ledger = ledger_on_disk(&served);
        assert!(
            ledger.contains(&line("v1.0/05", "moved:v1.0/03", "")),
            "{ledger}"
        );
        assert!(
            ledger.contains(&line("v1.0/05/U1", "result", "\"pass\"")),
            "{ledger}"
        );
        assert!(!ledger.contains("v1.0/03/U1"), "{ledger}");
    }

    #[ignore = "K5 T4"]
    #[spec(
        behavior = "land_rename_ledger",
        verify = "an undone LSP rename writes no ledger change"
    )]
    fn an_undone_lsp_rename_writes_no_ledger_change() {
        let mut served = served();
        let before = ledger_on_disk(&served);
        rename_and_type(&mut served);
        served.edit("m.spec", M);
        served.edit("u.spec", U);
        for file in ["m.spec", "u.spec"] {
            let uri = served.uri(file);
            served.apply(Change::Watched(vec![FileEvent {
                uri,
                typ: FileChangeType::CHANGED,
            }]));
        }
        assert_eq!(ledger_on_disk(&served), before);
    }
}

#[ignore = "K5 T4"]
#[spec(
    behavior = "land_rename_ledger",
    verify = "the LSP rename and fixes are versioned for a client that takes document changes"
)]
fn the_lsp_rename_and_fixes_are_versioned() {
    use specforge_lsp::ClientSupport;
    use tower_lsp::lsp_types::{
        ClientCapabilities, DocumentChanges, WorkspaceClientCapabilities,
        WorkspaceEditClientCapabilities,
    };
    let mut state = buffers(&[("/p/types.spec", TYPES), ("/p/auth.spec", AUTH)]);
    let rename = |state: &LspState| {
        answers::rename(
            state,
            &uri_of_path("/p/types.spec"),
            Position::new(0, 6),
            "session_token",
        )
        .unwrap()
        .unwrap()
    };
    // Without the client's support, `changes` as today.
    let plain = rename(&state);
    assert!(plain.changes.is_some() && plain.document_changes.is_none());
    state.set_client(ClientSupport::of(&ClientCapabilities {
        workspace: Some(WorkspaceClientCapabilities {
            workspace_edit: Some(WorkspaceEditClientCapabilities {
                document_changes: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }));
    let versioned = rename(&state);
    let Some(DocumentChanges::Edits(edits)) = versioned.document_changes else {
        panic!("a client that takes document changes gets versioned edits");
    };
    assert!(versioned.changes.is_none());
    assert!(
        edits.iter().all(|e| e.text_document.version.is_some()),
        "{edits:?}"
    );
}
