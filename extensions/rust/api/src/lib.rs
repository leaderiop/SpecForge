//! @rust/api — runs cargo public-api and records the crate's public API as `@code/core`'s `api_item` facts
//! (ADR 0389).
//!
//! The collector `rust-api` is selected by a `Cargo.toml` at the project root. The host runs, with consent,
//! `cargo-public-api --color never -ss`: the program is `cargo-public-api`, so a missing tool is "not found on PATH"
//! with its install line. `-ss` omits blanket and auto-trait impls, which every type has; derived impls stay, as they
//! are real surface. cargo public-api builds the crate's rustdoc JSON, so it runs build scripts and proc macros, which
//! the approval shows as it shows the command, and it needs a nightly toolchain, which it finds by itself when the
//! active one is stable (ADR 0389 D9).
//!
//! cargo public-api reads one package per run and has no `--workspace`: the root package, unless a caller names
//! another (`specforge collect --runner rust-api --filter <crate>`, which the host passes as `--package <crate>`). A
//! virtual workspace has no root package, so each crate is its own filtered run.
//!
//! The tool prints the API to standard output only, which the host captures into
//! `.specforge/reports/rust-api/rust-api.stdout.txt` and passes as `CollectInput::stdout`; `specforge collect --report
//! <file>` passes a saved capture as the one report instead. `specforge-code-api` reads the text and
//! `specforge-code-facts` mints one `api_item` per public entry, citing the one `@rust/source` item of the library
//! crate its path names. A failed run (a missing nightly toolchain, a failed build) prints its error to stderr and no
//! entry to stdout, so a capture with no entry gives one warning saying nothing was recorded, which the host shows as
//! W203, instead of a clean run.
//!
//! The accepted API (ADR 0390 D3) is a committed `public-api.txt` (the read `accepted`, glob `**/public-api.txt`, files
//! up to 4 MiB): cargo public-api's own snapshot file, `tests/public-api.txt` or `tests/snapshots/public-api.txt`. It
//! must be written with the pack's own flags, `cargo public-api --color never -ss > public-api.txt` (a snapshot test's
//! builder omitting blanket and auto-trait impls the same way), or every impl the run omits reads as removed. Only a
//! crate on both sides is judged: each item's `api_item` then carries `change` (`added`, `unchanged`, `changed`,
//! `narrowed`, `removed`) against it; without an accepted file the facts carry none. Accepting a break is committing
//! the updated file. A file that does not parse is the collector's error naming it; a file the host did not serve
//! (over 4 MiB, not UTF-8) or naming no crate the run read is one warning naming it, which the host shows as W203.
//! Limits: cargo public-api prints public items only, so a narrowing (`pub` to `pub(crate)`) reads as removed; the
//! line holds the attributes, so adding any attribute (`#[must_use]` as `#[non_exhaustive]`) reads as changed; the
//! tool's ordering and format are not stable, so a tool upgrade can change every line: regenerate the file with the
//! version that runs.

use std::collections::BTreeSet;

use specforge_code_api::read_rust_text;
use specforge_code_facts::{api_facts, api_facts_against, ApiEntry};
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "rust-api";
/// The read of the committed accepted API files.
const ACCEPTED: &str = "accepted";
/// The report directory, relative to the project root: the host keeps the captured standard output in it.
const REPORT: &str = ".specforge/reports/rust-api";
/// Where the host keeps the captured standard output: `<report dir>/<collector>.stdout.txt`.
const CAPTURE: &str = ".specforge/reports/rust-api/rust-api.stdout.txt";

#[specforge_extension_sdk::extension(
    name = "@rust/api",
    version = "1.0.0",
    description = "Runs cargo public-api and records the crate's public API as api_item facts (@code/core)"
)]
struct RustApi;

impl Contributions for RustApi {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.4");
        // Its items are what an api_item cites; without them an entry cites nothing.
        c.peer("@rust/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("public-api-text")
                .detect_files(&["Cargo.toml"])
                .run(&["cargo-public-api", "--color", "never", "-ss"])
                .filter(&["--package", "{filter}"])
                .env(&[
                    "CARGO_*",
                    "RUSTUP_*",
                    "RUSTC",
                    "RUSTC_*",
                    "RUSTFLAGS",
                    "RUSTDOCFLAGS",
                    "RUST_LOG",
                ])
                .install(
                    "cargo install cargo-public-api --locked (and: rustup toolchain install nightly --profile \
                     minimal)",
                )
                .report(REPORT)
                .capture_stdout()
                .input(Input::facts(&["item", "package"]))
                .input(Input::files(
                    ACCEPTED,
                    FileReadSpec::glob("**/public-api.txt").text_up_to(4_194_304),
                ))
                .answers(&["api_item"])
                .collect(collect);
        });
    }
}

/// Map cargo public-api's captured text (else the one report given) to `api_item` facts.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let (path, text) = match (&input.stdout, &input.reports[..]) {
        (Some(stdout), _) => (CAPTURE, stdout.as_str()),
        (None, [report]) => (report.path.as_str(), report.content.as_str()),
        (None, reports) => {
            return Err(format!(
                "{COLLECTOR}: cargo public-api's standard output was not captured, and it reads one report in its \
                 place; it was given {}",
                reports.len()
            ))
        }
    };
    let entries = read_rust_text(text).map_err(|why| format!("{path}: {why}"))?;
    let mut warnings = Vec::new();
    let accepted = accepted(input, &entries, &mut warnings)?;
    // The tool prints its errors (a missing nightly toolchain, a failed build) to stderr, so stdout holds no entry.
    if entries.is_empty() {
        warnings.push(format!(
            "{path}: no public API item was read; if cargo public-api failed (a missing nightly toolchain, a failed \
             build), nothing was recorded"
        ));
    }
    let facts = if accepted.is_empty() {
        api_facts(&entries, &input.facts)
    } else {
        api_facts_against(&entries, &accepted, &input.facts)
    };
    Ok(CollectOutput {
        facts,
        warnings,
        ..CollectOutput::default()
    })
}

/// The entries of the served accepted files that name a crate of `entries`; a warning for each file not served or
/// naming none, and for a read that matched more files than were served.
fn accepted(
    input: &CollectInput,
    entries: &[ApiEntry],
    warnings: &mut Vec<String>,
) -> Result<Vec<ApiEntry>, String> {
    let Some(read) = input.files.get(ACCEPTED) else {
        return Ok(Vec::new());
    };
    if read.truncated {
        warnings.push(format!(
            "the read '{ACCEPTED}' (**/public-api.txt): more files matched than were read ({FILE_READ_MAX}); no \
             break was judged against a file not read"
        ));
    }
    let crates: BTreeSet<&str> = entries.iter().map(|e| e.container.as_str()).collect();
    let mut accepted = Vec::new();
    for file in &read.facts {
        let Some(text) = file.text.as_deref().filter(|_| file.error.is_none()) else {
            let error = file.error.as_deref().unwrap_or("no text");
            warnings.push(format!(
                "{}: the accepted API was not read ({error}); no break was judged for it",
                file.path
            ));
            continue;
        };
        let mut read = read_rust_text(text).map_err(|why| format!("{}: {why}", file.path))?;
        read.retain(|e| crates.contains(e.container.as_str()));
        if read.is_empty() {
            warnings.push(format!(
                "{}: names no crate this run read; no break was judged for it",
                file.path
            ));
        }
        accepted.extend(read);
    }
    Ok(accepted)
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
