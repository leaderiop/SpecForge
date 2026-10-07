mod answers;
mod code_actions;
mod completion;
mod concurrency;
mod contracts;
mod cursor;
mod document;
mod e2e;
mod hover;
mod lifecycle;
mod navigation;
mod protocol;
mod publish;
mod registries;
mod rename;
mod semantic_tokens;
mod session;
mod state;

/// The LSP range from (`start_line`, `start_character`) to (`end_line`,
/// `end_character`): 0-based lines, UTF-16 columns.
pub fn lsp_range(
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
) -> tower_lsp::lsp_types::Range {
    tower_lsp::lsp_types::Range::new(
        tower_lsp::lsp_types::Position::new(start_line, start_character),
        tower_lsp::lsp_types::Position::new(end_line, end_character),
    )
}
