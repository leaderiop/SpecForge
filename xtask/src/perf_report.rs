//! `cargo run -p xtask --bin perf-report -- [--specforge <path>] [--runs <n>]`: measures `specforge check` and
//! `specforge collect --no-run` on a release build of SpecForge itself against the budget (ADR 0353 D8), and writes
//! `<target>/perf/report.json`. A measurement, never a check: no gate step or test runs it, and the nightly
//! `perf.yml` workflow records it. It copies the tracked tree (`git ls-files`) so no local fact store, cache or ledger
//! is measured, and runs with a fresh `HOME`.
//!
//! The steps, each timed in wall-clock milliseconds (the median of `runs`, a cold collect once) beside the counted
//! work it did: `check` before any collect; a cold collect (no `.specforge/`), each collector's files parsed, read
//! from the syntax cache and facts stored; a warm collect, which must parse no file; one edit (a line comment
//! appended to one Rust file) and a collect, which must parse exactly that file; and `check` with the facts served,
//! counted from the stores the harness ledger records as they are (what `check` serves).
//! Exit 0 within budget, 1 when a step is over budget or a counted expectation fails, 2 when a step cannot run.

#![expect(
    clippy::print_stdout,
    reason = "a developer tool: its report is its stdout and report.json"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use serde_json::{Value, json};

/// The file the edit step appends a line comment to: a crate root every source collect reads.
const EDITED: &str = "crates/kernel/common/src/lib.rs";

/// The steps, in the order they run: each one's key in `report.json`, its row in the table, and its budget in
/// milliseconds (none: measured, held to none).
const STEPS: [(&str, &str, Option<u64>); 5] = [
    ("check_empty", "check, no facts", None),
    // A collect with no syntax cache: every source file parsed.
    ("collect_cold", "collect cold", Some(10_000)),
    // A collect over an unchanged tree: every file read from the syntax cache.
    ("collect_warm", "collect warm", Some(2_000)),
    ("collect_edit", "collect, one edit", None),
    // `specforge check` with SpecForge's facts served.
    ("check", "check, facts served", Some(2_000)),
];
const COLD: usize = 1;
const WARM: usize = 2;
const EDIT: usize = 3;

/// What one step measured: its milliseconds and the work it counted.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Step {
    ms: u64,
    work: Work,
}

/// A step's counted work: the files its collect parsed and read from the syntax cache, over every collector, and
/// the facts the cold collect stored or the check was served.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Work {
    parsed: u64,
    cached: u64,
    facts: u64,
}

/// What a run measured: each of [`STEPS`], in its order.
type Measured = [Step; STEPS.len()];

/// Each step over its budget, and each counted expectation that failed, in words: empty when all hold.
fn verdicts(measured: &Measured) -> Vec<String> {
    let mut over: Vec<String> = STEPS
        .iter()
        .zip(measured)
        .filter_map(|(&(_, row, budget), step)| {
            let budget = budget?;
            (step.ms > budget)
                .then(|| format!("{row} took {} ms, over its {budget} ms budget", step.ms))
        })
        .collect();
    if measured[COLD].work.parsed == 0 {
        over.push("the cold collect parsed no file".to_string());
    }
    if measured[WARM].work.parsed != 0 {
        over.push(format!(
            "the warm collect parsed {} files, not 0: the syntax cache did not answer",
            measured[WARM].work.parsed
        ));
    }
    if measured[EDIT].work.parsed != 1 {
        over.push(format!(
            "the collect after one edit parsed {} files, not 1",
            measured[EDIT].work.parsed
        ));
    }
    over
}

/// The median of `samples` (the lower middle of an even count); 0 for none.
fn median(samples: &[u64]) -> u64 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    sorted
        .get(sorted.len().saturating_sub(1) / 2)
        .copied()
        .unwrap_or(0)
}

/// A step that could not run, said once.
struct Failed(String);

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(Failed(why)) => {
            eprintln!("perf-report: {why}");
            ExitCode::from(2)
        }
    }
}

