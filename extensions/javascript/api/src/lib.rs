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
//!
//! The accepted API (ADR 0390 D3) is the API report the project commits for review, read beside the run (files up to
//! 4 MiB): api-extractor's `etc/<package>.api.md` (the read `accepted`, glob `**/etc/*.api.md`) or rush's
//! `common/reviews/api/<package>.api.md` (the read `reviews`). Never `**/*.api.md`: api-extractor's `temp/` holds the
//! fresh run's own copy, which would hide every break; the `.alpha`, `.beta` and `.public` report variants are not read
//! either. The report's title names its package: only a report of the package the run read is judged, and each
//! `api_item` then carries `change` (`added`, `unchanged`, `changed`, `narrowed`, `removed`) against it; without one the
//! facts carry none. A report of the package that does not parse (a merge-conflict marker, a broken fence) is the
//! collector's error naming it, as are two reports of the package (the accepted side is one file); a file the host did
//! not serve (over 4 MiB, not UTF-8), with no report title (prose in an `etc/` folder) or of another package, broken or
//! not, is one warning naming it, which the host shows as W203, and the collect goes on.
//!
//! Accepting a break is committing the updated report: enable `apiReport` in your own `api-extractor.json` (the same
//! entry, `dist/index.d.ts`), run `api-extractor run --local` (it copies `temp/<package>.api.md` to `etc/`), commit
//! `etc/<package>.api.md`, then `specforge collect`. Your config and the pack's are two configs: they must agree on the
//! entry, `includeForgottenExports` and `bundledPackages`, or the report differs from the run in content, which the
//! comparison does not forgive (it forgives only the report's spelling). The pack's own run never writes a report.

use std::collections::BTreeSet;

use specforge_code_api::{js_report_package, read_js_model, read_js_report};
use specforge_code_facts::{api_facts, api_facts_against, ApiEntry};
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "javascript-api";
/// The reads of the committed accepted reports, and their globs: api-extractor's report folder, then rush's.
const ACCEPTED: [(&str, &str); 2] = [
    ("accepted", "**/etc/*.api.md"),
    ("reviews", "common/reviews/api/*.api.md"),
];
/// The report variants (`reportVariants`), never the accepted side.
const VARIANTS: [&str; 3] = [".alpha.api.md", ".beta.api.md", ".public.api.md"];
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
        c.peer("@code/core", "^1.4");
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
                .input(Input::facts(&["item"]));
            for (name, glob) in ACCEPTED {
                k.input(Input::files(
                    name,
                    FileReadSpec::glob(glob).text_up_to(4_194_304),
                ));
            }
            k.answers(&["api_item"]).collect(collect);
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
    let mut warnings = Vec::new();
    let facts = match accepted(input, &entries, &mut warnings)? {
        Some(accepted) => api_facts_against(&entries, &accepted, &input.facts),
        None => api_facts(&entries, &input.facts),
    };
    Ok(CollectOutput {
        facts,
        warnings,
        ..CollectOutput::default()
    })
}

/// The entries of the one served accepted report of the run's package (the doc model's); a warning for each file not
/// served, not a report or of another package, and for a read that matched more files than were served. A report of
/// the package that does not parse, and two reports of it, are the collector's error.
fn accepted(
    input: &CollectInput,
    entries: &[ApiEntry],
    warnings: &mut Vec<String>,
) -> Result<Option<Vec<ApiEntry>>, String> {
    let packages: BTreeSet<&str> = entries.iter().map(|e| e.container.as_str()).collect();
    let mut found: Option<(&str, Vec<ApiEntry>)> = None;
    for (name, glob) in ACCEPTED {
        let Some(read) = input.files.get(name) else {
            continue;
        };
        if read.truncated {
            warnings.push(format!(
                "the read '{name}' ({glob}): more files matched than were read ({FILE_READ_MAX}); no break was \
                 judged against a file not read"
            ));
        }
        for file in &read.facts {
            if VARIANTS.iter().any(|variant| file.path.ends_with(variant)) {
                continue;
            }
            let Some(text) = file.text.as_deref().filter(|_| file.error.is_none()) else {
                let error = file.error.as_deref().unwrap_or("no text");
                warnings.push(format!(
                    "{}: the accepted API was not read ({error}); no break was judged for it",
                    file.path
                ));
                continue;
            };
            let mut report = match read_js_report(text) {
                Ok(report) => report,
                Err(why) if js_report_package(text).is_some_and(|name| packages.contains(name)) => {
                    return Err(format!("{}: {why}", file.path));
                }
                Err(why) => {
                    warnings.push(format!(
                        "{}: not an API report of a package this run read ({why}); no break was judged \
                         for it",
                        file.path
                    ));
                    continue;
                }
            };
            report.retain(|e| packages.contains(e.container.as_str()));
            if report.is_empty() {
                warnings.push(format!(
                    "{}: names no package this run read; no break was judged for it",
                    file.path
                ));
            } else if let Some((first, _)) = &found {
                return Err(format!(
                    "{first} and {}: two accepted API reports of the package; keep the one api-extractor writes",
                    file.path
                ));
            } else {
                found = Some((&file.path, report));
            }
        }
    }
    Ok(found.map(|(_, report)| report))
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
