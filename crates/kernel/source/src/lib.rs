//! The host's one source reader (ADR 0344): tree-sitter with the rust, typescript and tsx grammars, running a
//! pack's query in the tags convention and answering every match with its role, kind, span, parent and named
//! captures, and each file's error count.
//!
//! The crate knows grammars, not items: what a function or an item id is lives in the packs' queries and code. Guests
//! reach it through `specforge:host/syntax` (ADR 0345), served by `specforge-wasm`'s port session.

mod language;
mod parse;
mod query;

pub use language::Language;
pub use parse::CAPTURE_TEXT_CAP;
pub use query::{QueryFault, SourceQuery};
