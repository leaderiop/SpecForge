//! @javascript/coverage — runs the project's own `npm test` under c8 and records its LCOV coverage as `@code/core`'s
//! `coverage` facts (ADR 0389).
//!
//! The collector `javascript-coverage` is selected by a `package.json` at the project root. The host runs, with
//! consent, `npx --no -- c8 --reporter=lcovonly --reports-dir=.specforge/reports/javascript-coverage npm test`. c8
//! wraps the project's own `test` script, so the project's test command decides what runs, whichever runner it uses;
//! the project's own tests run (ADR 0389 D9). `npx --no` never downloads c8: it must be a dev dependency of the
//! project. c8 writes into a directory, always `lcov.info`, so the argv names the directory and the report is the file
//! in it; c8 also leaves its raw V8 data under `tmp/` there, cleared on its next run. c8 exits with the test command's
//! code and still writes the report, so a failing test still records coverage.
//!
//! The LCOV is read by `specforge-code-lcov` and minted by `specforge-code-facts` against `@javascript/source`'s
//! items: a `coverage` fact per item a line reaches, one per file for what no item holds. A file outside the project
//! is skipped and counted, and the pack returns one warning naming the report and the paths, which the host shows as
//! W203 (ADR 0389 D7).

use specforge_code_facts::{coverage_facts, outside_line};
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "javascript-coverage";
/// The LCOV c8 writes in the directory its argv names, relative to the project root.
const REPORT: &str = ".specforge/reports/javascript-coverage/lcov.info";

#[specforge_extension_sdk::extension(
    name = "@javascript/coverage",
    version = "1.0.0",
    description = "Runs c8 around npm test and records its LCOV coverage as coverage facts (@code/core)"
)]
struct JavascriptCoverage;

impl Contributions for JavascriptCoverage {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its items place each line; without them every line is its file's.
        c.peer("@javascript/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("lcov")
                .detect_files(&["package.json"])
                // No filter: `npm test -- <text>` is the runner's, and a filtered run is partial coverage.
                .run(&[
                    "npx",
                    "--no",
                    "--",
                    "c8",
                    "--reporter=lcovonly",
                    "--reports-dir=.specforge/reports/javascript-coverage",
                    "npm",
                    "test",
                ])
                .env(&["NODE_*", "NPM_CONFIG_*", "npm_config_*", "CI"])
                .install("npm install --save-dev c8")
                .report(REPORT)
                .input(Input::facts(&["item"]))
                .answers(&["coverage"])
                .collect(collect);
        });
    }
}

/// Map c8's LCOV to `coverage` facts, and the files it skipped as outside the project to one warning.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let [report] = &input.reports[..] else {
        return Err(format!(
            "{COLLECTOR} reads one report, {REPORT}; it was given {}",
            input.reports.len()
        ));
    };
    let cwd = input
        .cwd
        .as_deref()
        .ok_or("the host sent no cwd: an LCOV file path is read against the project root")?;
    let read = specforge_code_lcov::read(&report.content, cwd)
        .map_err(|why| format!("{}: {why}", report.path))?;
    Ok(CollectOutput {
        facts: coverage_facts(&read.files, &input.facts),
        warnings: outside_line(&report.path, &read.outside)
            .into_iter()
            .collect(),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
