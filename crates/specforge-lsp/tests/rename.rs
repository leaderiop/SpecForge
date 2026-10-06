//! The LSP's rename: the shared plan (`specforge_ops::rename`) over the
//! session's navigator, every position from the parser.

use specforge_test_macros::test as spec;

/// An LSP state whose session holds `files` (absolute paths) as open
/// buffers: what navigation reads.
fn state_of(files: &[(&str, &str)]) -> specforge_lsp::LspState {
    let mut state = specforge_lsp::LspState::new();
    for (path, text) in files {
        state.open_document(&format!("file://{path}"), text);
        state
            .session_mut()
            .unwrap()
            .update(specforge_project::SourceChange::Buffer {
                path,
                text: Some(text),
            });
    }
    state
}

/// The plan's edits as `"file line:start-end"` (0-based byte columns).
fn rename_edits(
    state: &specforge_lsp::LspState,
    old: &str,
    new: &str,
) -> Result<Vec<String>, specforge_ops::OpError> {
    let plan = specforge_ops::rename::plan(&specforge_lsp::navigator(state), old, new)?;
    Ok(plan
        .edits
        .iter()
        .map(|e| format!("{} {}:{}-{}", e.file, e.line, e.start_col, e.end_col))
        .collect())
}

const TYPES: &str = "type auth_token \"Token\" {\n}\n";
const AUTH: &str = "behavior user_login \"Login\" {\n  types [auth_token]\n}\n";

// -- rename_entity_id ---------------------------------------------------------

#[spec(
    behavior = "rename_entity_id",
    verify = "rename updates declaration and all references"
)]
fn rename_updates_all_sites() {
    let state = state_of(&[("/p/types.spec", TYPES), ("/p/auth.spec", AUTH)]);
    let edits = rename_edits(&state, "auth_token", "session_token").unwrap();
    // The declaration's name, and the reference from user_login.
    assert_eq!(edits, ["/p/auth.spec 2:9-19", "/p/types.spec 1:5-15"]);
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
                "newName": "log-in",
            }),
        )
        .await;
    assert!(refused["result"].is_null(), "{refused}");
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("invalid entity ID 'log-in'"), "{refused}");
}

#[spec(behavior = "rename_entity_id", verify = "rename across multiple files")]
fn rename_across_files() {
    let state = state_of(&[
        ("/p/a.spec", "type tok \"T\" {\n}\n"),
        ("/p/b.spec", "behavior b1 \"B1\" {\n  types [tok]\n}\n"),
        ("/p/c.spec", "behavior b2 \"B2\" {\n  types [tok]\n}\n"),
    ]);
    let edits = rename_edits(&state, "tok", "token").unwrap();
    assert_eq!(
        edits,
        ["/p/a.spec 1:5-8", "/p/b.spec 2:9-12", "/p/c.spec 2:9-12"]
    );
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename rejects new name that duplicates existing entity ID"
)]
fn rename_rejects_duplicate() {
    let state = state_of(&[("/p/types.spec", TYPES), ("/p/auth.spec", AUTH)]);
    let refused = rename_edits(&state, "auth_token", "user_login").unwrap_err();
    assert_eq!(refused.code, specforge_ops::rename::TAKEN);
}
