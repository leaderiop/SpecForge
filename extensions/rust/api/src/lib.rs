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

use specforge_code_api::read_rust_text;
use specforge_code_facts::api_facts;
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "rust-api";
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
        c.peer("@code/core", "^1.2");
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
    // The tool prints its errors (a missing nightly toolchain, a failed build) to stderr, so stdout holds no entry.
    let warnings = entries
        .is_empty()
        .then(|| {
            format!(
                "{path}: no public API item was read; if cargo public-api failed (a missing nightly toolchain, a \
                 failed build), nothing was recorded"
            )
        })
        .into_iter()
        .collect();
    Ok(CollectOutput {
        facts: api_facts(&entries, &input.facts),
        warnings,
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
