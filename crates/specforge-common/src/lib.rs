mod diagnostic;
mod span;
mod suggest;

pub use diagnostic::{Diagnostic, Severity};
pub use span::SourceSpan;
pub use suggest::find_close_match;