/// The options: `--specforge <path>` (else a release build) and `--runs <n>` (else 5).
fn options() -> Result<(Option<PathBuf>, usize), Failed> {
    let mut args = std::env::args().skip(1);
    let (mut specforge, mut runs) = (None, 5);
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| Failed(format!("{arg} needs a value")))
        };
        match arg.as_str() {
            "--specforge" => specforge = Some(PathBuf::from(value()?)),
            "--runs" => {
                runs = value()?
                    .parse()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| Failed("--runs takes a positive number".into()))?
            }
            other => return Err(Failed(format!("unknown argument {other}"))),
        }
    }
    Ok((specforge, runs))
}

fn run() -> Result<bool, Failed> {
    let (specforge, runs) = options()?;
    let repo = specforge_repo_root::repo_root();
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("target"));
    let specforge = match specforge {
        Some(path) => path,
        None => {
            let mut build = Command::new("cargo");
            build.current_dir(repo).args([
                "build",
                "--release",
                "--locked",
                "-p",
                "specforge-cli",
                "--bin",
                "specforge",
            ]);
            succeeded(&mut build, "cargo build --release")?;
            target.join("release/specforge")
        }
    };
    let perf = target.join("perf");
    let (tree, home) = (perf.join("tree"), perf.join("home"));
    for dir in [&tree, &home] {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).map_err(|e| Failed(format!("{}: {e}", dir.display())))?;
    }
    copy_tracked(repo, &tree)?;
    let sf = Specforge {
        bin: &specforge,
        tree: &tree,
        home: &home,
    };

    let check_empty = Step {
        ms: median(&sf.times(runs, &["check"])?),
        work: Work::default(),
    };
    let (ms, answer) = sf.collect()?;
    let stored = answer["runners"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["facts"].as_u64())
        .sum();
    let cold = Step {
        ms,
        work: Work {
            facts: stored,
            ..work(&answer)
        },
    };
    // Every warm run must parse nothing: their files parsed add up, the cached count is the last run's.
    let (mut warm_ms, mut warm) = (Vec::new(), Work::default());
    for _ in 0..runs {
        let (ms, answer) = sf.collect()?;
        warm_ms.push(ms);
        let this = work(&answer);
        warm = Work {
            parsed: warm.parsed + this.parsed,
            ..this
        };
    }
    let warm = Step {
        ms: median(&warm_ms),
        work: warm,
    };
    append_comment(&tree.join(EDITED))?;
    let (ms, answer) = sf.collect()?;
    let edit = Step {
        ms,
        work: work(&answer),
    };
    let check = Step {
        ms: median(&sf.times(runs, &["check"])?),
        work: Work {
            facts: served_facts(&specforge_registry::PlainDisk, &tree),
            ..Work::default()
        },
    };
    let measured: Measured = [check_empty, cold, warm, edit, check];

    let verdicts = verdicts(&measured);
    let report = report(repo, runs, &measured, &verdicts);
    let path = perf.join("report.json");
    let text = serde_json::to_string_pretty(&report).expect("a report is JSON");
    std::fs::write(&path, text).map_err(|e| Failed(format!("{}: {e}", path.display())))?;
    print!("{}", table(&measured));
    for verdict in &verdicts {
        println!("over: {verdict}");
    }
    println!("wrote {}", path.display());
    Ok(verdicts.is_empty())
}

/// The built `specforge`, run in the copied tree with a fresh `HOME`.
struct Specforge<'a> {
    bin: &'a Path,
    tree: &'a Path,
    home: &'a Path,
}

