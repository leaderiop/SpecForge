use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as spec;

fn span(file: &str, line: usize, col: usize, end_col: usize) -> SourceSpan {
    SourceSpan {
        file: Sym::new(file),
        start_line: line,
        start_col: col,
        end_line: line,
        end_col,
    }
}

fn node_at(id: &str, kind: &str, file: &str, line: usize, col: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: None,
        fields: FieldMap::new(),
        source_span: span(file, line, col, col + id.len()),
        methods: Vec::new(),
    }
}

fn graph_with_refs() -> Graph {
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types.spec", 5, 5));
    g.add_node(node_at("user_login", "behavior", "auth.spec", 10, 9));
    g.add_edge(Edge {
        source: "user_login".into(),
        target: "auth_token".into(),
        label: "types".into(),
    });
    g
}

/// Text for each fixture file: every node's id written on each line of its
/// span at its column, and every referencing node's line also naming the
/// ids it references — what `identifier_edits` scans.
fn texts_for(g: &Graph) -> std::collections::HashMap<String, String> {
    let mut lines: std::collections::HashMap<String, std::collections::BTreeMap<usize, String>> =
        std::collections::HashMap::new();
    for node in g.nodes() {
        let span = &node.source_span;
        let refs: Vec<String> = g
            .edges_from(node.id.raw.as_str())
            .iter()
            .map(|e| e.target.to_string())
            .collect();
        let line = format!(
            "{}{} [{}]",
            " ".repeat(span.start_col),
            node.id.raw,
            refs.join(", ")
        );
        lines
            .entry(span.file.to_string())
            .or_default()
            .insert(span.start_line, line);
    }
    lines
        .into_iter()
        .map(|(file, by_line)| {
            let last = by_line.keys().max().copied().unwrap_or(0);
            let text: Vec<String> = (1..=last)
                .map(|n| by_line.get(&n).cloned().unwrap_or_default())
                .collect();
            (file, text.join("\n") + "\n")
        })
        .collect()
}

fn rename_edits(g: &Graph, old: &str, new: &str) -> Option<Vec<specforge_lsp::RenameEdit>> {
    let texts = texts_for(g);
    specforge_lsp::identifier_edits(g, old, new, |f| texts.get(f).cloned())
}

// -- prepare_rename -----------------------------------------------------------

#[spec(
    behavior = "prepare_rename",
    verify = "prepare rename on entity ID returns token range"
)]
fn prepare_rename_returns_range() {
    let g = graph_with_refs();
    let result = specforge_lsp::prepare_rename(&g, "auth_token");
    let range = result.expect("should return range");
    assert_eq!(range.file, "types.spec");
    assert_eq!(range.start_line, 5);
    assert_eq!(range.start_col, 5);
    assert_eq!(range.end_col, 5 + "auth_token".len());
}

#[spec(
    behavior = "prepare_rename",
    verify = "prepare rename on non-renameable token returns not available"
)]
fn prepare_rename_returns_none_for_missing() {
    let g = graph_with_refs();
    let result = specforge_lsp::prepare_rename(&g, "nonexistent");
    assert!(result.is_none());
}

// -- rename_entity_id ---------------------------------------------------------

#[spec(
    behavior = "rename_entity_id",
    verify = "rename updates declaration and all references"
)]
fn rename_updates_all_sites() {
    let g = graph_with_refs();
    let edits = rename_edits(&g, "auth_token", "session_token");
    let edits = edits.expect("should produce edits");
    // Declaration (types.spec) + reference from user_login (auth.spec)
    assert!(edits.len() >= 2);
    assert!(edits.iter().any(|e| e.file == "types.spec"));
    assert!(edits.iter().any(|e| e.file == "auth.spec"));
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename is atomic — all or nothing"
)]
#[tokio::test]
async fn rename_is_atomic() {
    use crate::contracts::wire::{Session, uri_of};
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
    use crate::contracts::wire::{Session, uri_of};
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
    let mut g = Graph::new();
    g.add_node(node_at("tok", "type", "a.spec", 1, 5));
    g.add_node(node_at("b1", "behavior", "b.spec", 1, 9));
    g.add_node(node_at("b2", "behavior", "c.spec", 1, 9));
    g.add_edge(Edge {
        source: "b1".into(),
        target: "tok".into(),
        label: "types".into(),
    });
    g.add_edge(Edge {
        source: "b2".into(),
        target: "tok".into(),
        label: "types".into(),
    });

    let edits = rename_edits(&g, "tok", "token").unwrap();
    let files: Vec<&str> = edits.iter().map(|e| e.file.as_str()).collect();
    assert!(files.contains(&"a.spec"));
    assert!(files.contains(&"b.spec"));
    assert!(files.contains(&"c.spec"));
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename rejects new name that duplicates existing entity ID"
)]
fn rename_rejects_duplicate() {
    let g = graph_with_refs();
    let edits = rename_edits(&g, "auth_token", "user_login");
    assert!(edits.is_none(), "rename to existing ID should be rejected");
}
