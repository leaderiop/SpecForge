//! The `dogfood` check (ADR 0374 D5): the repository's own tests prove its requirements.
//!
//! nextest writes a JUnit report under its `gate` and `ci` profiles (`<repository root>/target/nextest/<profile>/`):
//! the root run's (CI downloads each test partition's into its own directory there), every guest's under
//! `junit-guests/<guest>/` and the `@gsd` workspace's under `junit-gsd/` (ADR 0393 D2, D3). The check refuses to
//! judge when any of the three sets is missing; every `junit*.xml` found is merged into the one report `@rust/test`
//! reads ([`crate::junit::merge`]). In a copy of the tracked tree ([`crate::checked_tree`]: no fact store, and no
//! untracked copy of the sources, of this machine is in it), the root project is collected by `rust-cargo`,
//! `rust-source` and `rust-test` over that report; then each requirement's computed `proof` is read from `specforge
//! export --format graph --kinds requirement` and judged by [`crate::judge::requirement_verdict`], from the export
//! alone.
//!
//! It also prints X2 (ADR 0388) to its log, [`crate::x2`] over `specforge export --format graph --kinds item,test`
//! of each project it collects. X2 is print only: a failed collect or export of an X2 project is the check's error.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::checks::Selection;
use crate::run::Ctx;
use crate::{checked_tree, judge, junit, steps, x2};

pub fn run(ctx: &Ctx, _: &Selection) -> Result<(), String> {
    let dir = report_dir(&ctx.repo.root, ctx.mode.nextest_profile());
    let found = reports(&dir);
    report_sets(&dir, &found)?;
    let mut texts = Vec::new();
    for path in &found {
        ctx.log(&format!("report {}", path.display()));
        texts.push(std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?);
    }
    let merged = ctx.repo.target_dir.join("gate/rust-test.xml");
    std::fs::write(&merged, junit::merge(&texts)?)
        .map_err(|e| format!("{}: {e}", merged.display()))?;

    let specforge = ctx.specforge()?;
    let tree = ctx.repo.target_dir.join("gate/dogfood");
    let tracked = steps::tracked_paths(ctx)?;
    checked_tree::copy_tracked(&ctx.repo.root, &tracked, &tree)?;
    let tree = tree.to_string_lossy().into_owned();
    let merged = merged.to_string_lossy().into_owned();
    for runner in [
        &["rust-cargo"][..],
        &["rust-source"],
        &["rust-test", "--report", &merged],
    ] {
        let mut args = vec!["collect", "--path", &tree, "--runner"];
        args.extend(runner);
        args.extend(ONLINE);
        json(ctx, specforge, &args)
            .map_err(|e| format!("collecting the root with {}: {e}", runner[0]))?;
    }
    let export = json(
        ctx,
        specforge,
        &[
            "export",
            "--path",
            &tree,
            "--format",
            "graph",
            "--kinds",
            "requirement",
        ],
    )?;
    let (lines, failed) = judge::requirement_verdict(&export);
    for line in &lines {
        ctx.log(line);
    }
    // X2 is print only (ADR 0388 D2): a failed collect or export is the error, never the ratio.
    // SpecForge's missing list is every guest test (thousands): it goes to the log only.
    let x2 = log_x2(ctx, specforge, "SpecForge", &tree).and_then(|()| {
        FIXTURES
            .iter()
            .try_for_each(|fixture| fixture_x2(ctx, specforge, fixture, &tracked))
    });
    let (logged, result) = outcome(failed, &lines, x2);
    if let Some(line) = logged {
        ctx.log(&line);
    }
    result
}

/// The check's result and what to log first: the requirement verdict's first problem when it `failed` (its first line
/// is the counts, the problems follow), with X2's error logged, which the red run would otherwise drop unseen; else
/// X2's own result.
fn outcome(
    failed: bool,
    lines: &[String],
    x2: Result<(), String>,
) -> (Option<String>, Result<(), String>) {
    if failed {
        let problem = lines.get(1).or(lines.first()).cloned().unwrap_or_default();
        (x2.err(), Err(problem))
    } else {
        (None, x2)
    }
}

/// What every collect of the check passes besides its runner.
const ONLINE: [&str; 5] = ["--yes", "--effects", "online", "--format", "json"];

/// A parity fixture X2 is printed for (ADR 0388 D3), collected in a scratch copy of its tracked files under
/// `<target>/gate/x2/<name>` with a `specforge.json` loading `extensions`.
struct Fixture {
    name: &'static str,
    /// The fixture's directory, from the repository root.
    dir: &'static str,
    extensions: &'static [&'static str],
    /// Each collect in order: its runner and the recorded report it reads (`--no-run --report`, from the repository
    /// root), if any.
    collects: &'static [(&'static str, Option<&'static str>)],
}

/// Printed after SpecForge, in this order. Taskboard's JavaScript items carry no `test_id`, so it reads `0/0`.
const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "taskboard",
        dir: "gsd/parity/fixtures/taskboard",
        extensions: &[
            "@gsd/core",
            "@specforge/checks",
            "@code/core",
            "@javascript/node-test",
            "@javascript/source",
        ],
        collects: &[
            ("javascript-source", None),
            ("node-test", Some("gsd/parity/goldens/taskboard/junit.xml")),
        ],
    },
    Fixture {
        name: "ledger",
        dir: "gsd/parity/fixtures/ledger",
        extensions: &[
            "@gsd/core",
            "@specforge/checks",
            "@code/core",
            "@rust/cargo",
            "@rust/source",
            "@rust/test",
        ],
        collects: &[
            ("rust-cargo", None),
            ("rust-source", None),
            ("rust-test", Some("gsd/parity/goldens/ledger/rust-test.xml")),
        ],
    },
];

