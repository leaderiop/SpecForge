//! The file watcher `specforge watch` listens to: debounced batches of the
//! paths changed under a directory (the LSP shares the debounce window).
//! What a path is to the project, and what its change does, is the project
//! session's (`specforge_project::ProjectSession::changes`).

mod debounce;
mod watcher;

pub use debounce::{DEFAULT_DEBOUNCE_WINDOW, Debouncer};
pub use watcher::SpecWatcher;