impl Specforge<'_> {
    /// `args` with `--format json`: its milliseconds and its JSON answer.
    fn json(&self, args: &[&str]) -> Result<(u64, Value), Failed> {
        let mut command = Command::new(self.bin);
        command
            .current_dir(self.tree)
            .env("HOME", self.home)
            .args(args)
            .args(["--format", "json"])
            .stdin(std::process::Stdio::null());
        let started = Instant::now();
        let output = command
            .output()
            .map_err(|e| Failed(format!("{}: {e}", self.bin.display())))?;
        let ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let answer = serde_json::from_slice(&output.stdout).map_err(|e| {
            Failed(format!(
                "`specforge {}` printed no JSON ({e}): {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            ))
        })?;
        Ok((ms, answer))
    }

    /// `args`, `runs` times: the milliseconds of each.
    fn times(&self, runs: usize, args: &[&str]) -> Result<Vec<u64>, Failed> {
        (0..runs)
            .map(|_| self.json(args).map(|(ms, _)| ms))
            .collect()
    }

    /// `collect --no-run`: the report-less source packs read the tree, nothing runs.
    fn collect(&self) -> Result<(u64, Value), Failed> {
        let (ms, answer) = self.json(&["collect", "--no-run"])?;
        if answer["status"] != "collected" {
            return Err(Failed(format!(
                "`specforge collect --no-run` answered {answer}"
            )));
        }
        Ok((ms, answer))
    }
}

/// The syntax work of a collect's answer, over every collector.
fn work(answer: &Value) -> Work {
    let runners = answer["runners"].as_array().into_iter().flatten();
    runners.fold(Work::default(), |sum, runner| Work {
        parsed: sum.parsed + runner["syntax"]["parsed"].as_u64().unwrap_or(0),
        cached: sum.cached + runner["syntax"]["cached"].as_u64().unwrap_or(0),
        facts: 0,
    })
}

/// The facts `check` is served in the project at `root`, read through `disk`: those of each store the harness ledger
/// records exactly as it is (a store the ledger does not record, W191, serves none).
fn served_facts(disk: &dyn specforge_registry::Disk, root: &Path) -> u64 {
    specforge_project::facts::recorded_stores(disk, root)
        .iter()
        .map(|store| store.facts.len() as u64)
        .sum()
}

/// Copy every file git tracks in `repo`, as the working tree holds it, to `to`.
fn copy_tracked(repo: &Path, to: &Path) -> Result<(), Failed> {
    let mut listed = Command::new("git");
    listed.current_dir(repo).args(["ls-files", "-z"]);
    let out = listed
        .output()
        .map_err(|e| Failed(format!("git ls-files: {e}")))?;
    if !out.status.success() {
        return Err(Failed("git ls-files failed".into()));
    }
    for path in String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
    {
        let (from, into) = (repo.join(path), to.join(path));
        // A tracked file the working tree deleted is not copied.
        if !from.is_file() {
            continue;
        }
        if let Some(dir) = into.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Failed(format!("{}: {e}", dir.display())))?;
        }
        std::fs::copy(&from, &into).map_err(|e| Failed(format!("{path}: {e}")))?;
    }
    Ok(())
}

/// Append a line comment to `file`: an edit that changes its digest and nothing it defines.
fn append_comment(file: &Path) -> Result<(), Failed> {
    let mut text =
        std::fs::read_to_string(file).map_err(|e| Failed(format!("{}: {e}", file.display())))?;
    text.push_str("// perf-report: one edit\n");
    std::fs::write(file, text).map_err(|e| Failed(format!("{}: {e}", file.display())))
}

