//! @javascript/deps — runs cyclonedx-npm and osv-scanner and records a JavaScript project's dependencies and their
//! known vulnerabilities as `@code/core`'s `dependency` and `vulnerability` facts (ADR 0389). A collector runs one
//! command, so the pack holds two.
//!
//! The collector `javascript-sbom` is selected by a `package-lock.json` at the project root. The host runs, with
//! consent, `npx --no -- cyclonedx-npm --output-format JSON --spec-version 1.6 --package-lock-only --output-file
//! {report}`. `npx --no` never downloads anything: cyclonedx-npm must be a dev dependency of the project.
//! `--package-lock-only` reads the lockfile alone, so no `node_modules` is needed and no install script runs.
//! The BOM is read by `specforge-code-cyclonedx` and minted by `specforge-code-facts` against `@javascript/source`'s
//! packages: one `dependency` fact per component, citing the served package the BOM's root names. A BOM whose root
//! names no served package is an error: it is no report about this project.
//!
//! The collector `javascript-osv` is selected by the same lockfile. The host runs, with consent, `osv-scanner scan
//! source --lockfile package-lock.json --format json --output-file {report}`. The scan sends the lockfile's package
//! names and versions to the OSV API over the network, which the approval shows (ADR 0389 D9). osv-scanner exits 1
//! when it finds vulnerabilities and 128 when it finds no lockfile (then it writes no report: E045); the host judges
//! neither exit code (ADR 0348 D4), it reads the report. Each record becomes a `vulnerability` fact citing the
//! served npm package at or under the lockfile's directory; every package the scan lists that no served package
//! holds is returned as one warning line, which the host shows as W203 (ADR 0389 D7), never dropped.

use specforge_code_facts::{dependency_facts, vulnerability_facts};
use specforge_extension_sdk::prelude::*;

const SBOM: &str = "javascript-sbom";
/// The BOM cyclonedx-npm writes, relative to the project root.
const SBOM_REPORT: &str = ".specforge/reports/javascript-sbom.cdx.json";
const OSV: &str = "javascript-osv";
/// The report osv-scanner writes, relative to the project root.
const OSV_REPORT: &str = ".specforge/reports/javascript-osv.json";

#[specforge_extension_sdk::extension(
    name = "@javascript/deps",
    version = "1.0.0",
    description = "Runs cyclonedx-npm and osv-scanner and records dependencies and vulnerabilities as facts (@code/core)"
)]
struct JavascriptDeps;

impl Contributions for JavascriptDeps {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its packages are what a BOM's root names; without them a dependency cites nothing.
        c.peer("@javascript/source", "^1.0");
        c.collector(SBOM, |k| {
            k.input_format("cyclonedx-json")
                .detect_files(&["package-lock.json"])
                .run(&[
                    "npx",
                    "--no",
                    "--",
                    "cyclonedx-npm",
                    "--output-format",
                    "JSON",
                    "--spec-version",
                    "1.6",
                    "--package-lock-only",
                    "--output-file",
                    "{report}",
                ])
                .env(&["NODE_*", "NPM_CONFIG_*", "npm_config_*", "CI"])
                .install("npm install --save-dev @cyclonedx/cyclonedx-npm")
                .report(SBOM_REPORT)
                .input(Input::facts(&["package"]))
                .answers(&["dependency"])
                .collect(collect_sbom);
        });
        c.collector(OSV, |k| {
            k.input_format("osv-json")
                .detect_files(&["package-lock.json"])
                .run(&[
                    "osv-scanner",
                    "scan",
                    "source",
                    "--lockfile",
                    "package-lock.json",
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

/// Map cyclonedx-npm's BOM to `dependency` facts of the served package its root names.
fn collect_sbom(input: &CollectInput) -> Result<CollectOutput, String> {
    let [report] = &input.reports[..] else {
        return Err(format!(
            "{SBOM} reads one report, {SBOM_REPORT}; it was given {}",
            input.reports.len()
        ));
    };
    let bom = specforge_code_cyclonedx::read(&report.content)
        .map_err(|why| format!("{}: {why}", report.path))?;
    Ok(CollectOutput {
        facts: dependency_facts(&[bom], &input.facts)
            .map_err(|why| format!("{}: {why}", report.path))?,
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
