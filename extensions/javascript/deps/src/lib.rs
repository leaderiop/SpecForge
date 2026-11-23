//! @javascript/deps — runs cyclonedx-npm and records a JavaScript project's dependencies as `@code/core`'s
//! `dependency` facts (ADR 0389).
//!
//! The collector `javascript-sbom` is selected by a `package-lock.json` at the project root. The host runs, with
//! consent, `npx --no -- cyclonedx-npm --output-format JSON --spec-version 1.6 --package-lock-only --output-file
//! {report}`. `npx --no` never downloads anything: cyclonedx-npm must be a dev dependency of the project.
//! `--package-lock-only` reads the lockfile alone, so no `node_modules` is needed and no install script runs.
//!
//! The BOM is read by `specforge-code-cyclonedx` and minted by `specforge-code-facts` against `@javascript/source`'s
//! packages: one `dependency` fact per component, citing the served package the BOM's root names. A BOM whose root
//! names no served package is an error: it is no report about this project.

use specforge_code_facts::dependency_facts;
use specforge_extension_sdk::prelude::*;

const SBOM: &str = "javascript-sbom";
/// The BOM cyclonedx-npm writes, relative to the project root.
const SBOM_REPORT: &str = ".specforge/reports/javascript-sbom.cdx.json";

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
    }
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
