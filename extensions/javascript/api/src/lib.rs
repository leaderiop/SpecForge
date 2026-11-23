//! @javascript/api — runs api-extractor and records the package's public API as `@code/core`'s `api_item` facts
//! (ADR 0389).
//!
//! The collector `javascript-api` is selected by a `tsconfig.json` at the project root. The host writes the pack's
//! config file at `.specforge/reports/javascript-api.api-extractor.json` and runs, with consent, `npx --no --
//! api-extractor run --local --config {config}`. `npx --no` never downloads anything: api-extractor and TypeScript must
//! be dev dependencies of the project. api-extractor resolves `projectFolder` against the config's own folder, so
//! `../..` is the project root.
//!
//! api-extractor reads built declarations, never `.ts` sources, and its config names the entry as a literal path: the
//! pack's entry is `<projectFolder>/dist/index.d.ts`, so the project builds its declarations there first (the pack
//! runs no build). Only the doc model is written, to `{report}`: `apiReport`, `dtsRollup` and `tsdocMetadata` stay
//! off, so no `.api.md`, rollup or metadata file is written into the project (ADR 0389 D8), and every message reports
//! at `logLevel` `none`, so only a config or compile failure fails the run. `--local` keeps an API report diff from
//! failing it.
//!
//! The doc model is read by `specforge-code-api` and minted by `specforge-code-facts` against `@javascript/source`'s
//! items: one `api_item` per member, `api:js:<package>::<name>[.<member>…]`, citing the one served item of the package
//! with that name path.

use specforge_code_api::read_js_model;
use specforge_code_facts::api_facts;
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "javascript-api";
/// The doc model api-extractor writes, relative to the project root.
const REPORT: &str = ".specforge/reports/javascript-api.api.json";
/// The config file the host writes before the run, `{report}` the report's absolute path (ADR 0348 D2).
const CONFIG: &str = "api-extractor.json";
const TEMPLATE: &str = r#"{
  "projectFolder": "../..",
  "mainEntryPointFilePath": "<projectFolder>/dist/index.d.ts",
  "apiReport": { "enabled": false },
  "docModel": { "enabled": true, "apiJsonFilePath": "{report}" },
  "dtsRollup": { "enabled": false },
  "tsdocMetadata": { "enabled": false },
  "messages": {
    "compilerMessageReporting": { "default": { "logLevel": "none" } },
    "extractorMessageReporting": { "default": { "logLevel": "none" } },
    "tsdocMessageReporting": { "default": { "logLevel": "none" } }
  }
}
"#;

#[specforge_extension_sdk::extension(
    name = "@javascript/api",
    version = "1.0.0",
    description = "Runs api-extractor and records the package's public API as api_item facts (@code/core)"
)]
struct JavascriptApi;

impl Contributions for JavascriptApi {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its items are what an api_item cites; without them an entry cites nothing.
        c.peer("@javascript/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("api-extractor-json")
                .detect_files(&["tsconfig.json"])
                .run(&[
                    "npx",
                    "--no",
                    "--",
                    "api-extractor",
                    "run",
                    "--local",
                    "--config",
                    "{config}",
                ])
                .config(CONFIG, TEMPLATE)
                .env(&["NODE_*", "CI"])
                .install("npm install --save-dev @microsoft/api-extractor typescript")
                .report(REPORT)
                .input(Input::facts(&["item"]))
                .answers(&["api_item"])
                .collect(collect);
        });
    }
}

/// Map api-extractor's doc model to `api_item` facts.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let [report] = &input.reports[..] else {
        return Err(format!(
            "{COLLECTOR} reads one report, {REPORT}; it was given {}",
            input.reports.len()
        ));
    };
    let entries =
        read_js_model(&report.content).map_err(|why| format!("{}: {why}", report.path))?;
    Ok(CollectOutput {
        facts: api_facts(&entries, &input.facts),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
