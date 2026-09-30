use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as specforge_test;

fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: title.map(|t| t.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 0,
            start_col: 0,
            end_line: 3,
            end_col: 1,
        },
        methods: Vec::new(),
    }
}

fn node_at(id: &str, kind: &str, file: &str, line: usize, col: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: Some(format!("{id} title")),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new(file),
            start_line: line,
            start_col: col,
            end_line: line + 3,
            end_col: col + id.len(),
        },
        methods: Vec::new(),
    }
}

// B:lsp_initialize — verify contract "requires/ensures consistency for LSP initialization"
#[specforge_test(
    behavior = "lsp_initialize",
    verify = "LSP Initialize: LSP initialization holds — extensions_loaded, capabilities_reflect_extensions, semantic_legend_populated, incremental_sync_advertised, lsp_initialized_emitted"
)]
#[tokio::test]
async fn lsp_initialize_contract() {
    let extensions = ["@specforge/software", "@specforge/testing"];
    let dir = project_with(&extensions);
    let (mut session, init) = wire::Session::start(Some(dir.path())).await;
    let caps = &init["capabilities"];

    // incremental_sync_advertised: TextDocumentSyncKind::INCREMENTAL.
    assert_eq!(caps["textDocumentSync"], 2);

    // semantic_legend_populated: every standard LSP token type, in order.
    let legend = legend_of(&init);
    assert_eq!(legend, STANDARD_TOKEN_TYPES);

    // extensions_loaded, lsp_initialized_emitted: once the registries are
    // populated the server announces how many extensions and entity kinds
    // it loaded.
    let kinds = registries_for(&extensions).0;
    assert!(kinds.len() >= 5, "software alone declares five kinds");
    let announced = session
        .notification("window/logMessage", |p| {
            p["message"].as_str().is_some_and(|m| m.contains("loaded"))
        })
        .await
        .expect("no initialization announcement");
    assert_eq!(
        announced["message"],
        format!(
            "specforge-lsp: loaded 2 extension(s), {} entity kind(s)",
            kinds.len()
        )
    );

    // capabilities_reflect_extensions: nothing domain-specific is
    // hardcoded (the legend is exactly the standard list), and the
    // advertised legend carries what the loaded extension declares —
    // @specforge/software gives `port` IDs the `interface` token.
    let uri = "file:///buffer/repo.spec";
    session.open(uri, "port repo \"Repo\" {\n}\n").await;
    let tokens = session
        .request(
            "textDocument/semanticTokens/full",
            serde_json::json!({"textDocument": {"uri": uri}}),
        )
        .await;
    let data = tokens["result"]["data"].as_array().unwrap();
    // The second token is `repo` at line 0, column 5.
    assert_eq!(data[5..8], [0, 5, 4], "{data:?}");
    assert_eq!(legend[data[8].as_u64().unwrap() as usize], "interface");
}

/// Every standard LSP semantic token type, in the order the server's legend
/// lists them.
pub(crate) const STANDARD_TOKEN_TYPES: [&str; 23] = [
    "namespace",
    "type",
    "class",
    "enum",
    "interface",
    "struct",
    "typeParameter",
    "parameter",
    "variable",
    "property",
    "enumMember",
    "event",
    "function",
    "method",
    "macro",
    "keyword",
    "modifier",
    "comment",
    "string",
    "number",
    "regexp",
    "operator",
    "decorator",
];

/// The semantic token legend of an `initialize` result.
pub(crate) fn legend_of(init: &serde_json::Value) -> Vec<&str> {
    init["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .expect("initialize result has no legend")
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect()
}

/// A temp project whose specforge.json lists `extensions`.
pub(crate) fn project_with(extensions: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "test-project",
        "version": "0.1.0",
        "extensions": extensions,
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    dir
}

/// The kind and field registries `extensions` populate.
fn registries_for(
    extensions: &[&str],
) -> (
    specforge_registry::KindRegistry,
    specforge_registry::FieldRegistry,
) {
    let names: Vec<String> = extensions.iter().map(|s| s.to_string()).collect();
    let runtime = wasm_runtime_for(&names);
    let host = specforge_wasm::protocol::ProtocolHost::new(&runtime);
    let manifests: Vec<_> = names
        .iter()
        .map(|name| {
            let ext = specforge_wasm::protocol::load_protocol_extension(&host, name)
                .unwrap_or_else(|e| panic!("{name} does not load: {e:?}"));
            specforge_wasm::protocol::protocol_extension_to_manifest(&ext)
        })
        .collect();
    let (kinds, fields, _edges, _diags) = specforge_registry::populate_registries(&manifests);
    (kinds, fields)
}

// B:lsp_shutdown — verify contract "requires/ensures consistency for LSP shutdown"
#[specforge_test(
    behavior = "lsp_shutdown",
    verify = "LSP Shutdown: LSP shutdown holds — lsp_initialized_fired, resources_released, post_shutdown_rejected, no_disk_persistence, lsp_shutdown_complete_emitted"
)]
fn lsp_shutdown_contract() {
    // Requires: active LSP state with open documents
    // Ensures: shutdown releases state, subsequent operations rejected
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "content");
    assert!(state.is_open("file:///a.spec"));

    state.shutdown();

    assert!(state.is_shutdown(), "state must be marked as shutdown");
    assert!(
        !state.is_open("file:///a.spec"),
        "documents must be released"
    );

    // Post-shutdown operations should be rejected
    state.open_document("file:///b.spec", "new content");
    assert!(
        !state.is_open("file:///b.spec"),
        "must reject operations after shutdown"
    );
}

