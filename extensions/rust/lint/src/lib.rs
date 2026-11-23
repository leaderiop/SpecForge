//! @rust/lint — runs cargo clippy and records its diagnostics as `@code/core`'s `static_finding` facts (ADR 0389).
//!
//! The collector `rust-lint` is selected by a `Cargo.toml` at the project root. The host runs, with consent,
//! `cargo-clippy clippy --workspace --all-targets --message-format=json`: the program is `cargo-clippy`, so a missing
//! component is "not found on PATH" with its install line. clippy builds the workspace and runs its build scripts and
//! proc macros, which the approval shows as it shows the command (ADR 0389 D9). cargo writes its JSON messages to
//! standard output, which the host captures into `.specforge/reports/rust-lint/rust-lint.stdout.txt` and passes as
//! `CollectInput::stdout`; `specforge collect --report <file>` passes a saved capture as the one report instead.
//!
//! clippy writes no SARIF, and the host runs one argv, so no `clippy-sarif` pipe: [`clippy`] converts the JSON
//! messages into the SARIF reader's results, and `specforge-code-facts` mints one `static_finding` per diagnostic
//! against `@rust/source`'s items (ADR 0389 D8). Only a diagnostic's primary span is kept. `--all-targets` checks a
//! library twice, as lib and as lib test, and repeats each of its diagnostics: the converter keeps one. A span in a
//! file outside the project (a registry crate's macro) is skipped and counted, and the pack returns one warning naming
//! the capture and the paths, which the host shows as W203 (ADR 0389 D7). A span naming no file (rustc's pseudo names
//! such as `<std macros>`) is skipped and counted the same way, in a warning of its own, never minted.

use specforge_code_facts::{finding_facts, no_file_line, outside_line};
use specforge_extension_sdk::prelude::*;

mod clippy;

const COLLECTOR: &str = "rust-lint";
/// The report directory, relative to the project root: the host keeps the captured standard output in it.
const REPORT: &str = ".specforge/reports/rust-lint";
/// Where the host keeps the captured standard output: `<report dir>/<collector>.stdout.txt`.
const CAPTURE: &str = ".specforge/reports/rust-lint/rust-lint.stdout.txt";

#[specforge_extension_sdk::extension(
    name = "@rust/lint",
    version = "1.0.0",
    description = "Runs cargo clippy and records its diagnostics as static_finding facts (@code/core)"
)]
struct RustLint;

impl Contributions for RustLint {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its items and packages place each finding; without them a finding cites nothing.
        c.peer("@rust/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("cargo-json")
                .detect_files(&["Cargo.toml"])
                // No filter: a filtered run lints part of the workspace.
                .run(&[
                    "cargo-clippy",
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--message-format=json",
                ])
                .env(&[
                    "CARGO_*",
                    "RUSTUP_*",
                    "RUSTC",
                    "RUSTC_*",
                    "RUSTFLAGS",
                    "RUSTDOCFLAGS",
                    "CLIPPY_*",
                ])
                .install("rustup component add clippy")
                .report(REPORT)
                .capture_stdout()
                .input(Input::facts(&["item", "package"]))
                .answers(&["static_finding"])
                .collect(collect);
        });
    }
}

/// Map clippy's captured JSON messages (else the one report given) to `static_finding` facts, and the paths it skipped,
/// outside the project or naming no file, to a warning each.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let (path, text) = match (&input.stdout, &input.reports[..]) {
        (Some(stdout), _) => (CAPTURE, stdout.as_str()),
        (None, [report]) => (report.path.as_str(), report.content.as_str()),
        (None, reports) => {
            return Err(format!(
                "{COLLECTOR}: clippy's standard output was not captured, and it reads one report in its place; it \
                 was given {}",
                reports.len()
            ))
        }
    };
    let cwd = input
        .cwd
        .as_deref()
        .ok_or("the host sent no cwd: a clippy span's file is read against the project root")?;
    let converted = clippy::results(text, cwd).map_err(|why| format!("{path}: {why}"))?;
    Ok(CollectOutput {
        facts: finding_facts(&converted.results, &input.facts),
        warnings: outside_line(path, &converted.outside)
            .into_iter()
            .chain(no_file_line(path, &converted.no_file))
            .collect(),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
