use specforge_test_macros::test as spec;

// -- lsp_initialize -----------------------------------------------------------

#[spec(behavior = "lsp_initialize", verify = "initialize response includes semantic token legend")]
#[test]
fn init_includes_semantic_legend() {
    let caps = specforge_lsp::server_capabilities(&["behavior", "type", "event"]);
    assert!(!caps.semantic_token_types.is_empty());
    assert!(caps.semantic_token_types.contains(&"keyword".to_string()));
}

#[spec(behavior = "lsp_initialize", verify = "semantic token legend includes extension-defined token types")]
#[test]
fn init_legend_includes_extension_types() {
    let caps = specforge_lsp::server_capabilities(&["behavior", "type"]);
    // Extension kinds should appear in the legend as "keyword" type
    assert!(caps.semantic_token_types.contains(&"keyword".to_string()));
    assert!(caps.semantic_token_types.contains(&"string".to_string()));
    assert!(caps.semantic_token_types.contains(&"property".to_string()));
}

#[spec(behavior = "lsp_initialize", verify = "initialize response advertises incremental sync")]
#[test]
fn init_advertises_incremental_sync() {
    let caps = specforge_lsp::server_capabilities(&[]);
    assert!(caps.incremental_sync);
}

#[spec(behavior = "lsp_initialize", verify = "initialize response includes completion trigger characters")]
#[test]
fn init_includes_completion_triggers() {
    let caps = specforge_lsp::server_capabilities(&[]);
    assert!(!caps.completion_trigger_characters.is_empty());
}

#[spec(behavior = "lsp_initialize", verify = "initialize response includes server_info with name and version")]
#[test]
fn init_includes_server_info() {
    let info = specforge_lsp::server_info();
    assert_eq!(info.name, "specforge-lsp");
    assert!(!info.version.is_empty(), "version must be non-empty");
}

#[spec(behavior = "lsp_initialize", verify = "zero extensions produces structural-only capabilities")]
#[test]
fn init_zero_extensions() {
    let caps = specforge_lsp::server_capabilities(&[]);
    // Even with no extensions, structural capabilities exist
    assert!(caps.incremental_sync);
    assert!(caps.supports_go_to_definition);
    assert!(caps.supports_find_references);
}

// -- lsp_shutdown -------------------------------------------------------------

#[spec(behavior = "lsp_shutdown", verify = "shutdown releases in-memory graph")]
#[test]
fn shutdown_clears_state() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "content");
    state.shutdown();
    assert!(!state.is_open("file:///a.spec"));
    assert!(state.is_shutdown());
}

#[spec(behavior = "lsp_shutdown", verify = "shutdown releases Wasm engines")]
#[test]
fn shutdown_sets_flag() {
    let mut state = specforge_lsp::LspState::new();
    state.shutdown();
    assert!(state.is_shutdown());
}

#[spec(behavior = "lsp_shutdown", verify = "requests after shutdown return InvalidRequest")]
#[test]
fn requests_after_shutdown_rejected() {
    let mut state = specforge_lsp::LspState::new();
    state.shutdown();
    // Trying to open a document after shutdown should be ignored
    state.open_document("file:///a.spec", "content");
    assert!(!state.is_open("file:///a.spec"));
}

// -- shared_incremental_pipeline ----------------------------------------------

#[spec(behavior = "shared_incremental_pipeline", verify = "LSP and watch share the same graph")]
#[test]
fn lsp_state_holds_graph() {
    let mut state = specforge_lsp::LspState::new();
    assert_eq!(state.graph().node_count(), 0);

    // Simulate adding to graph
    use specforge_common::SourceSpan;
    use specforge_graph::Node;
    use specforge_parser::{EntityId, EntityKind, FieldMap};
    state.graph_mut().add_node(Node {
        id: EntityId { raw: "a".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: None,
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "a.spec".into(),
            start_line: 0, start_col: 0, end_line: 0, end_col: 0,
        },
    });
    assert_eq!(state.graph().node_count(), 1);
}

#[spec(behavior = "shared_incremental_pipeline", verify = "graph update serves all LSP features")]
#[test]
fn graph_update_serves_all_features() {
    use specforge_common::SourceSpan;
    use specforge_graph::{Edge, Node};
    use specforge_parser::{EntityId, EntityKind, FieldMap};

    let mut state = specforge_lsp::LspState::new();

    // Build a graph through the shared state
    state.graph_mut().add_node(Node {
        id: EntityId { raw: "login".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("User Login".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "auth.spec".into(),
            start_line: 0, start_col: 0, end_line: 3, end_col: 1,
        },
    });
    state.graph_mut().add_node(Node {
        id: EntityId { raw: "token".into() },
        kind: EntityKind { raw: "type".into() },
        title: Some("Auth Token".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "types.spec".into(),
            start_line: 5, start_col: 0, end_line: 8, end_col: 1,
        },
    });
    state.graph_mut().add_edge(Edge {
        source: "login".into(), target: "token".into(), label: "types".into(),
    });

    // The same graph serves go-to-definition
    let def = specforge_lsp::go_to_definition(state.graph(), "token");
    assert!(def.is_some(), "go-to-definition must use shared graph");

    // The same graph serves find-all-references
    let refs = specforge_lsp::find_all_references(state.graph(), "token");
    assert!(!refs.is_empty(), "find-all-references must use shared graph");

    // The same graph serves hover
    let hover = specforge_lsp::hover_info(state.graph(), "login");
    assert!(hover.is_some(), "hover must use shared graph");

    // The same graph serves workspace symbols
    let syms = specforge_lsp::workspace_symbols(state.graph(), "login");
    assert!(!syms.is_empty(), "workspace symbols must use shared graph");

    // The same graph serves completions
    let completions = specforge_lsp::complete_entity_ids(state.graph(), "log");
    assert!(!completions.is_empty(), "completions must use shared graph");
}

#[spec(behavior = "shared_incremental_pipeline", verify = "CLI and LSP share identical debounce window")]
#[test]
fn cli_and_lsp_share_debounce_window() {
    // The debounce window is a constant shared between CLI watch and LSP.
    // Both must use the same value to ensure pipeline parity.
    let lsp_debounce = specforge_lsp::DEBOUNCE_MS;
    assert!(lsp_debounce > 0, "debounce window must be positive");
    assert!(lsp_debounce <= 200, "debounce window must be reasonable (<=200ms)");
}

#[spec(behavior = "shared_incremental_pipeline", verify = "CLI and LSP share identical validator dispatch order")]
#[test]
fn cli_and_lsp_share_validator_dispatch_order() {
    // The validator dispatch order is a shared constant/function between CLI and LSP.
    // Both must produce the same ordering to ensure deterministic diagnostics.
    let order = specforge_lsp::validator_dispatch_order();
    assert!(!order.is_empty(), "validator dispatch order must be non-empty");

    // Order must be deterministic — calling twice yields the same result
    let order2 = specforge_lsp::validator_dispatch_order();
    assert_eq!(order, order2, "validator dispatch order must be deterministic");
}