// B:document_open_close — verify contract "requires/ensures consistency for document open/close"
#[specforge_test(
    behavior = "document_open_close",
    verify = "Document Open/Close: document open/close holds — lsp_initialized_fired, document_tracked, file_changed_emitted, closed_diagnostics_cleared"
)]
#[tokio::test]
async fn document_open_close_contract() {
    // lsp_initialized_fired: the session is initialized.
    let (mut session, _) = wire::Session::start(None).await;
    // Only in the editor buffer: nothing on disk.
    let uri = "file:///buffer/open_close.spec";
    let text = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n";

    // file_changed_emitted: didOpen compiles the buffer — the published
    // diagnostics are the buffer's dangling reference.
    session.open(uri, text).await;
    let opened = session.diagnostics(uri).await;
    assert_eq!(wire::codes(&opened), ["E003"], "{opened:?}");
    assert_eq!(
        opened[0]["message"],
        "unresolved reference 'session_limit' in entity 'login'"
    );

    // document_tracked: an open document is served from its buffer
    // (formatting answers only for open documents), a closed one is not.
    assert!(session.format(uri).await.is_array());
    session.close(uri).await;

    // closed_diagnostics_cleared: closing publishes an empty set, which
    // clears the editor's squiggles.
    let closed = session.diagnostics(uri).await;
    assert!(closed.is_empty(), "{closed:?}");
    assert!(session.format(uri).await.is_null());
}

// B:autocomplete_entity_ids — verify contract "requires/ensures consistency for entity ID autocomplete"
#[specforge_test(
    behavior = "autocomplete_entity_ids",
    verify = "Autocomplete Entity IDs: entity ID autocomplete holds — graph_available, field_registry_available, matching_ids_suggested, target_kind_filtering_applied"
)]
fn autocomplete_entity_ids_contract() {
    // Requires: graph with entities + prefix
    // Ensures: matching IDs returned with kind and title
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login")));
    g.add_node(node("user_logout", "behavior", Some("User Logout")));
    g.add_node(node("auth_token", "type", Some("Auth Token")));

    let items = specforge_lsp::complete_entity_ids(&g, "user");

    assert_eq!(items.len(), 2, "only matching IDs returned");
    for item in &items {
        assert!(item.id.starts_with("user"), "each item must match prefix");
        assert!(!item.kind.is_empty(), "kind must be populated");
    }
}

// B:complete_field_names — verify contract "requires/ensures consistency for field name completion"
#[specforge_test(
    behavior = "complete_field_names",
    verify = "Complete Field Names: field name completion holds — field_registry_available, cursor_inside_entity, fields_suggested, snippets_informed"
)]
fn complete_field_names_contract() {
    // Requires: entity kind name + populated FieldRegistry
    // Ensures: field names appropriate for that kind returned
    let ext_names: Vec<String> = [
        "@specforge/software",
        "@specforge/product",
        "@specforge/governance",
        "@specforge/formal",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let runtime = wasm_runtime_for(&ext_names);
    let host = specforge_wasm::protocol::ProtocolHost::new(&runtime);
    let mut manifests = Vec::new();
    for name in &ext_names {
        if let Ok(ext) = specforge_wasm::protocol::load_protocol_extension(&host, name) {
            manifests.push(specforge_wasm::protocol::protocol_extension_to_manifest(
                &ext,
            ));
        }
    }
    let (_kind_reg, field_reg, _edge_reg, _diags) =
        specforge_registry::populate_registries(&manifests);

    let behavior_fields = specforge_lsp::complete_field_names("behavior", Some(&field_reg));
    assert!(
        !behavior_fields.is_empty(),
        "known kind must have field suggestions"
    );
    assert!(
        behavior_fields.iter().any(|f| f == "contract"),
        "behavior must include 'contract'"
    );

    let unknown_fields = specforge_lsp::complete_field_names("__nonexistent__", Some(&field_reg));
    assert!(
        unknown_fields.is_empty(),
        "unknown kind must return no fields"
    );
}

// B:complete_keywords — verify contract "requires/ensures consistency for keyword completion"
#[specforge_test(
    behavior = "complete_keywords",
    verify = "Complete Keywords: keyword completion holds — kind_registry_available, cursor_at_top_level, keywords_delegated, structural_keywords_included"
)]
fn complete_keywords_contract() {
    // Requires: set of registered extension kinds
    // Ensures: all registered kinds + structural keywords returned, no duplicates
    let keywords = specforge_lsp::complete_keywords(&["behavior", "type"]);

    assert!(
        keywords.contains(&"behavior".to_string()),
        "registered kind must be included"
    );
    assert!(
        keywords.contains(&"type".to_string()),
        "registered kind must be included"
    );
    assert!(
        keywords.contains(&"use".to_string()),
        "structural keyword must be included"
    );
    assert!(
        keywords.contains(&"define".to_string()),
        "structural keyword must be included"
    );

    // No duplicates
    let mut sorted = keywords.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(keywords.len(), sorted.len(), "must have no duplicates");
}

