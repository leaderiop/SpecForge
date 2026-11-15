//! Reports a test's `verify` text its spec entity does not declare (A016)
//! without running a test.
//!
//! Every `#[spec(...)]` / `#[specforge_test_macros::test(...)]` attribute is
//! read from the Rust sources with `syn`; the project is compiled; and the
//! pairs go through the coverage crate's own rule (`Verdict::of`), the one
//! `specforge analyze coverage` applies to a recorded report, so the two
//! cannot disagree.
//!
//!   cargo run -p xtask --bin check-links                     the repository's tests against its spec
//!   cargo run -p xtask --bin check-links -- ROOT [DIR...]    ROOT's project, scanning DIR... under it
//!
//! Prints `A016 <file>:<line> <entity> "<verify>"` per finding (and the
//! closest declared text), `UNKNOWN <file>:<line> <entity>` for a link to an
//! entity the project does not have; exits 1 on any.
#![expect(
    clippy::print_stdout,
    reason = "a developer tool: its report is its stdout"
)]

use specforge_common::ProjectRoot;
use specforge_component::ComponentRuntime;
use specforge_coverage::assess;
use specforge_project::CompiledProject;
use specforge_protocol_types::{TestResult, TestResults, TestStatus};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use xtask::links;

const DEFAULT_SCAN: [&str; 3] = ["crates", "extensions", "integrations/rust"];
/// The macro's own self-tests link to entities invented for them.
const SELF_TEST_DIR: &str = "integrations/rust/specforge-test";

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
    let links = read.links;
    let project_root = ProjectRoot::of(root).map_err(|e| format!("{}: {e:?}", root.display()))?;
    let project = CompiledProject::compile(
        &project_root,
        Some(Arc::new(ComponentRuntime::with_user_cache())),
    );
    let entities = project.entities().coverage_entities();
    let known: BTreeMap<&str, &specforge_coverage::Entity> =
        entities.iter().map(|e| (e.id.as_str(), e)).collect();

    let mut results = TestResults::default();
    for link in links.iter().filter(|l| known.contains_key(l.id.as_str())) {
        results
            .results
            .entry(link.id.clone())
            .or_default()
            .tests
            .push(TestResult {
                name: format!("{}:{}", link.file, link.line),
                status: TestStatus::Passed,
                verify: link.verify.clone(),
                duration_ms: None,
            });
    }
    let assessment = assess(&entities, Some(&results), None, None);

    let mut findings = Vec::new();
    let mut reported: BTreeSet<(String, usize)> = BTreeSet::new();
    for link in &links {
        if !reported.insert((link.file.clone(), link.line)) {
            continue;
        }
        let Some(entity) = known.get(link.id.as_str()) else {
            findings.push(format!("UNKNOWN {}:{} {}", link.file, link.line, link.id));
            continue;
        };
        let Some(text) = &link.verify else { continue };
        let undeclared = assessment
            .verdicts
            .get(&link.id)
            .is_some_and(|v| v.undeclared.contains(text));
        if undeclared {
            let mut line = format!("A016 {}:{} {} {text:?}", link.file, link.line, link.id);
            if let Some(near) = closest(text, &entity.verify_texts) {
                line.push_str(&format!("\n     declared: {near:?}"));
            }
            findings.push(line);
        }
    }
    Ok(findings)
}

/// The declared text nearest `text`, when it is near enough to be a typo.
fn closest<'a>(text: &str, declared: &'a [String]) -> Option<&'a String> {
    declared
        .iter()
        .map(|d| (strsim::normalized_levenshtein(text, d), d))
        .filter(|(score, _)| *score >= 0.5)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, d)| d)
}
