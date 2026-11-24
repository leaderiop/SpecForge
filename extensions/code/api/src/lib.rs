//! `specforge-code-api`: the API snapshot readers the api packs link (ADR 0388).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so an api pack reads its snapshot here; `specforge-code-facts` mints the `api_item` facts from
//! what it reads ([`specforge_code_facts::api_facts`]). No format is sniffed: a pack calls the reader of its own.
//!
//! - [`read_rust_text`] reads a `cargo public-api` text snapshot into one [`ApiEntry`] per public entry.
//! - [`read_js_model`] reads an api-extractor `.api.json` doc model into one [`ApiEntry`] per member.
//! - [`read_js_report`] reads an api-extractor `.api.md` report, the API a package accepted, into one [`ApiEntry`]
//!   per declaration and member.
//! - [`js_report_package`] and [`rust_text_crate`] name the package or crate a file was meant to hold the API of, even
//!   when its reader refuses it: a pack fails a broken accepted file of its own run, and skips one of another.

#[cfg(test)]
mod tests;

mod js_model;
mod js_report;
mod rust_text;

pub use js_model::read_js_model;
pub use js_report::{js_report_package, read_js_report};
pub use rust_text::{read_rust_text, rust_text_crate};
pub use specforge_code_facts::ApiEntry;
