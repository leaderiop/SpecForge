//! @javascript/source — mints `@code/core`'s `item` facts from a JavaScript or TypeScript project's source files, and
//! the `package` facts they point at (ADR 0347).
//!
//! The collector `javascript-source` is selected by a `package.json` at the project root and runs no command. It is
//! served every `package.json` with its text (`specforge-javascript-workspace` names the package that owns a file)
//! and one listing per source extension; it keeps the files a named package owns that are neither test files nor
//! generated output ([`files`]) and has the host parse them with the pack's query (`queries/js-tags.scm`) through the
//! syntax import, which its fs read capability on the `project` root backs: the typescript grammar for `.ts .mts
//! .cts`, tsx for the rest, which reads JavaScript and JSX too. Every declaration at item scope (the program, a
//! namespace's body) and every member of a class or an interface there becomes an item named
//! `js:<package>/<file>::<name>[.<member>]` ([`items`]), minted by `specforge-code-facts`; every named package becomes
//! a `pkg:npm/<name>` package fact, the target of its items' `package` ([`packages`]). A second package of one name is
//! `pkg:npm/<name>#<dir>`, its items `js:<name>@<dir>/…` (`specforge-javascript-workspace`).

mod files;
mod items;
mod packages;

use specforge_code_facts::item_facts;
use specforge_extension_sdk::prelude::*;
use specforge_javascript_workspace::{packages_input, Workspace};

const COLLECTOR: &str = "javascript-source";
/// The pack's query, compiled by the host's reader for either grammar.
const QUERY: &str = include_str!("../queries/js-tags.scm");

#[specforge_extension_sdk::extension(
    name = "@javascript/source",
    version = "1.0.0",
    description = "Reads a JavaScript or TypeScript project's source with the host's parser into item facts (@code/core)"
)]
struct JavascriptSource;

impl Contributions for JavascriptSource {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.1");
        c.capability(
            Capability::fs(COLLECTOR)
                .roots(&["project"])
                .ops(&[FsAccess::Read])
                .call_kinds(&[Operation::Collect])
                .why("parse the project's JavaScript and TypeScript source with the host's reader"),
        );
        c.collector(COLLECTOR, |k| {
            k.detect_files(&["package.json"])
                .input(packages_input())
                .answers(&["item", "package"]);
            for listing in &files::LISTINGS {
                k.input(listing.input());
            }
            k.collect(collect);
        });
    }
}

/// Every item of every kept source file, and every named package, as facts.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let workspace = files::read_packages(Workspace::of(input));
    let parsed = files::parsed(input, &workspace)?;
    let mut definitions = Vec::new();
    for file in &parsed {
        if let Some((package, under)) = workspace.owner(&file.path) {
            definitions.extend(items::read(package, &file.path, under, &file.matches));
        }
    }
    let mut facts = item_facts(definitions);
    facts.extend(packages::facts(&workspace));
    Ok(CollectOutput {
        facts,
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
