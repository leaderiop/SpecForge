//! The file watcher `specforge watch` listens to ([`SpecWatcher`]) and the
//! one debounce rule watch and the LSP batch changes by ([`Coalescer`], on a
//! std or a tokio channel through [`Debouncer`]). What a path is to the
//! project, and what its change does, is the project session's
//! (`specforge_project::SessionInputs`).

mod coalesce;
mod debounce;
mod watcher;

pub use coalesce::{Coalescer, DEFAULT_DEBOUNCE_WINDOW};
pub use debounce::Debouncer;
pub use watcher::SpecWatcher;
