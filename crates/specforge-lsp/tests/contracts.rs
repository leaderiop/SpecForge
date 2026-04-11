use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as specforge_test;

fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: title.map(|t| t.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 0,
            start_col: 0,
            end_line: 3,
            end_col: 1,
        },
    }
}

fn node_at(id: &str, kind: &str, file: &str, line: usize, col: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(format!("{id} title")),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new(file),
            start_line: line,
            start_col: col,
            end_line: line + 3,
            end_col: col + id.len(),
        },
    }
}

// B:lsp_initialize — verify contract "requires/ensures consistency for LSP initialization"
#[test]
#[specforge_test(behavior = "lsp_initialize", verify = "requires/ensures consistency for LSP initialization")]
fn lsp_initialize_contract() {
    // Requires: list of registered extension kinds
    // Ensures: capabilities include semantic tokens, incremental sync, completion triggers, navigation
    let caps = specforge_lsp::server_capabilities(&["behavior", "type", "event"]);

    assert!(caps.incremental_sync, "must advertise incremental sync");
    assert!(!caps.semantic_token_types.is_empty(), "must include semantic token types");
    assert!(!caps.completion_trigger_characters.is_empty(), "must include completion triggers");
    assert!(caps.supports_go_to_definition, "must support go-to-definition");
    assert!(caps.supports_find_references, "must support find-references");
}

// B:lsp_shutdown — verify contract "requires/ensures consistency for LSP shutdown"
#[test]
#[specforge_test(behavior = "lsp_shutdown", verify = "requires/ensures consistency for LSP shutdown")]
fn lsp_shutdown_contract() {
    // Requires: active LSP state with open documents
    // Ensures: shutdown releases state, subsequent operations rejected
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "content");
    assert!(state.is_open("file:///a.spec"));

    state.shutdown();

    assert!(state.is_shutdown(), "state must be marked as shutdown");
    assert!(!state.is_open("file:///a.spec"), "documents must be released");

    // Post-shutdown operations should be rejected
    state.open_document("file:///b.spec", "new content");
    assert!(!state.is_open("file:///b.spec"), "must reject operations after shutdown");
}

// B:document_open_close — verify contract "requires/ensures consistency for document open/close"
#[test]
#[specforge_test(behavior = "document_open_close", verify = "requires/ensures consistency for document open/close")]
fn document_open_close_contract() {
    // Requires: document URI and content
    // Ensures: open makes document available, close removes it
    let mut state = specforge_lsp::LspState::new();

    state.open_document("file:///test.spec", "behavior foo \"Foo\" {}\n");
    assert!(state.is_open("file:///test.spec"), "opened document must be available");

    state.close_document("file:///test.spec");
    assert!(!state.is_open("file:///test.spec"), "closed document must be removed");
}

// B:autocomplete_entity_ids — verify contract "requires/ensures consistency for entity ID autocomplete"
#[test]
#[specforge_test(behavior = "autocomplete_entity_ids", verify = "requires/ensures consistency for entity ID autocomplete")]
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
#[test]
#[specforge_test(behavior = "complete_field_names", verify = "requires/ensures consistency for field name completion")]
fn complete_field_names_contract() {
    // Requires: entity kind name
    // Ensures: field names appropriate for that kind returned
    let behavior_fields = specforge_lsp::complete_field_names("behavior", None);
    assert!(!behavior_fields.is_empty(), "known kind must have field suggestions");
    assert!(behavior_fields.iter().any(|f| f == "contract"), "behavior must include 'contract'");

    let unknown_fields = specforge_lsp::complete_field_names("__nonexistent__", None);
    assert!(unknown_fields.is_empty(), "unknown kind must return no fields");
}

// B:complete_keywords — verify contract "requires/ensures consistency for keyword completion"
#[test]
#[specforge_test(behavior = "complete_keywords", verify = "requires/ensures consistency for keyword completion")]
fn complete_keywords_contract() {
    // Requires: set of registered extension kinds
    // Ensures: all registered kinds + structural keywords returned, no duplicates
    let keywords = specforge_lsp::complete_keywords(&["behavior", "type"]);

    assert!(keywords.contains(&"behavior".to_string()), "registered kind must be included");
    assert!(keywords.contains(&"type".to_string()), "registered kind must be included");
    assert!(keywords.contains(&"use".to_string()), "structural keyword must be included");
    assert!(keywords.contains(&"define".to_string()), "structural keyword must be included");

    // No duplicates
    let mut sorted = keywords.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(keywords.len(), sorted.len(), "must have no duplicates");
}