// B:hover_information — verify contract "requires/ensures consistency for hover information"
#[specforge_test(
    behavior = "hover_information",
    verify = "Hover Information: hover information holds — graph_available, kind_registry_available, hover_delegated, markdown_produced"
)]
fn hover_information_contract() {
    // Requires: entity ID exists in graph
    // Ensures: hover returns markdown with kind, id, title; None for missing
    let mut g = Graph::new();
    g.add_node(node("user_login", "behavior", Some("User Login")));

    let hover = specforge_lsp::hover_info(&g, "user_login");
    let text = hover.expect("existing entity must produce hover");
    assert!(text.contains("behavior"), "hover must include kind");
    assert!(text.contains("user_login"), "hover must include id");
    assert!(text.contains("User Login"), "hover must include title");

    let missing = specforge_lsp::hover_info(&g, "nonexistent");
    assert!(missing.is_none(), "missing entity must return None");
}

// B:find_all_references — verify contract "requires/ensures consistency for find all references"
#[specforge_test(
    behavior = "find_all_references",
    verify = "Find All References: find all references holds — graph_available, all_references_returned, declaration_included"
)]
fn find_all_references_contract() {
    // Requires: entity in graph with edges from other entities
    // Ensures: declaration + all reference sites returned
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types.spec", 10, 5));
    g.add_node(node_at("login", "behavior", "auth.spec", 5, 9));
    g.add_node(node_at("refresh", "behavior", "session.spec", 3, 9));
    g.add_edge(Edge {
        source: "login".into(),
        target: "auth_token".into(),
        label: "types".into(),
    });
    g.add_edge(Edge {
        source: "refresh".into(),
        target: "auth_token".into(),
        label: "types".into(),
    });

    let refs = specforge_lsp::find_all_references(&g, "auth_token");

    assert_eq!(refs.len(), 3, "declaration + 2 reference sites");
    let files: Vec<&str> = refs.iter().map(|l| l.file.as_str()).collect();
    assert!(
        files.contains(&"types.spec"),
        "must include declaration site"
    );
    assert!(files.contains(&"auth.spec"), "must include reference site");
    assert!(
        files.contains(&"session.spec"),
        "must include reference site"
    );
}

// B:goto_import_definition — verify contract "requires/ensures consistency for import go-to-definition"
#[specforge_test(
    behavior = "goto_import_definition",
    verify = "Go-to-Definition on Imports: import go-to-definition holds — imports_resolved, target_file_navigated"
)]
fn goto_import_definition_contract() {
    // Requires: use import path + spec root with target file
    // Ensures: resolves to target file location; None for missing
    let tmp = tempfile::tempdir().unwrap();
    let behaviors_dir = tmp.path().join("behaviors");
    std::fs::create_dir_all(&behaviors_dir).unwrap();
    std::fs::write(
        behaviors_dir.join("auth.spec"),
        "behavior auth \"Auth\" {}\n",
    )
    .unwrap();

    let spec_root = tmp.path().to_str().unwrap();

    let result = specforge_lsp::goto_import_definition("behaviors/auth", spec_root);
    let loc = result.expect("valid import path must resolve");
    assert!(
        loc.file.as_str().ends_with("behaviors/auth.spec"),
        "must resolve to correct file"
    );

    let missing = specforge_lsp::goto_import_definition("nonexistent/path", spec_root);
    assert!(missing.is_none(), "missing import must return None");
}

// B:prepare_rename — verify contract "requires/ensures consistency for prepare rename"
#[specforge_test(
    behavior = "prepare_rename",
    verify = "Prepare Rename: prepare rename holds — graph_available, token_range_returned, non_renameable_rejected"
)]
fn prepare_rename_contract() {
    // Requires: entity ID in graph
    // Ensures: returns token range for existing entity; None for missing
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types.spec", 5, 5));

    let result = specforge_lsp::prepare_rename(&g, "auth_token");
    let range = result.expect("existing entity must return range");
    assert_eq!(range.file, "types.spec");
    assert_eq!(range.start_line, 5);
    assert_eq!(range.start_col, 5);

    let missing = specforge_lsp::prepare_rename(&g, "nonexistent");
    assert!(missing.is_none(), "missing entity must return None");
}

