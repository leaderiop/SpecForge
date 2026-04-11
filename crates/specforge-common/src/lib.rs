mod diagnostic;
mod interner;
mod project;
mod span;
mod suggest;

pub use diagnostic::{Diagnostic, DiagnosticsExt, Severity};
pub use interner::Sym;
pub use project::{find_project_root, load_project_config, ProjectConfig};
pub use span::SourceSpan;
pub use suggest::find_close_match;