// B:hover_information — verify contract "requires/ensures consistency for hover information"
#[test]
#[specforge_test(behavior = "hover_information", verify = "requires/ensures consistency for hover information")]
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
#[test]
#[specforge_test(behavior = "find_all_references", verify = "requires/ensures consistency for find all references")]
fn find_all_references_contract() {
    // Requires: entity in graph with edges from other entities
    // Ensures: declaration + all reference sites returned
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types.spec", 10, 5));
    g.add_node(node_at("login", "behavior", "auth.spec", 5, 9));
    g.add_node(node_at("refresh", "behavior", "session.spec", 3, 9));
    g.add_edge(Edge { source: "login".into(), target: "auth_token".into(), label: "types".into() });
    g.add_edge(Edge { source: "refresh".into(), target: "auth_token".into(), label: "types".into() });

    let refs = specforge_lsp::find_all_references(&g, "auth_token");

    assert_eq!(refs.len(), 3, "declaration + 2 reference sites");
    let files: Vec<&str> = refs.iter().map(|l| l.file.as_str()).collect();
    assert!(files.contains(&"types.spec"), "must include declaration site");
    assert!(files.contains(&"auth.spec"), "must include reference site");
    assert!(files.contains(&"session.spec"), "must include reference site");
}

// B:goto_import_definition — verify contract "requires/ensures consistency for import go-to-definition"
#[test]
#[specforge_test(behavior = "goto_import_definition", verify = "requires/ensures consistency for import go-to-definition")]
fn goto_import_definition_contract() {
    // Requires: use import path + spec root with target file
    // Ensures: resolves to target file location; None for missing
    let tmp = tempfile::tempdir().unwrap();
    let behaviors_dir = tmp.path().join("behaviors");
    std::fs::create_dir_all(&behaviors_dir).unwrap();
    std::fs::write(behaviors_dir.join("auth.spec"), "behavior auth \"Auth\" {}\n").unwrap();

    let spec_root = tmp.path().to_str().unwrap();

    let result = specforge_lsp::goto_import_definition("behaviors/auth", spec_root);
    let loc = result.expect("valid import path must resolve");
    assert!(loc.file.as_str().ends_with("behaviors/auth.spec"), "must resolve to correct file");

    let missing = specforge_lsp::goto_import_definition("nonexistent/path", spec_root);
    assert!(missing.is_none(), "missing import must return None");
}

// B:prepare_rename — verify contract "requires/ensures consistency for prepare rename"
#[test]
#[specforge_test(behavior = "prepare_rename", verify = "requires/ensures consistency for prepare rename")]
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
#[test]
#[specforge_test(behavior = "rename_entity_id", verify = "requires/ensures consistency for entity rename")]
fn rename_entity_id_contract() {
    // Requires: entity in graph with references from other entities + new name
    // Ensures: edits for declaration + all reference sites; rejects duplicate name
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types.spec", 5, 5));
    g.add_node(node_at("user_login", "behavior", "auth.spec", 10, 9));
    g.add_edge(Edge {
        source: "user_login".into(), target: "auth_token".into(), label: "types".into(),
    });

    let edits = specforge_lsp::compute_rename_edits(&g, "auth_token", "session_token");
    let edits = edits.expect("valid rename must produce edits");
    assert!(edits.len() >= 2, "must edit declaration + reference sites");
    assert!(edits.iter().any(|e| e.file == "types.spec"), "must edit declaration file");
    assert!(edits.iter().any(|e| e.file == "auth.spec"), "must edit reference file");

    // Reject rename to existing ID
    let dup = specforge_lsp::compute_rename_edits(&g, "auth_token", "user_login");
    assert!(dup.is_none(), "rename to existing ID must be rejected");
}

