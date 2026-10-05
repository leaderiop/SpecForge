pub mod backend;
mod capabilities;
mod code_actions;
mod completion;
mod document;
pub mod formatting;
mod hover;
mod navigation;
mod semantic_tokens;
mod state;
mod symbols;
pub mod watchers;

pub use capabilities::{ServerCapabilities, ServerInfo, server_capabilities, server_info};
pub use code_actions::{
    CodeAction, code_action_create_stub, code_actions_create_stubs, code_actions_from_diagnostics,
    code_actions_missing_verify,
};
pub use completion::{
    CompletionItem, CursorContext, complete_entity_ids, complete_entity_ids_filtered,
    complete_field_names, complete_keywords, cursor_context,
};
pub use completion::{enclosing_block, enclosing_entity_kind, field_snippet, keyword_snippet};
pub use document::DocumentBuffer;
pub use hover::{diagnostic_hover, hover_field_info, hover_info, hover_info_with_registries};
pub use navigation::{find_all_references, go_to_definition, goto_import_definition};
pub use semantic_tokens::{
    MOD_DECLARATION, MOD_REFERENCE, SemanticToken, TOKEN_MODIFIERS, TOKEN_TYPES, byte_col_to_utf16,
    classify_tokens, utf16_len,
};
pub use specforge_graph::rename::{RenameEdit, identifier_edits, prepare_rename};
pub use state::LspState;
pub use symbols::{SymbolEntry, document_symbols, workspace_symbols};

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
/// Text-aware variant: SourceSpan columns are byte offsets (tree-sitter),
/// LSP positions are UTF-16 code units — convert per line using the file's
/// content so non-ASCII text cannot shift ranges (C3-09).
pub fn source_span_to_lsp_range_with_text(
    span: &specforge_common::SourceSpan,
    content: &str,
) -> LspRange {
    use crate::document::byte_col_to_utf16_col;

    let lines: Vec<&str> = content.split('\n').collect();
    let line_text = |n: usize| lines.get(n).copied().unwrap_or("");
    LspRange {
        start_line: span.start_line.saturating_sub(1) as u32,
        start_col: byte_col_to_utf16_col(
            line_text(span.start_line.saturating_sub(1)),
            span.start_col.saturating_sub(1),
        ) as u32,
        end_line: span.end_line.saturating_sub(1) as u32,
        end_col: byte_col_to_utf16_col(
            line_text(span.end_line.saturating_sub(1)),
            span.end_col.saturating_sub(1),
        ) as u32,
    }
}

pub fn source_span_to_lsp_range(span: &specforge_common::SourceSpan) -> LspRange {
    LspRange {
        start_line: span.start_line.saturating_sub(1) as u32,
        start_col: span.start_col.saturating_sub(1) as u32,
        end_line: span.end_line.saturating_sub(1) as u32,
        end_col: span.end_col.saturating_sub(1) as u32,
    }
}

/// Quiet window the reparse worker waits for before recompiling: the one
/// `specforge watch` uses, so both coalesce edits the same way.
pub const DEBOUNCE_WINDOW: std::time::Duration = specforge_watch::DEFAULT_DEBOUNCE_WINDOW;