// B:rename_entity_id — verify contract "requires/ensures consistency for entity rename"
#[specforge_test(
    behavior = "rename_entity_id",
    verify = "Rename Entity ID: entity rename holds — graph_available, prepare_rename_ready, all_references_updated, rename_atomic, entity_renamed_emitted"
)]
fn rename_entity_id_contract() {
    // Requires: entity in graph with references from other entities + new name
    // Ensures: edits for declaration + all reference sites; rejects duplicate name
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types.spec", 5, 5));
    g.add_node(node_at("user_login", "behavior", "auth.spec", 10, 9));
    g.add_edge(Edge {
        source: "user_login".into(),
        target: "auth_token".into(),
        label: "types".into(),
    });

    // Each node's id on its first line; user_login's line also names what
    // it references.
    let texts: std::collections::HashMap<&str, String> = [
        ("types.spec", format!("{}     auth_token\n", "\n".repeat(4))),
        (
            "auth.spec",
            format!("{}         user_login [auth_token]\n", "\n".repeat(9)),
        ),
    ]
    .into();
    let text_of = |f: &str| texts.get(f).cloned();
    let edits = specforge_lsp::identifier_edits(&g, "auth_token", "session_token", text_of);
    let edits = edits.expect("valid rename must produce edits");
    assert!(edits.len() >= 2, "must edit declaration + reference sites");
    assert!(
        edits.iter().any(|e| e.file == "types.spec"),
        "must edit declaration file"
    );
    assert!(
        edits.iter().any(|e| e.file == "auth.spec"),
        "must edit reference file"
    );

    // Reject rename to existing ID
    let dup = specforge_lsp::identifier_edits(&g, "auth_token", "user_login", text_of);
    assert!(dup.is_none(), "rename to existing ID must be rejected");
}

// B:outline_view — verify contract "requires/ensures consistency for outline view"
#[specforge_test(
    behavior = "outline_view",
    verify = "Outline View: outline view holds — graph_available, kind_registry_available, all_entities_listed, symbol_kind_delegated"
)]
fn outline_view_contract() {
    // Requires: graph with entities across files
    // Ensures: document_symbols returns entities in the specified file with kind, id, title
    let mut g = Graph::new();
    g.add_node(node_at("a", "behavior", "test.spec", 0, 0));
    g.add_node(node_at("b", "type", "test.spec", 5, 0));
    g.add_node(node_at("c", "event", "other.spec", 10, 0));

    let symbols = specforge_lsp::document_symbols(&g, "test.spec");

    assert_eq!(symbols.len(), 2, "only entities from target file");
    for sym in &symbols {
        assert!(!sym.kind.is_empty(), "each symbol must have kind");
        assert!(!sym.id.is_empty(), "each symbol must have id");
    }
}

// B:workspace_symbol_search — verify contract "requires/ensures consistency for workspace symbol search"
#[specforge_test(
    behavior = "workspace_symbol_search",
    verify = "Workspace Symbol Search: workspace symbol search holds — graph_available, kind_registry_available, matching_entities_returned, symbol_kind_delegated"
)]
fn workspace_symbol_search_contract() {
    // Requires: graph with entities + search query
    // Ensures: results match by ID prefix or title fragment with kind
    let mut g = Graph::new();
    g.add_node(node_at("user_login", "behavior", "a.spec", 0, 0));
    g.add_node(node_at("user_logout", "behavior", "a.spec", 5, 0));
    g.add_node(node_at("auth_token", "type", "b.spec", 0, 0));

    let by_prefix = specforge_lsp::workspace_symbols(&g, "user");
    assert_eq!(by_prefix.len(), 2, "ID prefix search must match");

    let by_title = specforge_lsp::workspace_symbols(&g, "Auth");
    assert_eq!(by_title.len(), 1, "title fragment search must match");
    assert_eq!(by_title[0].kind, "type", "result must include kind");
}

// B:provide_semantic_tokens — verify contract "requires/ensures consistency for semantic tokens"
#[specforge_test(
    behavior = "provide_semantic_tokens",
    verify = "Provide Semantic Tokens: semantic tokens holds — graph_available, kind_registry_available, tokens_classified, structural_keywords_enforced, extension_delegation_applied"
)]
fn provide_semantic_tokens_contract() {
    // Requires: source text + registered kinds
    // Ensures: tokens classified with correct types (keyword, property, string for triple-quoted)
    let source = "behavior foo \"Foo\" {\n  contract \"\"\"\n    hello\n  \"\"\"\n}\n";
    let tokens = specforge_lsp::classify_tokens(source, &verifiable(&["behavior"], &[]));

    assert!(!tokens.is_empty(), "must produce tokens");
    assert!(
        tokens
            .iter()
            .any(|t| t.text == "behavior" && t.token_type == "type"),
        "entity keyword must be classified as type"
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.text == "contract" && t.token_type == "property"),
        "field names must be classified as property"
    );
    assert!(
        tokens.iter().any(|t| t.token_type == "string"),
        "triple-quoted strings must be classified as string"
    );
}

const TOKENS_REFRESH: &str = "workspace/semanticTokens/refresh";

