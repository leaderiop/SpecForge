//! `specforge-code-facts`: the ids of `@code/core`'s facts, which every pack links (ADR 0347 D1).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so a runner pack (`@javascript/node-test`, `@rust/test`), a source pack (`@rust/source`,
//! `@javascript/source`), a manifest pack (`@rust/cargo`) and `@code/core` itself name a fact here, and so agree.
//!
//! - [`TestId`]: `js:<package>/<file>#<s1 > … > name>`, `rust:<nextest binary id>#<test path>`.
//! - [`ItemId`]: `rust:<crate segment>::<mod>::…::<name>`, `js:<package>/<file>::<name>[.<member>]`; never `#`.
//! - [`PackageId`]: `pkg:cargo/<name>`, `pkg:cargo/<name>#<dir>`, `pkg:npm/<name>`.
//! - [`crate_segment`]: the crate part of a Rust item id, the package name qualified by its [`TargetKind`] as
//!   rust-analyzer qualifies it; an item's test id writes the words its [`Edition`] reserves raw, as libtest does.
//! - [`test_facts`] mints a `test` and, when it ran, a `test_run` per test id from a runner's [`TestCase`]s.
//! - [`item_facts`] mints one `item` per item id from a source pack's [`Definition`]s, every place and kind listed.
//! - [`Package::fact`] mints a `package`; [`location`] turns the reader's points into a fact's place.
//!
//! Every id parses back to itself (`FromStr`), and a SCIP symbol of rust-analyzer or scip-typescript maps into exactly
//! one (ADR 0347 D4).

mod ids;
mod mint;
#[cfg(test)]
mod tests;

pub use ids::{
    crate_segment, Edition, IdError, ItemId, Owner, PackageId, Scheme, TargetKind, TestId,
};
pub use mint::{
    item_facts, location, test_facts, Definition, ItemKind, Names, Package, Target, TestCase,
    Visibility,
};
