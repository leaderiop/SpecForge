//! `cargo run -p xtask --bin perf-report -- [--specforge <path>] [--runs <n>]`: measures `specforge check` and
//! `specforge collect --no-run` on a release build of SpecForge itself against the budget (ADR 0353 D8), and writes
//! `<target>/perf/report.json`. A measurement, never a check: no gate step or test runs it, and the nightly
//! `perf.yml` workflow records it. It copies the tracked tree (`git ls-files`) so no local fact store, cache or ledger
//! is measured, and runs with a fresh `HOME`.
//!
//! The steps, each timed in wall-clock milliseconds (the median of `runs`, a cold collect once) beside the counted
//! work it did: `check` before any collect; a cold collect (no `.specforge/`), each collector's files parsed, read
//! from the syntax cache and facts stored; a warm collect, which must parse no file; one edit (a line comment
//! appended to one Rust file) and a collect, which must parse exactly that file; and `check` with the facts served.
//! Exit 0 within budget, 1 when a step is over budget or a counted expectation fails, 2 when a step cannot run.

#![expect(
    clippy::print_stdout,
    reason = "a developer tool: its report is its stdout and report.json"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use serde_json::{Value, json};

/// The budget of a collect with no syntax cache: every source file parsed.
const COLLECT_COLD_MS: u64 = 10_000;
/// The budget of a collect over an unchanged tree: every file read from the syntax cache.
const COLLECT_WARM_MS: u64 = 2_000;
/// The budget of `specforge check` with SpecForge's facts served.
const CHECK_MS: u64 = 2_000;
/// The file the edit step appends a line comment to: a crate root every source collect reads.
const EDITED: &str = "crates/kernel/common/src/lib.rs";

/// What a run of the steps measured: milliseconds, and the counted work.
#[derive(Debug, Clone, Default, PartialEq)]
struct Measured {
    check_empty_ms: u64,
    collect_cold_ms: u64,
    collect_warm_ms: u64,
    collect_edit_ms: u64,
    check_ms: u64,
    /// Files parsed and read from the cache by the cold, warm and edit collects, over every collector.
    cold: Work,
    warm: Work,
    edit: Work,
    /// The facts the cold collect stored.
    facts: u64,
}

/// One collect's syntax work over every collector.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Work {
    parsed: u64,
    cached: u64,
}

/// The ceilings the steps are held to.
#[derive(Debug, Clone, Copy)]
struct Budget {
    collect_cold_ms: u64,
    collect_warm_ms: u64,
    check_ms: u64,
}

const BUDGET: Budget = Budget {
    collect_cold_ms: COLLECT_COLD_MS,
    collect_warm_ms: COLLECT_WARM_MS,
    check_ms: CHECK_MS,
};

