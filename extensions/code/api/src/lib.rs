//! `specforge-code-api`: the API snapshot readers the api packs link (ADR 0388).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so an api pack reads its snapshot here; `specforge-code-facts` mints the `api_item` facts from
//! what it reads ([`specforge_code_facts::api_facts`]). No format is sniffed: a pack calls the reader of its own.
//!
//! - [`read_rust_text`] reads a `cargo public-api` text snapshot into one [`ApiEntry`] per public entry.
//! - [`read_js_model`] reads an api-extractor `.api.json` doc model into one [`ApiEntry`] per member.

#[cfg(test)]
mod tests;

mod js_model;
mod rust_text;

pub use js_model::read_js_model;
pub use rust_text::read_rust_text;
pub use specforge_code_facts::ApiEntry;
