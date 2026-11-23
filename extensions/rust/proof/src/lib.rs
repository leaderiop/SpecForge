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
//! says FAILED, placed at the item `@rust/source` answered for the harness function. Kani prints a harness's path
//! without its crate, and one `Complete` line per crate: the crate of a run is the one whose served items hold every
//! harness of it (`rust:<crate>::<harness path>`), and none or several is the collector's error naming the harnesses
//! (ADR 0389 D10). A harness is a test fact but not an item's `test_id`: `@rust/source` does not see
//! `#[kani::proof]` as a test and is unchanged.
//!
//! An output cut off inside a run is the collector's error (the reader's). An output with no harness at all reads the
//! same whether the crate has none or Kani stopped before verifying (a failed build prints to stderr), so the pack
//! returns one warning saying nothing was checked, which the host shows as W203, instead of a clean run.
//!
//! A caller narrows the run with a filter (`specforge collect --runner rust-proof --filter <TEXT>`): the host appends
//! `--harness <TEXT>`, which Kani matches against harness names. Kani runs on Linux x86_64 and macOS only; it is not
//! supported on Windows (ADR 0389 D12).

use std::collections::{BTreeMap, BTreeSet};

use specforge_code_facts::{test_facts, TestCase, TestId};
use specforge_code_kani::{read, Run, Verdict};
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
    let served = Served::of(&input.facts);
    let mut tests = Vec::new();
    for run in runs.iter().filter(|run| !run.harnesses.is_empty()) {
        let krate = crate_of(run, &served)?;
        for harness in &run.harnesses {
            let Some((_, path, item)) = served
                .holders(&harness.name)
                .into_iter()
                .find(|(c, _, _)| *c == krate)
            else {
                return Err(format!(
                    "{COLLECTOR}: no item of the crate {krate} holds the harness '{}'",
                    harness.name
                ));
            };
            let status = match harness.verdict {
                Verdict::Successful => TestStatus::Passed,
                Verdict::Failed => TestStatus::Failed,
            };
            let test = TestCase {
                seconds: harness.seconds,
                ran: true,
                ..TestCase::unrun(TestId::rust(&krate, path), status)
            };
            tests.push(test.placed(item));
        }
    }
    // Kani's stdout looks the same when the crate has no harness and when Kani never reached one (a failed build).
    let warnings = tests
        .is_empty()
        .then(|| {
            format!(
                "{path}: no proof harness was read; if cargo-kani failed to start or to build the crate, nothing \
                 was checked"
            )
        })
        .into_iter()
        .collect();
    Ok(CollectOutput {
        facts: test_facts(tests),
        warnings,
        ..CollectOutput::default()
    })
}

/// The served `item` facts, looked up by id and by the path after their crate.
struct Served<'f> {
    by_id: BTreeMap<&'f str, &'f Fact>,
    /// `rust:<crate>::<path>` as `<path>` to each `(<crate>, item)`; a crate segment never holds `::`.
    by_path: BTreeMap<&'f str, Vec<(&'f str, &'f Fact)>>,
}

impl<'f> Served<'f> {
    fn of(facts: &'f [Fact]) -> Served<'f> {
        let mut served = Served {
            by_id: BTreeMap::new(),
            by_path: BTreeMap::new(),
        };
        for fact in facts.iter().filter(|fact| fact.kind == "item") {
            served.by_id.insert(fact.id.as_str(), fact);
            if let Some((krate, path)) = fact
                .id
                .strip_prefix("rust:")
                .and_then(|id| id.split_once("::"))
            {
                if !krate.is_empty() {
                    served.by_path.entry(path).or_default().push((krate, fact));
                }
            }
        }
        served
    }

    /// Each `(crate, harness path, item)` the harness `name` can be: the item `rust:<crate>::<name>`, or the item
    /// `rust:<name>` when the name starts with its crate (then the path is the rest).
    fn holders<'n>(&self, name: &'n str) -> Vec<(&'f str, &'n str, &'f Fact)> {
        let mut holders: Vec<(&'f str, &'n str, &'f Fact)> = self
            .by_path
            .get(name)
            .into_iter()
            .flatten()
            .map(|(krate, item)| (*krate, name, *item))
            .collect();
        if let Some((_, path)) = name.split_once("::") {
            if let Some(item) = self.by_id.get(format!("rust:{name}").as_str()) {
                if let Some((krate, _)) = item
                    .id
                    .strip_prefix("rust:")
                    .and_then(|id| id.split_once("::"))
                {
                    holders.push((krate, path, *item));
                }
            }
        }
        holders
    }
}

/// The one crate whose served items hold every harness of `run`: Kani prints a harness's path without its crate and
/// one `Complete` line per crate, so a run is one crate's (ADR 0389 D10). None or several is the collector's error,
/// never a guess.
fn crate_of(run: &Run, served: &Served) -> Result<String, String> {
    let mut common: Option<BTreeSet<&str>> = None;
    for harness in &run.harnesses {
        let crates: BTreeSet<&str> = served
            .holders(&harness.name)
            .into_iter()
            .map(|(krate, _, _)| krate)
            .collect();
        common = Some(match common {
            None => crates,
            Some(kept) => kept.intersection(&crates).copied().collect(),
        });
    }
    let common: Vec<&str> = common.unwrap_or_default().into_iter().collect();
    let names = run
        .harnesses
        .iter()
        .map(|h| format!("'{}'", h.name))
        .collect::<Vec<_>>()
        .join(", ");
    match common[..] {
        [krate] => Ok(krate.to_string()),
        [] => Err(format!(
            "{COLLECTOR}: no crate's served items hold every harness of one Kani run ({names}), as an item \
             rust:<crate>::<harness>; collect @rust/source first"
        )),
        _ => Err(format!(
            "{COLLECTOR}: the crates {} each hold every harness of one Kani run ({names}): the run's crate is \
             ambiguous",
            common.join(" and ")
        )),
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
