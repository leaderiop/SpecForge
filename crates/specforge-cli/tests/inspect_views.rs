//! Inspect on every surface (architecture plan 2026-10-06, 07).
//!
//! MCP `specforge.inspect`, the MCP context prompt and the LSP hover each
//! say what one entity is. The `*_today` tests pin what each answers on two
//! small projects, `fixtures/coverage/fx1` and `fixtures/navigation/cyc`, as
//! insta snapshots (`tests/snapshots/tests__inspect_views__*.snap`). They
//! prove no spec obligation (they pin current behavior, bugs included), so
//! they carry no `specforge_test` link; a change that alters what they pin
//! re-blesses the snapshot in the same commit, where the diff shows it.
//!
//! cyc holds two behaviors, `alpha` and `beta`, that depend on each other
//! (a reference cycle, W061, which names both in its data).

use serde_json::{Value, json};
use std::path::Path;
use tempfile::TempDir;

use crate::coverage_corpus::{copy_tree, mcp_calls, project};
use crate::parity::normalized;
use crate::surface_parity::lsp::Client;

/// The entities of fx1 the views are pinned on.
const FX1: [&str; 6] = [
    "login",
    "logout",
    "Status",
    "signin",
    "Payload",
    "no_lost_login",
];

/// The entities of cyc the views are pinned on.
const CYC: [&str; 2] = ["alpha", "beta"];

/// A scratch copy of `fixtures/navigation/cyc` (its spec root is the
/// project root).
fn cyc() -> TempDir {
    let tmp = TempDir::new().unwrap();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/navigation/cyc"),
        tmp.path(),
    );
    tmp
}

fn inspect(entity_id: &str) -> Value {
    json!({"name": "specforge.inspect", "arguments": {"entity_id": entity_id}})
}

/// The 0-based position of `id`'s declaration name in `text`: the line
/// whose second word is `id`, one character into the name.
fn declaration(text: &str, id: &str) -> (u32, u32) {
    text.lines()
        .enumerate()
        .find_map(|(line, l)| {
            (l.split_whitespace().nth(1) == Some(id))
                .then(|| (line as u32, (l.find(id).unwrap() + 1) as u32))
        })
        .unwrap_or_else(|| panic!("no declaration of {id}"))
}

/// The hover markdown at each position of `file` (relative to `root`), in
/// order, from the real LSP backend serving `root`; `"null"` for no hover.
fn hovers(root: &Path, file: &str, positions: &[(u32, u32)]) -> Vec<String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut client = Client::start();
        client.open_workspace(root).await;
        let path = root.join(file);
        client.open(&path).await;
        let uri = tower_lsp::lsp_types::Url::from_file_path(&path)
            .unwrap()
            .to_string();
        let mut out = Vec::new();
        for (line, character) in positions {
            let response = client
                .request(
                    "textDocument/hover",
                    json!({"textDocument": {"uri": uri},
                           "position": {"line": line, "character": character}}),
                )
                .await;
            let result = &response["result"];
            out.push(match result["contents"]["value"].as_str() {
                Some(markdown) => markdown.to_string(),
                None => result.to_string(),
            });
        }
        out
    })
}

#[test]
fn inspect_today() {
    let fx1 = project("fx1");
    let root = fx1.path();
    let mut calls: Vec<Value> = FX1.iter().map(|id| inspect(id)).collect();
    calls.push(inspect("ghost"));
    let results = mcp_calls(root, &calls);
    for (id, result) in FX1.iter().chain(["ghost"].iter()).zip(&results) {
        insta::assert_snapshot!(format!("inspect_fx1_{id}"), normalized(result, root));
    }

    let cyc = cyc();
    let root = cyc.path();
    let calls: Vec<Value> = CYC.iter().map(|id| inspect(id)).collect();
    for (id, result) in CYC.iter().zip(mcp_calls(root, &calls)) {
        insta::assert_snapshot!(format!("inspect_cyc_{id}"), normalized(&result, root));
    }
}

#[test]
fn context_prompt_today() {
    let fx1 = project("fx1");
    let root = fx1.path();
    let calls = [json!({"method": "prompts/get", "params": {
        "name": "specforge://prompts/context", "arguments": {"entity_id": "login"}}})];
    let result = mcp_calls(root, &calls).pop().unwrap();
    insta::assert_snapshot!("context_fx1_login", normalized(&result, root));
}

#[test]
fn hover_today() {
    let fx1 = project("fx1");
    let root = fx1.path();
    let text = std::fs::read_to_string(root.join("spec/main.spec")).unwrap();
    let mut positions: Vec<(u32, u32)> = FX1.iter().map(|id| declaration(&text, id)).collect();
    // `signin` inside `login`'s `features [signin]`.
    positions.push((2, 13));
    let markdown = hovers(root, "spec/main.spec", &positions);
    let names = FX1
        .iter()
        .map(|id| format!("hover_fx1_{id}"))
        .chain(["hover_fx1_signin_reference".to_string()]);
    for (name, markdown) in names.zip(markdown) {
        insta::assert_snapshot!(name, markdown);
    }

    let cyc = cyc();
    let root = cyc.path();
    let markdown = hovers(root, "a.spec", &[(0, 10), (3, 10)]);
    for (id, markdown) in CYC.iter().zip(markdown) {
        insta::assert_snapshot!(format!("hover_cyc_{id}"), markdown);
    }
}
