//! The bare algorithms, parsers and pinned tables of generic value checks (ADR 0338 D5), one feature per family, for
//! `@specforge/checks` and the domain packs that build on it.
//!
//! It holds no probes, no roots and no codes: which code a finding carries is the foundation's policy, and where a path
//! leads is the host's alone. A parser here answers whether a text is well formed, and which of its bytes are not.

mod invalid;
pub mod tables;

#[cfg(feature = "digits")]
pub mod digits;
#[cfg(feature = "files")]
pub mod files;
#[cfg(feature = "formats")]
pub mod formats;
#[cfg(feature = "git")]
pub mod git;
#[cfg(any(feature = "links", feature = "structured"))]
pub mod html;
#[cfg(feature = "links")]
pub mod links;
#[cfg(feature = "structured")]
pub mod pointer;
#[cfg(feature = "structured")]
pub mod slug;
#[cfg(feature = "structured")]
pub mod tree;

#[cfg(all(test, feature = "structured"))]
mod structured_tests;
#[cfg(all(
    test,
    feature = "digits",
    feature = "files",
    feature = "git",
    feature = "links"
))]
mod tests;

pub use invalid::Invalid;
pub use tables::TableSource;