/// Each step over its budget, and each counted expectation that failed, in words: empty when all hold.
fn verdicts(measured: &Measured, budget: &Budget) -> Vec<String> {
    let mut over = Vec::new();
    let mut ceiling = |step: &str, ms: u64, budget: u64| {
        if ms > budget {
            over.push(format!("{step} took {ms} ms, over its {budget} ms budget"));
        }
    };
    ceiling(
        "collect cold",
        measured.collect_cold_ms,
        budget.collect_cold_ms,
    );
    ceiling(
        "collect warm",
        measured.collect_warm_ms,
        budget.collect_warm_ms,
    );
    ceiling("check", measured.check_ms, budget.check_ms);
    if measured.cold.parsed == 0 {
        over.push("the cold collect parsed no file".to_string());
    }
    if measured.warm.parsed != 0 {
        over.push(format!(
            "the warm collect parsed {} files, not 0: the syntax cache did not answer",
            measured.warm.parsed
        ));
    }
    if measured.edit.parsed != 1 {
        over.push(format!(
            "the collect after one edit parsed {} files, not 1",
            measured.edit.parsed
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

    let check_empty_ms = median(&sf.times(runs, &["check"])?);
    let (collect_cold_ms, cold) = sf.collect()?;
    let facts = cold["runners"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["facts"].as_u64())
        .sum();
    // Every warm run must parse nothing: their files parsed add up, the cached count is the last run's.
    let (mut warm_ms, mut warm) = (Vec::new(), Work::default());
    for _ in 0..runs {
        let (ms, answer) = sf.collect()?;
        warm_ms.push(ms);
        let this = work(&answer);
        warm = Work {
            parsed: warm.parsed + this.parsed,
            cached: this.cached,
        };
    }
    append_comment(&tree.join(EDITED))?;
    let (collect_edit_ms, edit) = sf.collect()?;
    let measured = Measured {
        check_empty_ms,
        collect_cold_ms,
        collect_warm_ms: median(&warm_ms),
        collect_edit_ms,
        check_ms: median(&sf.times(runs, &["check"])?),
        cold: work(&cold),
        warm,
        edit: work(&edit),
        facts,
    };

    let verdicts = verdicts(&measured, &BUDGET);
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
    })
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
fn report(repo: &Path, runs: usize, m: &Measured, verdicts: &[String]) -> Value {
    let sha = Command::new("git")
        .current_dir(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default();
    let cpus = std::thread::available_parallelism().map_or(0, usize::from);
    json!({
        "sha": sha,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "cpus": cpus,
        "runs": runs,
        "steps": {
            "check_empty": {"ms": m.check_empty_ms},
            "collect_cold": {"ms": m.collect_cold_ms, "parsed": m.cold.parsed, "cached": m.cold.cached,
                             "facts": m.facts},
            "collect_warm": {"ms": m.collect_warm_ms, "parsed": m.warm.parsed, "cached": m.warm.cached},
            "collect_edit": {"ms": m.collect_edit_ms, "parsed": m.edit.parsed, "cached": m.edit.cached,
                             "edited": EDITED},
            "check": {"ms": m.check_ms, "facts": m.facts},
        },
        "budget": {
            "collect_cold_ms": BUDGET.collect_cold_ms,
            "collect_warm_ms": BUDGET.collect_warm_ms,
            "check_ms": BUDGET.check_ms,
        },
        "verdicts": verdicts,
    })
}

/// The steps as a table, one per line.
fn table(m: &Measured) -> String {
    let rows = [
        ("check, no facts", m.check_empty_ms, None, String::new()),
        (
            "collect cold",
            m.collect_cold_ms,
            Some(COLLECT_COLD_MS),
            format!(
                "{} parsed, {} cached, {} facts",
                m.cold.parsed, m.cold.cached, m.facts
            ),
        ),
        (
            "collect warm",
            m.collect_warm_ms,
            Some(COLLECT_WARM_MS),
            format!("{} parsed, {} cached", m.warm.parsed, m.warm.cached),
        ),
        (
            "collect, one edit",
            m.collect_edit_ms,
            None,
            format!("{} parsed, {} cached", m.edit.parsed, m.edit.cached),
        ),
        (
            "check, facts served",
            m.check_ms,
            Some(CHECK_MS),
            format!("{} facts", m.facts),
        ),
    ];
    let mut out = String::from("step                  ms  budget  work\n");
    for (step, ms, budget, work) in rows {
        let budget = budget.map_or_else(|| "-".to_string(), |b| b.to_string());
        out.push_str(&format!("{step:<20}{ms:>6}  {budget:>6}  {work}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run within budget, every counted expectation holding.
    fn within() -> Measured {
        Measured {
            check_empty_ms: 400,
            collect_cold_ms: 6_000,
            collect_warm_ms: 900,
            collect_edit_ms: 950,
            check_ms: 1_200,
            cold: Work {
                parsed: 2_461,
                cached: 0,
            },
            warm: Work {
                parsed: 0,
                cached: 2_461,
            },
            edit: Work {
                parsed: 1,
                cached: 2_460,
            },
            facts: 36_640,
        }
    }

    #[test]
    fn perf_report_verdicts_name_each_step_over_budget() {
        assert_eq!(verdicts(&within(), &BUDGET), Vec::<String>::new());
        let over = Measured {
            collect_cold_ms: 10_001,
            collect_warm_ms: 2_001,
            check_ms: 2_500,
            ..within()
        };
        assert_eq!(
            verdicts(&over, &BUDGET),
            [
                "collect cold took 10001 ms, over its 10000 ms budget",
                "collect warm took 2001 ms, over its 2000 ms budget",
                "check took 2500 ms, over its 2000 ms budget",
            ]
        );
    }

    #[test]
    fn perf_report_flags_a_warm_collect_that_parses() {
        let measured = Measured {
            warm: Work {
                parsed: 3,
                cached: 2_458,
            },
            edit: Work {
                parsed: 2,
                cached: 2_459,
            },
            ..within()
        };
        assert_eq!(
            verdicts(&measured, &BUDGET),
            [
                "the warm collect parsed 3 files, not 0: the syntax cache did not answer",
                "the collect after one edit parsed 2 files, not 1",
            ]
        );
    }

    #[test]
    fn perf_report_takes_the_median_of_its_runs() {
        assert_eq!(median(&[30, 10, 20]), 20);
        assert_eq!(median(&[40, 10, 30, 20]), 20);
        assert_eq!(median(&[]), 0);
    }
}
