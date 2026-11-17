//! Reports a test's spec-link attribute that does not name a requirement of the project's spec, without running a
//! test (ADR 0374 D4).
//!
//! Every `#[spec(...)]` / `#[specforge_test(...)]` / `#[specforge_test_macros::test(...)]` attribute is read from
//! the Rust sources with `syn` (a crate under a `tests/fixtures/` directory is another project's and is skipped, and
//! so are the macro's own self-tests), and the project is compiled. Each attribute names, with the kind word
//! `requirement`, a requirement of the spec, and carries no `verify` text:
//!
//! - `UNKNOWN <file>:<line> <id>`: no entity has the id;
//! - `KIND <file>:<line> <word> <id>: a test names a requirement`: the kind word is not `requirement`;
//! - `KIND <file>:<line> requirement <id> is a <kind>, not a requirement`: the entity is of another kind;
//! - `VERIFY <file>:<line> <id>: a requirement takes no verify text`.
//!
//!   cargo run -p xtask --bin check-links                     the repository's tests against its spec
//!   cargo run -p xtask --bin check-links -- ROOT [DIR...]    ROOT's project, scanning DIR... under it
//!
//! Prints one line per finding; exits 1 on any.
#![expect(
    clippy::print_stdout,
    reason = "a developer tool: its report is its stdout"
)]

use specforge_common::ProjectRoot;
use specforge_component::ComponentRuntime;
use specforge_project::CompiledProject;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use xtask::links;

const DEFAULT_SCAN: [&str; 3] = ["crates", "extensions", "integrations/rust"];
/// The macro's own self-tests link to entities invented for them.
const SELF_TEST_DIR: &str = "integrations/rust/specforge-test";
/// The kind word, and the entity kind, of a link that names a requirement (`@gsd/core`).
const REQUIREMENT: &str = "requirement";

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().unwrap_or_else(|| ".".to_string()));
    let mut scan: Vec<String> = args.collect();
    if scan.is_empty() {
        scan = DEFAULT_SCAN.iter().map(|s| s.to_string()).collect();
    }
    let findings = match check(&root, &scan) {
        Ok(findings) => findings,
        Err(message) => {
            eprintln!("check-links: {message}");
            std::process::exit(2);
        }
    };
    for finding in &findings {
        println!("{finding}");
    }
    if !findings.is_empty() {
        std::process::exit(1);
    }
}

fn check(root: &Path, scan: &[String]) -> Result<Vec<String>, String> {
    let read = links::read(root, scan, &[SELF_TEST_DIR]);
    for path in &read.unparsed {
        eprintln!("check-links: skipped {}: does not parse", path.display());
    }
    let project_root = ProjectRoot::of(root).map_err(|e| format!("{}: {e:?}", root.display()))?;
    let project = CompiledProject::compile(
        &project_root,
        Some(Arc::new(ComponentRuntime::with_user_cache())),
    );
    let entities = project.entities().coverage_entities();
    let kinds: BTreeMap<&str, &str> = entities
        .iter()
        .map(|e| (e.id.as_str(), e.kind.as_str()))
        .collect();

    let mut findings = Vec::new();
    let mut reported: BTreeSet<(String, usize)> = BTreeSet::new();
    for link in &read.links {
        if !reported.insert((link.file.clone(), link.line)) {
            continue;
        }
        let Some(kind) = kinds.get(link.id.as_str()) else {
            findings.push(format!("UNKNOWN {}:{} {}", link.file, link.line, link.id));
            continue;
        };
        if link.kind != REQUIREMENT {
            findings.push(format!(
                "KIND {}:{} {} {}: a test names a requirement",
                link.file, link.line, link.kind, link.id
            ));
        } else if *kind != REQUIREMENT {
            findings.push(format!(
                "KIND {}:{} requirement {} is a {kind}, not a requirement",
                link.file, link.line, link.id
            ));
        }
        if link.verify.is_some() {
            findings.push(format!(
                "VERIFY {}:{} {}: a requirement takes no verify text",
                link.file, link.line, link.id
            ));
        }
    }
    Ok(findings)
}