/// `didChange` params replacing `uri`'s whole text.
fn replace_all(uri: &str, version: i32, text: &str) -> serde_json::Value {
    serde_json::json!({
        "textDocument": {"uri": uri, "version": version},
        "contentChanges": [{"text": text}],
    })
}

/// A session whose client declares `workspace.semanticTokens.refreshSupport`
/// as `refresh_support`, with `text` open at `uri` and its first compile done.
async fn session_with_open(refresh_support: bool, uri: &str, text: &str) -> wire::Session {
    let caps = serde_json::json!({
        "workspace": {"semanticTokens": {"refreshSupport": refresh_support}},
    });
    let (mut session, _) = wire::Session::start_with_capabilities(None, caps).await;
    session.open(uri, text).await;
    session.diagnostics(uri).await;
    session
}

const LOGIN: &str = "behavior login \"Login\" {\n  contract \"x\"\n}\n";

#[specforge_test(
    behavior = "provide_semantic_tokens",
    verify = "a recompile that changes the graph asks the client to refresh semantic tokens"
)]
#[tokio::test]
async fn graph_changing_recompile_requests_token_refresh() {
    let uri = "file:///buffer/refresh.spec";
    let mut session = session_with_open(true, uri, LOGIN).await;
    // Opening compiled `login` into an empty graph: that is a change too.
    assert!(
        session
            .notification(TOKENS_REFRESH, |_| true)
            .await
            .is_some(),
        "the first compile of an entity must ask for a refresh"
    );

    // didChange adds an entity: the recompiled graph differs.
    let grown = format!("{LOGIN}\ninvariant quota \"Quota\" {{\n}}\n");
    session
        .notify("textDocument/didChange", replace_all(uri, 2, &grown))
        .await;
    session.diagnostics(uri).await;
    assert!(
        session
            .notification(TOKENS_REFRESH, |_| true)
            .await
            .is_some(),
        "adding an entity must ask the client to refresh semantic tokens"
    );
}

#[specforge_test(
    behavior = "provide_semantic_tokens",
    verify = "a recompile that changes nothing token-relevant sends no semantic token refresh"
)]
#[tokio::test]
async fn whitespace_only_recompile_sends_no_token_refresh() {
    let uri = "file:///buffer/whitespace.spec";
    let mut session = session_with_open(true, uri, LOGIN).await;
    session.notification(TOKENS_REFRESH, |_| true).await;

    // Same entities, kinds and titles; only the layout moves.
    let spaced = format!("\n\n{}", LOGIN.replace("  contract", "      contract"));
    session
        .notify("textDocument/didChange", replace_all(uri, 2, &spaced))
        .await;
    session.diagnostics(uri).await;
    let refresh = session
        .notification_within(
            TOKENS_REFRESH,
            std::time::Duration::from_millis(500),
            |_| true,
        )
        .await;
    assert!(refresh.is_none(), "a whitespace-only edit must not refresh");

    // A retitle does change what is highlighted: it refreshes.
    session
        .notify(
            "textDocument/didChange",
            replace_all(uri, 3, &spaced.replace("\"Login\"", "\"Sign in\"")),
        )
        .await;
    session.diagnostics(uri).await;
    assert!(
        session
            .notification(TOKENS_REFRESH, |_| true)
            .await
            .is_some(),
        "a changed title must ask for a refresh"
    );
}

#[specforge_test(
    behavior = "provide_semantic_tokens",
    verify = "no semantic token refresh is sent to a client without refreshSupport"
)]
#[tokio::test]
async fn client_without_refresh_support_never_gets_token_refresh() {
    let uri = "file:///buffer/no_refresh.spec";
    let mut session = session_with_open(false, uri, LOGIN).await;

    let grown = format!("{LOGIN}\ninvariant quota \"Quota\" {{\n}}\n");
    session
        .notify("textDocument/didChange", replace_all(uri, 2, &grown))
        .await;
    session.diagnostics(uri).await;
    let refresh = session
        .notification_within(
            TOKENS_REFRESH,
            std::time::Duration::from_millis(500),
            |_| true,
        )
        .await;
    assert!(
        refresh.is_none(),
        "a client that did not declare refreshSupport must never be asked"
    );
}

// B:code_action_create_entity_stub — verify contract "requires/ensures consistency for create entity stub"
#[specforge_test(
    behavior = "code_action_create_entity_stub",
    verify = "Code Action: Create Entity Stub: create entity stub holds — graph_available, field_registry_available, stub_created, kind_inferred, no_code_generated"
)]
fn code_action_create_entity_stub_contract() {
    // Requires: missing entity ID + target kind from FieldRegistry
    // Ensures: stub with correct kind inserted in current file; None without target_kind
    let action =
        specforge_lsp::code_action_create_stub("missing_event", Some("event"), "current.spec");
    let action = action.expect("entity stub must be created with target_kind");
    assert!(
        action.edit_text.contains("event missing_event"),
        "stub must use correct kind and ID"
    );
    assert_eq!(action.file, "current.spec", "stub must target current file");

    let no_kind = specforge_lsp::code_action_create_stub("unknown", None, "current.spec");
    assert!(no_kind.is_none(), "must return None without target_kind");
}