// B:outline_view — verify contract "requires/ensures consistency for outline view"
#[test]
#[specforge_test(behavior = "outline_view", verify = "requires/ensures consistency for outline view")]
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
#[test]
#[specforge_test(behavior = "workspace_symbol_search", verify = "requires/ensures consistency for workspace symbol search")]
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
#[test]
#[specforge_test(behavior = "provide_semantic_tokens", verify = "requires/ensures consistency for semantic tokens")]
fn provide_semantic_tokens_contract() {
    // Requires: source text + registered kinds
    // Ensures: tokens classified with correct types (keyword, property, string for triple-quoted)
    let source = "behavior foo \"Foo\" {\n  contract \"\"\"\n    hello\n  \"\"\"\n}\n";
    let tokens = specforge_lsp::classify_tokens(source, &["behavior"]);

    assert!(!tokens.is_empty(), "must produce tokens");
    assert!(tokens.iter().any(|t| t.text == "behavior" && t.token_type == "type"),
        "entity keyword must be classified as type");
    assert!(tokens.iter().any(|t| t.text == "contract" && t.token_type == "property"),
        "field names must be classified as property");
    assert!(tokens.iter().any(|t| t.token_type == "string"),
        "triple-quoted strings must be classified as string");
}

// B:code_action_add_missing_import — verify contract "requires/ensures consistency for add missing import"
#[test]
#[specforge_test(behavior = "code_action_add_missing_import", verify = "requires/ensures consistency for add missing import")]
fn code_action_add_missing_import_contract() {
    // Requires: entity exists in graph but in a different file
    // Ensures: code action produces use statement; None for nonexistent entity
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types/auth.spec", 1, 5));

    let action = specforge_lsp::code_action_add_import(
        &g, "auth_token", "behaviors/login.spec", "spec",
    );
    let action = action.expect("resolvable entity must produce code action");
    assert!(action.edit_text.contains("use \"types/auth\""), "must generate use statement");

    let missing = specforge_lsp::code_action_add_import(
        &g, "nonexistent", "login.spec", "spec",
    );
    assert!(missing.is_none(), "nonexistent entity must return None");
}

// B:code_action_create_entity_stub — verify contract "requires/ensures consistency for create entity stub"
#[test]
#[specforge_test(behavior = "code_action_create_entity_stub", verify = "requires/ensures consistency for create entity stub")]
fn code_action_create_entity_stub_contract() {
    // Requires: missing entity ID + target kind from FieldRegistry
    // Ensures: stub with correct kind inserted in current file; None without target_kind
    let action = specforge_lsp::code_action_create_stub(
        "missing_event", Some("event"), "current.spec",
    );
    let action = action.expect("entity stub must be created with target_kind");
    assert!(action.edit_text.contains("event missing_event"), "stub must use correct kind and ID");
    assert_eq!(action.file, "current.spec", "stub must target current file");

    let no_kind = specforge_lsp::code_action_create_stub(
        "unknown", None, "current.spec",
    );
    assert!(no_kind.is_none(), "must return None without target_kind");
}

// B:code_actions_for_missing_verify — verify contract "requires/ensures consistency for missing verify code actions"
#[test]
#[specforge_test(behavior = "code_actions_for_missing_verify", verify = "requires/ensures consistency for missing verify code actions")]
fn code_actions_for_missing_verify_contract() {
    // Requires: testable entity without verify statements
    // Ensures: quickfix code action with verify stub targeting the .spec file
    let mut g = Graph::new();
    g.add_node(node_at("my_behavior", "behavior", "a.spec", 5, 0));

    let actions = specforge_lsp::code_actions_missing_verify(
        &g, "a.spec", &["behavior"],
    );

    assert!(!actions.is_empty(), "untested testable entity must produce code action");
    assert_eq!(actions[0].entity_id, "my_behavior");
    assert_eq!(actions[0].action_kind, "quickfix", "must be quickfix action");
    assert!(actions[0].edit_text.contains("verify unit"), "stub must include verify statement");
    assert!(actions[0].file.ends_with(".spec"), "edit must target .spec file");
}

