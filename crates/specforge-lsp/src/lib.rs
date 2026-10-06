pub mod backend;
mod capabilities;
pub mod completion;
mod document;
pub mod formatting;
pub mod hover;
mod navigation;
pub mod publish;
mod state;
pub mod watchers;

pub use capabilities::{ServerCapabilities, ServerInfo, server_capabilities, server_info};
pub use completion::{field_snippet, keyword_snippet};
pub use document::{
    CompletionSite, Cursor, Document, EntityAt, LineIndex, MOD_DECLARATION, MOD_REFERENCE, Place,
    SemanticToken, TOKEN_MODIFIERS, TOKEN_TYPES, Target, Word, WordEdit,
};
pub use hover::hover_field_info;
pub use navigation::{goto_import_definition, navigator};
pub use specforge_graph::rename::RenameEdit;
pub use state::LspState;

/// Quiet window the reparse worker waits for before recompiling: the one
/// `specforge watch` uses, so both coalesce edits the same way.
pub const DEBOUNCE_WINDOW: std::time::Duration = specforge_watch::DEFAULT_DEBOUNCE_WINDOW;
