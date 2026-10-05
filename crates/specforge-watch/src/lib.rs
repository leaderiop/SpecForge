//! The file watcher `specforge watch` listens to: debounced batches of
//! `.spec`, `specforge.json` and extension changes (the LSP shares the
//! debounce window). What a change does to the project is the project
//! session's (`specforge_project::ProjectSession`).

mod debounce;
mod watcher;

pub use debounce::{DEFAULT_DEBOUNCE_WINDOW, Debouncer};
pub use watcher::{SpecWatcher, WatchEvent, WatchEventKind};
