pub mod backend;
mod capabilities;
pub mod completion;
mod document;
pub mod formatting;
mod hover;
mod navigation;
mod semantic_tokens;
mod state;
pub mod watchers;

pub use capabilities::{ServerCapabilities, ServerInfo, server_capabilities, server_info};
pub use completion::{field_snippet, keyword_snippet};
pub use document::{
    CompletionSite, Cursor, Document, EntityAt, LineIndex, Place, Target, Word, WordEdit,
};
pub use hover::{diagnostic_hover, hover_field_info, hover_info, hover_info_with_registries};
pub use navigation::{goto_import_definition, navigator};
pub use semantic_tokens::{
    MOD_DECLARATION, MOD_REFERENCE, SemanticToken, TOKEN_MODIFIERS, TOKEN_TYPES, classify_tokens,
};
pub use specforge_graph::rename::RenameEdit;
pub use state::LspState;

/// Quiet window the reparse worker waits for before recompiling: the one
/// `specforge watch` uses, so both coalesce edits the same way.
pub const DEBOUNCE_WINDOW: std::time::Duration = specforge_watch::DEFAULT_DEBOUNCE_WINDOW;