// B:code_actions_for_missing_verify — verify contract "requires/ensures consistency for missing verify code actions"
/// A registry where `kinds` accept verify statements of `verify_kinds`.
fn verifiable(kinds: &[&str], verify_kinds: &[&str]) -> specforge_registry::KindRegistry {
    let mut registry = specforge_registry::KindRegistry::new();
    for kind in kinds {
        registry.register(specforge_registry::KindRegistryEntry {
            kind_name: kind.to_string(),
            description: None,
            source_extension: "@test/ext".into(),
            testable: true,
            singleton: false,
            supports_verify: true,
            allowed_verify_kinds: verify_kinds.iter().map(|k| k.to_string()).collect(),
            has_body_parser: false,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            open_fields: false,
        });
    }
    registry
}

#[specforge_test(
    behavior = "code_actions_for_missing_verify",
    verify = "Code Actions for Missing Verify: missing verify code actions holds — kind_registry_available, graph_available, quickfix_offered, verify_stubs_produced, no_code_generated"
)]
fn code_actions_for_missing_verify_contract() {
    // Requires: testable entity without verify statements
    // Ensures: quickfix code action with verify stub targeting the .spec file
    let mut g = Graph::new();
    g.add_node(node_at("my_behavior", "behavior", "a.spec", 5, 0));

    let actions =
        specforge_lsp::code_actions_missing_verify(&g, "a.spec", &verifiable(&["behavior"], &[]));

    assert!(
        !actions.is_empty(),
        "untested testable entity must produce code action"
    );
    assert_eq!(actions[0].entity_id, "my_behavior");
    assert_eq!(
        actions[0].action_kind, "quickfix",
        "must be quickfix action"
    );
    assert!(
        actions[0].edit_text.contains("verify unit"),
        "stub must include verify statement"
    );
    assert!(
        actions[0].file.ends_with(".spec"),
        "edit must target .spec file"
    );
}

// B:go_to_definition — verify contract "requires/ensures consistency for go-to-definition"
#[specforge_test(
    behavior = "go_to_definition",
    verify = "Go-to-Definition: go-to-definition holds — graph_available, declaration_site_returned"
)]
fn go_to_definition_contract() {
    // Requires: graph with resolved entity declarations
    // Ensures: declaration site (file, line, col) returned for existing entity; None for missing
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types.spec", 10, 5));
    g.add_node(node_at("login", "behavior", "auth.spec", 3, 0));
    g.add_edge(Edge {
        source: "login".into(),
        target: "auth_token".into(),
        label: "types".into(),
    });

    let loc = specforge_lsp::go_to_definition(&g, "auth_token");
    let loc = loc.expect("existing entity must return declaration site");
    assert_eq!(loc.file, "types.spec", "must return correct file");
    assert_eq!(loc.start_line, 10, "must return correct line");
    assert_eq!(loc.start_col, 5, "must return correct column");

    let missing = specforge_lsp::go_to_definition(&g, "nonexistent");
    assert!(missing.is_none(), "missing entity must return None");
}

// B:incremental_document_sync — verify contract "requires/ensures consistency for incremental document sync"
#[specforge_test(
    behavior = "incremental_document_sync",
    verify = "Incremental Document Sync: incremental document sync holds — lsp_initialized_fired, document_open, buffer_consistent, partial_update_applied"
)]
fn incremental_document_sync_contract() {
    // Requires: LSP initialized with INCREMENTAL sync, document open
    // Ensures: buffer consistent after partial update; only changed range applied
    let mut buf = specforge_lsp::DocumentBuffer::new(
        "file:///test.spec".into(),
        "behavior foo \"Foo\" {\n  contract \"old\"\n}\n".into(),
    );

    // Apply partial change: only replace "old" with "new"
    buf.apply_change(1, 12, 1, 15, "new");
    assert_eq!(
        buf.content(),
        "behavior foo \"Foo\" {\n  contract \"new\"\n}\n",
        "buffer must reflect incremental change"
    );

    // Apply another partial change at a different location
    buf.apply_change(0, 9, 0, 12, "bar");
    assert_eq!(
        buf.content(),
        "behavior bar \"Foo\" {\n  contract \"new\"\n}\n",
        "buffer must reflect second incremental change"
    );
}

