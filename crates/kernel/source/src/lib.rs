//! The host's one source reader (ADR 0344): tree-sitter with the rust, typescript and tsx grammars, running a
//! pack's query in the tags convention and answering every match with its role, kind, span, parent and named
//! captures, and each file's error count. A [`SourceReader`] reads many files on scoped threads through a
//! content-addressed [`SyntaxCache`] and counts its [`SyntaxWork`].
//!
//! The crate knows grammars, not items: what a function or an item id is lives in the packs' queries and code. Guests
//! reach it through `specforge:host/syntax` (ADR 0345), served by `specforge-wasm`'s port session.

mod cache;
mod language;
mod parse;
mod query;
mod reader;

pub use cache::{CACHE_DIR, CACHE_MAX_AGE, SyntaxCache};
pub use language::Language;
pub use parse::{CAPTURE_TEXT_CAP, Deadline, PARSE_DEADLINE};
pub use query::{QueryFault, SourceQuery};
pub use reader::{Loaded, Read, SourceReader, SyntaxWork};
