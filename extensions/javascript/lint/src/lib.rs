//! @javascript/lint — runs ESLint with its SARIF formatter and records its findings as `@code/core`'s
//! `static_finding` facts (ADR 0389).
//!
//! The collector `javascript-lint` is selected by an `eslint.config.*` at the project root: the project's own flat
//! config decides the rules. The host runs, with consent, `npx --no -- eslint --format
//! @microsoft/eslint-formatter-sarif --output-file {report} .`. `npx --no` never downloads anything: ESLint and the
//! formatter must be dev dependencies of the project. ESLint loads the project's config and its plugins, which are
//! code the project chose (ADR 0389 D9). ESLint exits 1 when it finds problems and still writes the report; the host
//! does not judge the exit code (ADR 0348 D4), it reads the report.
//!
//! The SARIF is read by `specforge-code-sarif` and minted by `specforge-code-facts` against `@javascript/source`'s
//! items: one `static_finding` per result, citing the innermost item holding its line, else the package holding its
//! file. A result in a file outside the project is skipped and counted, and the pack returns one warning naming the
//! report and the paths, which the host shows as W203 (ADR 0389 D7); a result whose path names no file (the project
//! root itself) is skipped and named in a warning of its own.

use specforge_code_facts::{finding_facts, no_file_line, outside_line};
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "javascript-lint";
/// The SARIF ESLint writes, relative to the project root.
const REPORT: &str = ".specforge/reports/javascript-lint.sarif";

#[specforge_extension_sdk::extension(
    name = "@javascript/lint",
    version = "1.0.0",
    description = "Runs ESLint with its SARIF formatter and records its findings as static_finding facts (@code/core)"
)]
struct JavascriptLint;

impl Contributions for JavascriptLint {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its items and packages place each finding; without them a finding cites nothing.
        c.peer("@javascript/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("sarif")
                .detect_files(&["eslint.config.*"])
                // No filter: a filtered run lints part of the project.
                .run(&[
                    "npx",
                    "--no",
                    "--",
                    "eslint",
                    "--format",
                    "@microsoft/eslint-formatter-sarif",
                    "--output-file",
                    "{report}",
                    ".",
                ])
                .env(&["NODE_*", "ESLINT_*", "CI"])
                .install("npm install --save-dev eslint @microsoft/eslint-formatter-sarif")
                .report(REPORT)
                .input(Input::facts(&["item", "package"]))
                .answers(&["static_finding"])
                .collect(collect);
        });
    }
}

/// Map ESLint's SARIF to `static_finding` facts, and the paths it skipped, outside the project or naming no file, to a
/// warning each.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let [report] = &input.reports[..] else {
        return Err(format!(
            "{COLLECTOR} reads one report, {REPORT}; it was given {}",
            input.reports.len()
        ));
    };
    let cwd = input.cwd.as_deref().ok_or(
        "the host sent no cwd: ESLint's SARIF names absolute file: URIs, read against the project root",
    )?;
    let sarif = specforge_code_sarif::read(&report.content, cwd)
        .map_err(|why| format!("{}: {why}", report.path))?;
    Ok(CollectOutput {
        facts: finding_facts(&sarif.results, &input.facts),
        warnings: outside_line(&report.path, &sarif.outside)
            .into_iter()
            .chain(no_file_line(&report.path, &sarif.no_file))
            .collect(),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