// B:live_diagnostics — verify contract "requires/ensures consistency for live diagnostics"
#[specforge_test(
    behavior = "emit_live_diagnostics",
    verify = "Live Diagnostics: live diagnostics holds — lsp_initialized_fired, graph_available, diagnostics_pushed, latency_enforced"
)]
#[tokio::test]
async fn live_diagnostics_contract() {
    // lsp_initialized_fired, graph_available: an initialized session with
    // a document compiled into the graph, cleanly.
    let (mut session, _) = wire::Session::start(None).await;
    let uri = "file:///buffer/live.spec";
    let text = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n\n\
                invariant session_limit \"Limit\" {\n}\n";
    session.open(uri, text).await;
    let clean = session.diagnostics(uri).await;
    assert!(clean.is_empty(), "{clean:?}");

    // diagnostics_pushed: an edit renaming the invariant (line 4, columns
    // 10..23) leaves the reference dangling; the recompiled diagnostics
    // are pushed without being asked for.
    let edit = |version: i32, new_id: &str, old_len: u32| {
        serde_json::json!({
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{
                "range": {
                    "start": {"line": 4, "character": 10},
                    "end": {"line": 4, "character": 10 + old_len},
                },
                "text": new_id,
            }],
        })
    };
    let typed = std::time::Instant::now();
    session
        .notify("textDocument/didChange", edit(2, "quota", 13))
        .await;
    let broken = session.diagnostics(uri).await;
    // latency_enforced: squiggles within 100ms of the last keystroke.
    let latency = typed.elapsed();
    assert_eq!(wire::codes(&broken), ["E003"], "{broken:?}");
    assert_eq!(
        broken[0]["message"],
        "unresolved reference 'session_limit' in entity 'login'"
    );
    assert!(latency.as_millis() <= 100, "diagnostics took {latency:?}");

    // Every change is recompiled: restoring the ID clears them again.
    session
        .notify("textDocument/didChange", edit(3, "session_limit", 5))
        .await;
    let fixed = session.diagnostics(uri).await;
    assert!(fixed.is_empty(), "{fixed:?}");
}

