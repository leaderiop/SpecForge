//! @rust/deps — runs cargo-cyclonedx and osv-scanner and records a Rust project's dependencies and their known
//! vulnerabilities as `@code/core`'s `dependency` and `vulnerability` facts (ADR 0389). A collector runs one
//! command, so the pack holds two.
//!
//! The collector `rust-sbom` is selected by a `Cargo.toml` at the project root. The host runs, with consent,
//! `cargo-cyclonedx cyclonedx --format json --spec-version 1.5 --all-features --override-filename
//! rust-sbom.cdx.json`. cargo-cyclonedx has no output path: it writes one BOM named `rust-sbom.cdx.json` beside each
//! workspace member's `Cargo.toml`. So the report is the glob `**/rust-sbom.cdx.json`, read fresh after each run
//! (ADR 0389 D6), and a project may add `rust-sbom.cdx.json` to its `.gitignore`. cargo-cyclonedx calls `cargo
//! metadata`, which may run build scripts (ADR 0389 D9). Every BOM the glob hands in is read by
//! `specforge-code-cyclonedx` and minted by `specforge-code-facts` against `@rust/cargo`'s packages: one `dependency`
//! fact per component, citing the served package its BOM's root names. A BOM whose root names no served package is
//! an error naming that report: it is no report about this project.
//!
//! The collector `rust-osv` is selected by a `Cargo.lock`. The host runs, with consent, `osv-scanner scan source
//! --lockfile Cargo.lock --format json --output-file {report}`. The scan sends the lockfile's package names and
//! versions to the OSV API over the network, which the approval shows (ADR 0389 D9). osv-scanner exits 1 when it
//! finds vulnerabilities and 128 when it finds no lockfile (then it writes no report: E045); the host judges neither
//! exit code (ADR 0348 D4), it reads the report. Each record becomes a `vulnerability` fact citing the served cargo
//! packages at or under the lockfile's directory; every package the scan lists that no served package holds is
//! returned as one warning line, which the host shows as W203 (ADR 0389 D7), never dropped.

use specforge_code_facts::{dependency_facts, vulnerability_facts};
use specforge_extension_sdk::prelude::*;

const SBOM: &str = "rust-sbom";
/// Every BOM cargo-cyclonedx writes, one beside each member manifest, relative to the project root.
const SBOM_REPORT: &str = "**/rust-sbom.cdx.json";
const OSV: &str = "rust-osv";
/// The report osv-scanner writes, relative to the project root.
const OSV_REPORT: &str = ".specforge/reports/rust-osv.json";

#[specforge_extension_sdk::extension(
    name = "@rust/deps",
    version = "1.0.0",
    description = "Runs cargo-cyclonedx and osv-scanner and records dependencies and vulnerabilities as facts (@code/core)"
)]
struct RustDeps;

impl Contributions for RustDeps {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // It brings @rust/cargo, whose packages are what a BOM's root names and a lockfile's directory holds.
        c.peer("@rust/source", "^1.0");
        c.collector(SBOM, |k| {
            k.input_format("cyclonedx-json")
                .detect_files(&["Cargo.toml"])
                .run(&[
                    "cargo-cyclonedx",
                    "cyclonedx",
                    "--format",
                    "json",
                    "--spec-version",
                    "1.5",
                    "--all-features",
                    "--override-filename",
                    "rust-sbom.cdx.json",
                ])
                .env(&[
                    "CARGO_*",
                    "RUSTUP_*",
                    "RUSTC",
                    "RUSTC_*",
                    "SOURCE_DATE_EPOCH",
                ])
                .install("cargo install cargo-cyclonedx --locked")
                .report(SBOM_REPORT)
                .input(Input::facts(&["package"]))
                .answers(&["dependency"])
                .collect(collect_sbom);
        });
        c.collector(OSV, |k| {
            k.input_format("osv-json")
                .detect_files(&["Cargo.lock"])
                .run(&[
                    "osv-scanner",
                    "scan",
                    "source",
                    "--lockfile",
                    "Cargo.lock",
                    "--format",
                    "json",
                    "--output-file",
                    "{report}",
                ])
                .env(&[
                    "HTTPS_PROXY",
                    "HTTP_PROXY",
                    "NO_PROXY",
                    "https_proxy",
                    "http_proxy",
                    "no_proxy",
                    "OSV_*",
                ])
                .install("go install github.com/google/osv-scanner/v2/cmd/osv-scanner@latest (or: brew install osv-scanner)")
                .report(OSV_REPORT)
                .input(Input::facts(&["package"]))
                .answers(&["vulnerability"])
                .collect(collect_osv);
        });
    }
}

/// Map every BOM cargo-cyclonedx wrote to `dependency` facts of the served package each root names.
fn collect_sbom(input: &CollectInput) -> Result<CollectOutput, String> {
    if input.reports.is_empty() {
        return Err(format!(
            "{SBOM} reads every {SBOM_REPORT} cargo-cyclonedx wrote; it was given none"
        ));
    }
    let mut reports: Vec<&CollectReportFile> = input.reports.iter().collect();
    reports.sort_by(|a, b| a.path.cmp(&b.path));
    let mut boms = Vec::with_capacity(reports.len());
    for report in reports {
        let bom = specforge_code_cyclonedx::read(&report.content)
            .map_err(|why| format!("{}: {why}", report.path))?;
        // Minted one BOM at a time first, so an error names the report whose root is not served.
        dependency_facts(std::slice::from_ref(&bom), &input.facts)
            .map_err(|why| format!("{}: {why}", report.path))?;
        boms.push(bom);
    }
    Ok(CollectOutput {
        facts: dependency_facts(&boms, &input.facts)?,
        ..CollectOutput::default()
    })
}

/// Map osv-scanner's report to `vulnerability` facts, and every package it lists that no served package holds to a
/// warning line.
fn collect_osv(input: &CollectInput) -> Result<CollectOutput, String> {
    let [report] = &input.reports[..] else {
        return Err(format!(
            "{OSV} reads one report, {OSV_REPORT}; it was given {}",
            input.reports.len()
        ));
    };
    let cwd = input.cwd.as_deref().ok_or(
        "the host sent no cwd: osv-scanner's report names absolute lockfile paths, read against the project root",
    )?;
    let scan = specforge_code_osv::read(&report.content, cwd)
        .map_err(|why| format!("{}: {why}", report.path))?;
    let minted = vulnerability_facts(&scan, &input.facts);
    Ok(CollectOutput {
        facts: minted.facts,
        warnings: minted.skipped,
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
