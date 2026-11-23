//! @rust/coverage — runs the project's tests under cargo-llvm-cov and records its LCOV coverage as `@code/core`'s
//! `coverage` facts (ADR 0389).
//!
//! The collector `rust-coverage` is selected by a `Cargo.toml` at the project root. The host runs, with consent,
//! `cargo-llvm-cov llvm-cov nextest --workspace --ignore-run-fail --lcov --output-path {report}`: the program is
//! `cargo-llvm-cov`, so a missing tool is "not found on PATH" with its install line, and nextest runs the tests, as
//! `@rust/test` does. `--ignore-run-fail` makes a failing test still write the report. The run builds the workspace a
//! second time, instrumented, under `target/llvm-cov-target`, and runs the project's own tests (ADR 0389 D9). It runs
//! in the caller's environment and declares the cargo, rustup and LLVM variables it reads, so the approval names them.
//!
//! The LCOV is read by `specforge-code-lcov` and minted by `specforge-code-facts` against `@rust/source`'s items: a
//! `coverage` fact per item a line reaches, one per file for what no item holds. A file outside the project (a path
//! dependency beside the workspace) is skipped and counted, and the pack returns one warning naming the report and the
//! paths, which the host shows as W203 (ADR 0389 D7).

use specforge_code_facts::{coverage_facts, outside_line};
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "rust-coverage";
/// The LCOV cargo-llvm-cov writes, relative to the project root.
const REPORT: &str = ".specforge/reports/rust-coverage.lcov";

#[specforge_extension_sdk::extension(
    name = "@rust/coverage",
    version = "1.0.0",
    description = "Runs cargo llvm-cov and records its LCOV coverage as coverage facts (@code/core)"
)]
struct RustCoverage;

impl Contributions for RustCoverage {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its items place each line; without them every line is its file's.
        c.peer("@rust/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("lcov")
                .detect_files(&["Cargo.toml"])
                // No filter: a filtered run is partial coverage.
                .run(&[
                    "cargo-llvm-cov",
                    "llvm-cov",
                    "nextest",
                    "--workspace",
                    "--ignore-run-fail",
                    "--lcov",
                    "--output-path",
                    "{report}",
                ])
                .env(&[
                    "CARGO_*",
                    "RUSTUP_*",
                    "RUSTC",
                    "RUSTC_*",
                    "RUSTFLAGS",
                    "RUSTDOCFLAGS",
                    "LLVM_*",
                ])
                .install("cargo install cargo-llvm-cov cargo-nextest --locked")
                .report(REPORT)
                .input(Input::facts(&["item"]))
                .answers(&["coverage"])
                .collect(collect);
        });
    }
}

/// Map cargo-llvm-cov's LCOV to `coverage` facts, and the files it skipped as outside the project to one warning.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let [report] = &input.reports[..] else {
        return Err(format!(
            "{COLLECTOR} reads one report, {REPORT}; it was given {}",
            input.reports.len()
        ));
    };
    let cwd = input.cwd.as_deref().ok_or(
        "the host sent no cwd: cargo-llvm-cov writes absolute file paths, read against the project root",
    )?;
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