#[test]
fn shared_incremental_pipeline_contract() {
    // Requires: incremental_rebuild_complete event has fired
    // Ensures: shared graph updated, diagnostics pushed, pipeline parity enforced
    let mut state = specforge_lsp::LspState::new();

    // Simulate pipeline: open doc, build graph, push diagnostics
    state.open_document("file:///a.spec", "behavior a \"A\" {}\n");

    state.graph_mut().add_node(specforge_graph::Node {
        id: specforge_parser::EntityId { raw: "a".into() },
        kind: specforge_parser::EntityKind {
            raw: "behavior".into(),
        },
        title: Some("A".into()),
        fields: specforge_parser::FieldMap::new(),
        source_span: specforge_common::SourceSpan {
            file: "a.spec".into(),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
        methods: Vec::new(),
    });

    // Graph is shared: navigation works on the same graph instance
    let def = specforge_lsp::go_to_definition(state.graph(), "a");
    assert!(def.is_some(), "shared graph must serve navigation");

    // Diagnostics pushed through the shared state
    state.set_diagnostics("file:///a.spec", vec![]);
    assert!(
        state.diagnostics("file:///a.spec").is_empty(),
        "diagnostics must be pushable"
    );
}

/// A JSON-RPC session with an in-process server that keeps every message
/// the server sends, so tests can assert on published diagnostics and log
/// messages (the e2e client reads past them).
pub(crate) mod wire {
    use serde_json::{Value, json};
    use std::path::Path;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
    use tokio::task::JoinHandle;
    use tower_lsp::{LspService, Server};

    pub(crate) struct Session {
        writer: DuplexStream,
        reader: DuplexStream,
        next_id: i64,
        server: JoinHandle<()>,
        /// Server notifications and server-to-client requests (already
        /// answered) not yet taken by `notification`.
        pending: Vec<Value>,
    }

    impl Drop for Session {
        fn drop(&mut self) {
            self.server.abort();
        }
    }

    /// The `file://` URI of `path`.
    pub(crate) fn uri_of(path: &Path) -> String {
        tower_lsp::lsp_types::Url::from_file_path(path)
            .unwrap()
            .to_string()
    }

    impl Session {
        /// Start a server, send `initialize` (with `root` as rootUri) and
        /// `initialized`, and wait until workspace indexing has ended.
        /// Returns the session and the `initialize` result.
        pub(crate) async fn start(root: Option<&Path>) -> (Session, Value) {
            Self::start_with_capabilities(root, json!({})).await
        }

        /// [`Self::start`] with the client declaring `capabilities` in
        /// its `initialize` request.
        pub(crate) async fn start_with_capabilities(
            root: Option<&Path>,
            capabilities: Value,
        ) -> (Session, Value) {
            let (client_to_server, server_stdin) = tokio::io::duplex(1 << 20);
            let (server_stdout, server_to_client) = tokio::io::duplex(1 << 20);
            let (service, socket) = LspService::new(specforge_lsp::backend::Backend::new);
            let server = tokio::spawn(async move {
                Server::new(server_stdin, server_stdout, socket)
                    .serve(service)
                    .await;
            });
            let mut session = Session {
                writer: client_to_server,
                reader: server_to_client,
                next_id: 1,
                server,
                pending: Vec::new(),
            };
            let root_uri = root.map(uri_of);
            let init = session
                .request(
                    "initialize",
                    json!({"processId": null, "rootUri": root_uri, "capabilities": capabilities}),
                )
                .await;
            session.notify("initialized", json!({})).await;
            session
                .notification("$/progress", |p| p["value"]["kind"] == "end")
                .await
                .expect("workspace indexing never ended");
            (session, init["result"].clone())
        }

        async fn write(&mut self, msg: &Value) {
            let body = serde_json::to_string(msg).unwrap();
            let frame = format!("Content-Length: {}\r\n\r\n{body}", body.len());
            self.writer.write_all(frame.as_bytes()).await.unwrap();
            self.writer.flush().await.unwrap();
        }

        /// The next message from the server, answering server-to-client
        /// requests (registrations, progress tokens) with a null result.
        async fn read(&mut self) -> Value {
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut byte = [0u8; 1];
                self.reader.read_exact(&mut byte).await.unwrap();
                header.push(byte[0]);
            }
            let header = String::from_utf8(header).unwrap();
            let length: usize = header
                .lines()
                .find_map(|l| l.strip_prefix("Content-Length:"))
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            let mut body = vec![0u8; length];
            self.reader.read_exact(&mut body).await.unwrap();
            let msg: Value = serde_json::from_slice(&body).unwrap();
            if msg.get("method").is_some() && msg.get("id").is_some() {
                let reply = json!({"jsonrpc": "2.0", "id": msg["id"], "result": null});
                self.write(&reply).await;
            }
            msg
        }

        /// Send a request and return its response; notifications that
        /// arrive meanwhile are kept for `notification`.
        pub(crate) async fn request(&mut self, method: &str, params: Value) -> Value {
            let id = self.next_id;
            self.next_id += 1;
            let mut msg = json!({"jsonrpc": "2.0", "id": id, "method": method});
            if !params.is_null() {
                msg["params"] = params;
            }
            self.write(&msg).await;
            loop {
                let msg = self.read().await;
                if msg.get("method").is_none() && msg["id"] == id {
                    return msg;
                }
                if msg.get("method").is_some() {
                    self.pending.push(msg);
                }
            }
        }

        pub(crate) async fn notify(&mut self, method: &str, params: Value) {
            let msg = json!({"jsonrpc": "2.0", "method": method, "params": params});
            self.write(&msg).await;
        }

        /// The params of the first `method` notification (or answered
        /// server-to-client request) matching `pred`, kept or arriving
        /// within `wait`; it is taken, the others kept.
        pub(crate) async fn notification_within(
            &mut self,
            method: &str,
            wait: Duration,
            pred: impl Fn(&Value) -> bool,
        ) -> Option<Value> {
            let hit = |m: &Value| m["method"] == method && pred(&m["params"]);
            if let Some(i) = self.pending.iter().position(hit) {
                return Some(self.pending.remove(i)["params"].clone());
            }
            let deadline = tokio::time::Instant::now() + wait;
            loop {
                let msg = tokio::time::timeout_at(deadline, self.read()).await.ok()?;
                if hit(&msg) {
                    return Some(msg["params"].clone());
                }
                if msg.get("method").is_some() {
                    self.pending.push(msg);
                }
            }
        }

        /// [`Self::notification_within`] ten seconds.
        pub(crate) async fn notification(
            &mut self,
            method: &str,
            pred: impl Fn(&Value) -> bool,
        ) -> Option<Value> {
            self.notification_within(method, Duration::from_secs(10), pred)
                .await
        }

        /// The next diagnostics published for `uri`.
        pub(crate) async fn diagnostics(&mut self, uri: &str) -> Vec<Value> {
            let params = self
                .notification("textDocument/publishDiagnostics", |p| p["uri"] == uri)
                .await
                .unwrap_or_else(|| panic!("no diagnostics published for {uri}"));
            params["diagnostics"].as_array().unwrap().clone()
        }

        pub(crate) async fn open(&mut self, uri: &str, text: &str) {
            let doc = json!({"uri": uri, "languageId": "specforge", "version": 1, "text": text});
            self.notify("textDocument/didOpen", json!({"textDocument": doc}))
                .await;
        }

        pub(crate) async fn close(&mut self, uri: &str) {
            self.notify(
                "textDocument/didClose",
                json!({"textDocument": {"uri": uri}}),
            )
            .await;
        }

        /// `textDocument/formatting`, which only serves open documents.
        pub(crate) async fn format(&mut self, uri: &str) -> Value {
            let params = json!({
                "textDocument": {"uri": uri},
                "options": {"tabSize": 2, "insertSpaces": true},
            });
            self.request("textDocument/formatting", params).await["result"].clone()
        }
    }

    /// The codes of `diagnostics`.
    pub(crate) fn codes(diagnostics: &[Value]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|d| d["code"].as_str().unwrap_or(""))
            .collect()
    }
}

/// Build a Wasm runtime for a temp project listing `ext_names`, mirroring
/// how a real session loads extensions from specforge.json.
fn wasm_runtime_for(ext_names: &[String]) -> specforge_component::ComponentRuntime {
    let dir = tempfile::TempDir::new().unwrap();
    let config = serde_json::json!({
        "name": "test-project",
        "version": "0.1.0",
        "extensions": ext_names,
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    specforge_component::project_runtime(dir.path())
}
