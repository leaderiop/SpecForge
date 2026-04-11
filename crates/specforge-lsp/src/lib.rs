pub mod backend;
mod capabilities;
mod code_actions;
mod completion;
pub mod formatting;
mod grammar_cache;
mod document;
mod hover;
mod navigation;
mod rename;
mod semantic_tokens;
mod state;
mod symbols;

pub use capabilities::{server_capabilities, server_info, ServerCapabilities, ServerInfo};
pub use code_actions::{
    code_action_add_import, code_action_create_stub, code_actions_missing_verify, CodeAction,
};
pub use completion::{
    complete_entity_ids, complete_entity_ids_filtered, complete_field_names, complete_keywords,
    cursor_context, CompletionItem, CursorContext,
};
pub use document::DocumentBuffer;
pub use grammar_cache::GrammarCache;
pub use completion::enclosing_entity_kind;
pub use hover::{hover_field_info, hover_info, hover_info_with_registries};
pub use navigation::{find_all_references, go_to_definition, goto_import_definition};
pub use rename::{compute_rename_edits, prepare_rename, RenameEdit};
pub use semantic_tokens::{
    classify_tokens, SemanticToken, TOKEN_MODIFIERS, TOKEN_TYPES,
    MOD_DECLARATION, MOD_REFERENCE,
};
pub use state::LspState;
pub use symbols::{document_symbols, workspace_symbols, SymbolEntry};

/// An LSP-compatible position range (0-based line and column).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Convert a parser SourceSpan (1-based) to an LSP-compatible range (0-based).
/// The parser stores 1-based lines and columns for human-readable diagnostics,
/// but the LSP protocol requires 0-based positions.
pub fn source_span_to_lsp_range(span: &specforge_common::SourceSpan) -> LspRange {
    LspRange {
        start_line: span.start_line.saturating_sub(1) as u32,
        start_col: span.start_col.saturating_sub(1) as u32,
        end_line: span.end_line.saturating_sub(1) as u32,
        end_col: span.end_col.saturating_sub(1) as u32,
    }
}

/// Debounce window in milliseconds shared between CLI watch and LSP modes.
/// Both pipelines must use the same value for pipeline parity.
pub const DEBOUNCE_MS: u64 = 50;

/// Returns the canonical validator dispatch order shared between CLI and LSP.
/// Both pipelines must dispatch validators in this order for deterministic diagnostics.
pub fn validator_dispatch_order() -> Vec<&'static str> {
    vec!["parse", "resolve", "validate", "emit"]
}