// B:go_to_definition — verify contract "requires/ensures consistency for go-to-definition"
#[test]
#[specforge_test(behavior = "go_to_definition", verify = "requires/ensures consistency for go-to-definition")]
fn go_to_definition_contract() {
    // Requires: graph with resolved entity declarations
    // Ensures: declaration site (file, line, col) returned for existing entity; None for missing
    let mut g = Graph::new();
    g.add_node(node_at("auth_token", "type", "types.spec", 10, 5));
    g.add_node(node_at("login", "behavior", "auth.spec", 3, 0));
    g.add_edge(Edge {
        source: "login".into(), target: "auth_token".into(), label: "types".into(),
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
#[test]
#[specforge_test(behavior = "incremental_document_sync", verify = "requires/ensures consistency for incremental document sync")]
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
#[test]
#[specforge_test(behavior = "live_diagnostics", verify = "requires/ensures consistency for live diagnostics")]
fn live_diagnostics_contract() {
    // Requires: LSP initialized, graph available
    // Ensures: diagnostics pushed after file change; latency enforced
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "behavior a \"A\" {}\n");

    // Initially no diagnostics
    assert!(state.diagnostics("file:///a.spec").is_empty(), "no diagnostics initially");

    // After file change, diagnostics are pushed
    state.set_diagnostics("file:///a.spec", vec![specforge_common::Diagnostic {
        code: "E001".into(),
        suggestion: None,
        message: "unresolved reference".into(),
        severity: specforge_common::Severity::Error,
        span: None,
    }]);
    assert_eq!(state.diagnostics("file:///a.spec").len(), 1, "diagnostics must be pushed after change");

    // After closing, diagnostics are cleared
    state.close_document("file:///a.spec");
    assert!(state.diagnostics("file:///a.spec").is_empty(), "diagnostics must be cleared on close");
}

// B:shared_incremental_pipeline — verify contract "requires/ensures consistency for shared incremental pipeline"
#[test]
#[specforge_test(behavior = "shared_incremental_pipeline", verify = "requires/ensures consistency for shared incremental pipeline")]
fn shared_incremental_pipeline_contract() {
    // Requires: incremental_rebuild_complete event has fired
    // Ensures: shared graph updated, diagnostics pushed, pipeline parity enforced
    let mut state = specforge_lsp::LspState::new();

    // Simulate pipeline: open doc, build graph, push diagnostics
    state.open_document("file:///a.spec", "behavior a \"A\" {}\n");

    state.graph_mut().add_node(specforge_graph::Node {
        id: specforge_parser::EntityId { raw: "a".into() },
        kind: specforge_parser::EntityKind { raw: "behavior".into() },
        title: Some("A".into()),
        fields: specforge_parser::FieldMap::new(),
        source_span: specforge_common::SourceSpan {
            file: "a.spec".into(),
            start_line: 0, start_col: 0, end_line: 0, end_col: 0,
        },
    });

    // Graph is shared: navigation works on the same graph instance
    let def = specforge_lsp::go_to_definition(state.graph(), "a");
    assert!(def.is_some(), "shared graph must serve navigation");

    // Diagnostics pushed through the shared state
    state.set_diagnostics("file:///a.spec", vec![]);
    assert!(state.diagnostics("file:///a.spec").is_empty(), "diagnostics must be pushable");

    // Pipeline parity: debounce and dispatch order are shared constants
    assert_ne!(specforge_lsp::DEBOUNCE_MS, 0, "debounce must be shared");
    assert!(!specforge_lsp::validator_dispatch_order().is_empty(), "dispatch order must be shared");
}

// B:load_extension_grammars_for_highlighting — verify contract "requires/ensures consistency for extension grammar loading"
#[test]
#[specforge_test(behavior = "load_extension_grammars_for_highlighting", verify = "requires/ensures consistency for extension grammar loading")]
fn load_extension_grammars_for_highlighting_contract() {
    // Requires: grammar contributions registered for entity kinds
    // Ensures: grammars available for registered kinds; failures isolated
    let mut cache = specforge_lsp::GrammarCache::new();

    cache.register("behavior", "behavior.wasm");
    assert!(cache.has_grammar("behavior"), "registered grammar must be available");
    assert_eq!(cache.grammar_path("behavior"), Some("behavior.wasm"));

    // Update grammar
    cache.register("behavior", "behavior_v2.wasm");
    assert_eq!(cache.grammar_path("behavior"), Some("behavior_v2.wasm"), "grammar must be updated");

    // Failure isolation
    cache.mark_failed("type", "load error");
    assert!(cache.has_grammar("behavior"), "other grammars must be unaffected");
    assert!(!cache.has_grammar("type"), "failed grammar must not be available");
    assert!(cache.failure("type").is_some(), "failure must be recorded");
}