/// Collect `fixture` in its scratch copy and log its X2 lines; an error names the fixture and the step that failed.
fn fixture_x2(
    ctx: &Ctx,
    specforge: &Path,
    fixture: &Fixture,
    tracked: &[String],
) -> Result<(), String> {
    let name = fixture.name;
    let prefix = format!("{}/", fixture.dir);
    let files: Vec<String> = tracked
        .iter()
        .filter_map(|path| path.strip_prefix(&prefix).map(String::from))
        .collect();
    let copy = ctx.repo.target_dir.join("gate/x2").join(name);
    checked_tree::copy_tracked(&ctx.repo.root.join(fixture.dir), &files, &copy)
        .map_err(|e| format!("X2 {name}: copying the fixture: {e}"))?;
    let project = serde_json::json!({
        "name": name,
        "version": "0.1.0",
        "spec_root": "spec",
        "extensions": fixture.extensions,
    });
    let manifest = copy.join("specforge.json");
    std::fs::write(&manifest, format!("{project:#}\n"))
        .map_err(|e| format!("X2 {name}: {}: {e}", manifest.display()))?;
    let copy = copy.to_string_lossy().into_owned();
    for &(runner, report) in fixture.collects {
        let report = report.map(|r| ctx.repo.root.join(r).to_string_lossy().into_owned());
        let mut args = vec!["collect", "--path", copy.as_str(), "--runner", runner];
        if let Some(report) = &report {
            args.extend(["--no-run", "--report", report.as_str()]);
        }
        args.extend(ONLINE);
        json(ctx, specforge, &args)
            .map_err(|e| format!("X2 {name}: collecting with {runner}: {e}"))?;
    }
    log_x2(ctx, specforge, name, &copy)
}

/// Log the X2 lines of the collected project at `path`, read from `specforge export --format graph --kinds
/// item,test`; an error names the project.
fn log_x2(ctx: &Ctx, specforge: &Path, name: &str, path: &str) -> Result<(), String> {
    let export = json(
        ctx,
        specforge,
        &[
            "export",
            "--path",
            path,
            "--format",
            "graph",
            "--kinds",
            "item,test",
        ],
    )
    .map_err(|e| format!("X2 {name}: exporting its items and tests: {e}"))?;
    let (items, tests) = x2::read(&export).map_err(|e| format!("X2 {name}: {e}"))?;
    for line in x2::lines(name, &x2::ratio(&items, &tests)) {
        ctx.log(&line);
    }
    Ok(())
}

/// `<root>/target/nextest/<profile>`: where nextest writes the JUnit report of a run under `profile` (its store is
/// the workspace root's `target/`, whatever the target directory).
pub(crate) fn report_dir(root: &Path, profile: &str) -> PathBuf {
    root.join("target/nextest").join(profile)
}

/// The guests' reports, one directory per guest under the report dir; the name of their CI artifact (ADR 0393 D2).
pub(crate) const GUESTS: &str = "junit-guests";

/// The `@gsd` workspace's report directory under the report dir; the name of its CI artifact (ADR 0393 D2).
pub(crate) const GSD: &str = "junit-gsd";

/// Empty a report directory before the run that writes it, so a report from an earlier run is never read as current:
/// Ok when it is not there.
pub(crate) fn clear_reports(dir: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("clearing {}: {e}", dir.display()))
        }
        _ => Ok(()),
    }
}

/// Every file named `junit*.xml` under `dir`, at any depth, sorted by path; none when `dir` is not there.
pub(crate) fn reports(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("junit") && name.ends_with(".xml"))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Ok when `found` (the reports under `dir`) holds a report of each set: the root tests' (any not under the guests'
/// or the gsd directory), the guests' and the `@gsd` workspace's. A lost set would read as "all tests" (ADR 0393 D3).
fn report_sets(dir: &Path, found: &[PathBuf]) -> Result<(), String> {
    let (guests, gsd) = (dir.join(GUESTS), dir.join(GSD));
    let root = found
        .iter()
        .any(|path| !path.starts_with(&guests) && !path.starts_with(&gsd));
    for (set, present) in [
        ("root tests", root),
        ("guests", found.iter().any(|path| path.starts_with(&guests))),
        ("gsd", found.iter().any(|path| path.starts_with(&gsd))),
    ] {
        if !present {
            return Err(format!(
                "no {set} JUnit report under {}: run cargo gate full --only tests,guests,gsd first",
                dir.display()
            ));
        }
    }
    Ok(())
}

/// `specforge <args>`'s stdout as JSON; an error when it exits non-zero (carrying the reason it printed, which the
/// log holds too: [`Ctx::answer`]) or prints none.
fn json(ctx: &Ctx, specforge: &Path, args: &[&str]) -> Result<Value, String> {
    let out = ctx.answer(ctx.cmd(specforge).args(args))?;
    serde_json::from_str(&out)
        .map_err(|e| format!("`specforge {}` printed no JSON ({e})", args.join(" ")))
}

#[cfg(test)]
mod tests;
