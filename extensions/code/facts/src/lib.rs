//! `specforge-code-facts`: the ids of `@code/core`'s facts, which every pack links (ADR 0347 D1).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so a runner pack (`@javascript/node-test`, `@rust/test`), a source pack (`@rust/source`,
//! `@javascript/source`), a manifest pack (`@rust/cargo`) and `@code/core` itself name a fact here, and so agree.
//!
//! - [`TestId`]: `js:<package>/<file>#<s1 > … > name>`, `rust:<nextest binary id>#<test path>`.
//! - [`ItemId`]: `rust:<crate segment>::<mod>::…::<name>`, `js:<package>/<file>::<name>[.<member>]`; never `#`.
//! - [`PackageId`]: `pkg:cargo/<name>`, `pkg:npm/<name>`, `#<dir>` appended for a second package of one name.
//! - [`crate_segment`]: the crate part of a Rust item id, the package name qualified by its [`TargetKind`] as
//!   rust-analyzer qualifies it; an item's test id writes the words its [`Edition`] reserves raw, as libtest does.
//!   [`js_segment`]: the package part of a JavaScript id, a second package of one name qualified by its directory.
//! - [`test_facts`] mints a `test` and, when it ran, a `test_run` per test id from a runner's [`TestCase`]s.
//! - [`item_facts`] mints one `item` per item id from a source pack's [`Definition`]s, every place and kind listed;
//!   [`signature_spelling`] is the one spelling a pack gives a signature in.
//! - [`Package::fact`] mints a `package`; [`location`] turns the reader's points into a fact's place; [`joined`]
//!   resolves a path a manifest or a source file writes against the project-relative directory it is written in;
//!   [`relative`] makes a report's file path project-relative, whatever spelling the host's root has, refusing one
//!   outside the project, and [`file_uri_path`] decodes a report's `file:` URI to a path for it to judge.
//! - [`parse_all`] asks the host's source reader until it has answered every file a source pack keeps, the files an
//!   answer's cap left unparsed asked again.
//! - [`Places`] finds where a report's location lies among the served facts: the innermost `item` holding a file's
//!   line, the `package` holding a file or a directory (ADR 0388).
//! - [`ReportKind`]s name the five kinds a standard report mints and their schemes ([`COVERAGE`], [`API_ITEM`],
//!   [`STATIC_FINDING`], [`DEPENDENCY`], [`VULNERABILITY`]), [`UNKNOWN`] what a report leaves unstated; [`id_part`]
//!   makes report text safe in an id and [`Repeats`] suffixes an id answered twice `~2`, `~3`, ….
//! - [`coverage_facts`] mints a `coverage` per item a coverage report's [`CoverageFile`]s reach, and one per file for
//!   what no item holds.
//! - [`api_facts`] mints an `api_item` per public entry an API snapshot's reader read ([`ApiEntry`], [`ApiKind`]),
//!   citing the one served item its path names.
//! - [`finding_facts`] mints a `static_finding` per result a SARIF reader read ([`SarifResult`], [`Level`]), citing
//!   the innermost served item holding its location, else the package holding its file.
//! - [`dependency_facts`] mints a `dependency` per component of the CycloneDX [`Bom`]s a reader read ([`Component`],
//!   [`Scope`]), citing the one served package each BOM describes.
//! - [`vulnerability_facts`] mints a `vulnerability` per OSV record and project package of an osv-scanner
//!   [`OsvReport`] a reader read ([`OsvSource`], [`OsvPackage`], [`OsvRecord`], [`OsvScore`]), citing the packages the
//!   scanned lockfile belongs to, with a [`Severity`]: the record's label, else the [`rating`] of its highest CVSS v3
//!   [`base_score`].
//!
//! Every id parses back to itself (`FromStr`), and a SCIP symbol of rust-analyzer or scip-typescript maps into exactly
//! one (ADR 0347 D4).

mod api;
mod coverage;
mod cvss;
mod dependency;
mod finding;
mod ids;
mod mint;
mod paths;
mod places;
mod report;
mod syntax;
#[cfg(test)]
mod tests;
mod vulnerability;

pub use api::{api_facts, ApiEntry, ApiKind};
pub use coverage::{coverage_facts, Branch, CoverageFile, Function};
pub use cvss::{base_score, rating};
pub use dependency::{dependency_facts, Bom, Component, Scope};
pub use finding::{finding_facts, Level, SarifResult};
pub use ids::{
    crate_segment, js_segment, Edition, IdError, ItemId, Owner, PackageId, Scheme, TargetKind,
    TestId,
};
pub use mint::{
    item_facts, location, signature_spelling, test_facts, Definition, ItemKind, Names, Package,
    Signature, Target, TestCase, Visibility,
};
pub use paths::{file_uri_path, joined, relative};
pub use places::Places;
pub use report::{
    id_part, Repeats, ReportKind, API_ITEM, COVERAGE, DEPENDENCY, STATIC_FINDING, UNKNOWN,
    VULNERABILITY,
};
pub use syntax::parse_all;
pub use vulnerability::{
    vulnerability_facts, OsvPackage, OsvRecord, OsvReport, OsvScore, OsvSource, Severity,
};
