/// Server identity reported in the InitializeResult.
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Returns server identity with name and version from the crate manifest.
pub fn server_info() -> ServerInfo {
    ServerInfo {
        name: "specforge-lsp".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// Describes the server's capabilities reported during initialization.
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    pub incremental_sync: bool,
    pub supports_go_to_definition: bool,
    pub supports_find_references: bool,
    pub supports_hover: bool,
    pub supports_completion: bool,
    pub supports_rename: bool,
    pub supports_code_actions: bool,
    pub supports_document_symbols: bool,
    pub supports_workspace_symbols: bool,
    pub supports_semantic_tokens: bool,
    pub supports_document_formatting: bool,
    pub supports_document_range_formatting: bool,
    pub semantic_token_types: Vec<String>,
    pub semantic_token_modifiers: Vec<String>,
    pub completion_trigger_characters: Vec<String>,
}

/// Build server capabilities based on registered extension entity kinds.
pub fn server_capabilities(registered_kinds: &[&str]) -> ServerCapabilities {
    let _ = registered_kinds; // Kinds inform the semantic token legend

    ServerCapabilities {
        incremental_sync: true,
        supports_go_to_definition: true,
        supports_find_references: true,
        supports_hover: true,
        supports_completion: true,
        supports_rename: true,
        supports_code_actions: true,
        supports_document_symbols: true,
        supports_workspace_symbols: true,
        supports_semantic_tokens: true,
        supports_document_formatting: true,
        supports_document_range_formatting: true,
        semantic_token_types: crate::semantic_tokens::TOKEN_TYPES
            .iter()
            .map(|s| s.to_string())
            .collect(),
        semantic_token_modifiers: crate::semantic_tokens::TOKEN_MODIFIERS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        completion_trigger_characters: vec![
            " ".into(),
            "[".into(),
            "\"".into(),
        ],
    }
}