/// Run `command`, failing with `what` when it does not succeed.
fn succeeded(command: &mut Command, what: &str) -> Result<(), Failed> {
    let status = command
        .status()
        .map_err(|e| Failed(format!("{what}: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Failed(format!("{what} failed ({status})")))
    }
}

/// `report.json`: where it ran, each step's milliseconds and counts, the budget and the verdicts.
fn report(repo: &Path, runs: usize, measured: &Measured, verdicts: &[String]) -> Value {
    let sha = Command::new("git")
        .current_dir(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default();
    let cpus = std::thread::available_parallelism().map_or(0, usize::from);
    let steps: serde_json::Map<String, Value> = STEPS
        .iter()
        .zip(measured)
        .map(|(&(key, _, budget), step)| {
            let w = step.work;
            let row = json!({"ms": step.ms, "budget_ms": budget, "parsed": w.parsed, "cached": w.cached,
                             "facts": w.facts});
            (key.to_string(), row)
        })
        .collect();
    json!({
        "sha": sha,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "cpus": cpus,
        "runs": runs,
        "edited": EDITED,
        "steps": steps,
        "verdicts": verdicts,
    })
}

/// The steps as a table, one per line.
fn table(measured: &Measured) -> String {
    let mut out = String::from("step                  ms  budget  work\n");
    for (&(_, row, budget), step) in STEPS.iter().zip(measured) {
        let budget = budget.map_or_else(|| "-".to_string(), |b| b.to_string());
        let Work {
            parsed,
            cached,
            facts,
        } = step.work;
        let mut work = Vec::new();
        if parsed + cached > 0 {
            work.push(format!("{parsed} parsed, {cached} cached"));
        }
        if facts > 0 {
            work.push(format!("{facts} facts"));
        }
        out.push_str(&format!(
            "{row:<20}{:>6}  {budget:>6}  {}\n",
            step.ms,
            work.join(", ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECK: usize = STEPS.len() - 1;

    /// A step of `ms` that parsed and read `parsed` and `cached` files.
    fn step(ms: u64, parsed: u64, cached: u64, facts: u64) -> Step {
        Step {
            ms,
            work: Work {
                parsed,
                cached,
                facts,
            },
        }
    }

    /// A run within budget, every counted expectation holding.
    fn within() -> Measured {
        [
            step(400, 0, 0, 0),
            step(6_000, 2_461, 0, 36_640),
            step(900, 0, 2_461, 0),
            step(950, 1, 2_460, 0),
            step(1_200, 0, 0, 36_640),
        ]
    }

    #[test]
    fn perf_report_verdicts_name_each_step_over_budget() {
        assert_eq!(verdicts(&within()), Vec::<String>::new());
        let mut over = within();
        over[COLD].ms = 10_001;
        over[WARM].ms = 2_001;
        over[CHECK].ms = 2_500;
        assert_eq!(
            verdicts(&over),
            [
                "collect cold took 10001 ms, over its 10000 ms budget",
                "collect warm took 2001 ms, over its 2000 ms budget",
                "check, facts served took 2500 ms, over its 2000 ms budget",
            ]
        );
    }

    #[test]
    fn perf_report_flags_a_warm_collect_that_parses() {
        let mut measured = within();
        measured[WARM] = step(900, 3, 2_458, 0);
        measured[EDIT] = step(950, 2, 2_459, 0);
        assert_eq!(
            verdicts(&measured),
            [
                "the warm collect parsed 3 files, not 0: the syntax cache did not answer",
                "the collect after one edit parsed 2 files, not 1",
            ]
        );
    }

    #[test]
    fn perf_report_tables_each_step_once_with_its_budget_and_work() {
        assert_eq!(
            table(&within()),
            "step                  ms  budget  work\n\
             check, no facts        400       -  \n\
             collect cold          6000   10000  2461 parsed, 0 cached, 36640 facts\n\
             collect warm           900    2000  0 parsed, 2461 cached\n\
             collect, one edit      950       -  1 parsed, 2460 cached\n\
             check, facts served   1200    2000  36640 facts\n"
        );
    }

    #[test]
    fn perf_report_takes_the_median_of_its_runs() {
        assert_eq!(median(&[30, 10, 20]), 20);
        assert_eq!(median(&[40, 10, 30, 20]), 20);
        assert_eq!(median(&[]), 0);
    }

    /// The ledger and the store of a collect that recorded `facts`, the store's bytes as `on_disk` says.
    fn recorded(
        facts: usize,
        on_disk: impl FnOnce(Vec<u8>) -> Vec<u8>,
    ) -> specforge_registry::MemoryDisk {
        use specforge_project::facts::Store;
        use specforge_project::harness::{LEDGER_FILE, Ledger, Line};
        let items = (0..facts)
            .map(|n| specforge_protocol_types::Fact::new("item", format!("rust:p::f{n}")))
            .collect();
        let store = Store::new(
            "rust-source",
            "@rust/source",
            "worktree",
            items,
            Default::default(),
        );
        let bytes = store.bytes();
        let mut ledger = Ledger::default();
        ledger.insert(Line::facts("rust-source", "worktree", &bytes));
        specforge_registry::MemoryDisk::new()
            .with_file(
                Path::new("/p").join(LEDGER_FILE),
                ledger.canonical().as_bytes(),
            )
            .with_file(
                Path::new("/p").join(Store::path("rust-source", "worktree")),
                &on_disk(bytes),
            )
    }

    #[test]
    fn perf_report_counts_the_facts_check_is_served_not_those_collected() {
        let served = recorded(3, |bytes| bytes);
        assert_eq!(served_facts(&served, Path::new("/p")), 3);
        // A store changed since the ledger recorded it is W191 to `check`: none of its facts is served.
        let changed = recorded(3, |mut bytes| {
            bytes.extend_from_slice(b"\n");
            bytes
        });
        assert_eq!(served_facts(&changed, Path::new("/p")), 0);
    }
}
