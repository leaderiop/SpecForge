//! @rust/source — mints `@code/core`'s `item` facts from each cargo target's module tree (ADR 0348 D6).
//!
//! The collector `rust-source` is selected by a `Cargo.toml` at the project root and runs no command. It is served
//! the `package` facts `@rust/cargo` answers; per target of each package it walks the target's **module tree** from
//! its root by rustc's rules ([`modules`]) and has the host parse each round's new files with the pack's query
//! (`queries/rust-tags.scm`) through the syntax import, which its fs read capability on the `project` root backs. Every
//! definition at module level, in inline modules and in `impl` and `trait` blocks becomes an `item` ([`items`]),
//! minted by `specforge-code-facts`; a file two targets reach mints items in each crate.

mod items;
mod modules;

use specforge_code_facts::item_facts;
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "rust-source";
/// The pack's query, compiled by the host's reader.
const QUERY: &str = include_str!("../queries/rust-tags.scm");

#[specforge_extension_sdk::extension(
    name = "@rust/source",
    version = "1.0.0",
    description = "Reads each cargo target's module tree with the host's parser into item facts (@code/core)"
)]
struct RustSource;

impl Contributions for RustSource {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.1");
        c.peer("@rust/cargo", "^1.0");
        c.capability(
            Capability::fs(COLLECTOR)
                .roots(&["project"])
                .ops(&[FsAccess::Read])
                .call_kinds(&[Operation::Collect])
                .why("parse the project's Rust source with the host's reader"),
        );
        c.collector(COLLECTOR, |k| {
            k.detect_files(&["Cargo.toml"])
                .input(Input::facts(&["package"]))
                .answers(&["item"])
                .collect(collect);
        });
    }
}

/// Every item of every target's module tree, as an `item` fact.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let crates = modules::crates(&input.facts)?;
    let definitions = modules::walk(&crates)?;
    Ok(CollectOutput {
        facts: item_facts(definitions),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
