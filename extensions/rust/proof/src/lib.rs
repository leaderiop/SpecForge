//! @rust/proof — runs Kani and records each proof harness as `@code/core`'s `test` and `test_run` facts (ADR 0389).
//!
//! The collector `rust-proof` is selected by a `Cargo.toml` at the project root. The host runs, with consent,
//! `cargo-kani --workspace --output-format regular`: the program is `cargo-kani`, so a missing Kani is "not found on
//! PATH" with its install line. Kani builds the crates and model-checks them, which the approval shows as it shows the
//! command. It prints its results to standard output only, which the host captures into
//! `.specforge/reports/rust-proof/rust-proof.stdout.txt` and passes as `CollectInput::stdout`; `specforge collect
//! --report <file>` passes a saved capture as the one report instead. `specforge-code-kani` reads the text.
//!
//! Each harness becomes the test `rust:<crate>#<harness path>`, passed when Kani says SUCCESSFUL and failed when it
//! says FAILED, placed at the item `@rust/source` answered for the harness function.

use std::collections::BTreeMap;

use specforge_code_facts::{test_facts, TestCase, TestId};
use specforge_code_kani::{read, Verdict};
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "rust-proof";
/// The report directory, relative to the project root: the host keeps the captured standard output in it.
const REPORT: &str = ".specforge/reports/rust-proof";
/// Where the host keeps the captured standard output: `<report dir>/<collector>.stdout.txt`.
const CAPTURE: &str = ".specforge/reports/rust-proof/rust-proof.stdout.txt";

#[specforge_extension_sdk::extension(
    name = "@rust/proof",
    version = "1.0.0",
    description = "Runs Kani and records each proof harness as test facts (@code/core)"
)]
struct RustProof;

impl Contributions for RustProof {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its items name each harness's crate and place it; without them no harness can be named.
        c.peer("@rust/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("kani-regular")
                .detect_files(&["Cargo.toml"])
                .run(&["cargo-kani", "--workspace", "--output-format", "regular"])
                .filter(&["--harness", "{filter}"])
                .env(&[
                    "CARGO_*",
                    "RUSTUP_*",
                    "RUSTC",
                    "RUSTC_*",
                    "RUSTFLAGS",
                    "KANI_*",
                ])
                .install("cargo install --locked kani-verifier && cargo kani setup")
                .report(REPORT)
                .capture_stdout()
                .input(Input::facts(&["item", "package"]))
                .answers(&["test", "test_run"])
                .collect(collect);
        });
    }
}

/// Map Kani's captured text (else the one report given) to a `test` and a `test_run` fact per harness.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let (path, text) = match (&input.stdout, &input.reports[..]) {
        (Some(stdout), _) => (CAPTURE, stdout.as_str()),
        (None, [report]) => (report.path.as_str(), report.content.as_str()),
        (None, reports) => {
            return Err(format!(
                "{COLLECTOR}: Kani's standard output was not captured, and it reads one report in its place; it \
                 was given {}",
                reports.len()
            ))
        }
    };
    let runs = read(text).map_err(|why| format!("{path}: {why}"))?;
    let items: BTreeMap<&str, &Fact> = input
        .facts
        .iter()
        .filter(|fact| fact.kind == "item")
        .map(|fact| (fact.id.as_str(), fact))
        .collect();
    let mut tests = Vec::new();
    for harness in runs.iter().flat_map(|run| &run.harnesses) {
        let suffix = format!("::{}", harness.name);
        let held: Vec<(&str, &Fact)> = items
            .iter()
            .filter_map(|(id, fact)| {
                let krate = id.strip_prefix("rust:")?.strip_suffix(suffix.as_str())?;
                (!krate.is_empty() && !krate.contains("::")).then_some((krate, *fact))
            })
            .collect();
        let [(krate, item)] = held[..] else {
            return Err(format!(
                "{COLLECTOR}: the harness '{}' is held by {} crates' served items; collect @rust/source first",
                harness.name,
                held.len()
            ));
        };
        let status = match harness.verdict {
            Verdict::Successful => TestStatus::Passed,
            Verdict::Failed => TestStatus::Failed,
        };
        let test = TestCase {
            seconds: harness.seconds,
            ran: true,
            ..TestCase::unrun(TestId::rust(krate, &harness.name), status)
        };
        tests.push(test.placed(item));
    }
    Ok(CollectOutput {
        facts: test_facts(tests),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
