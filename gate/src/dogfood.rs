//! The `dogfood` check (ADR 0374 D5): the repository's own tests prove its requirements.
//!
//! nextest writes a JUnit report under its `gate` and `ci` profiles (`<repository root>/target/nextest/<profile>/`;
//! CI downloads each test partition's into its own directory there). Every `junit*.xml` found is merged into the one
//! report `@rust/test` reads ([`crate::junit::merge`]). In a copy of the tracked tree ([`crate::checked_tree`]: no
//! fact store, and no untracked copy of the sources, of this machine is in it), the root project is collected by
//! `rust-cargo`, `rust-source` and `rust-test` over that report; then each requirement's computed `proof` is read
//! from `specforge export --format graph --kinds requirement` and judged by [`crate::judge::requirement_verdict`].
//! The requirements the guests' tests name come from `specforge anchors --file extensions`: guests run under `cargo
//! test` and write no JUnit, so what only they prove reads `no_test`, and the verdict counts it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::checks::Selection;
use crate::run::Ctx;
use crate::{checked_tree, judge, junit, steps};

pub fn run(ctx: &Ctx, _: &Selection) -> Result<(), String> {
    let dir = report_dir(&ctx.repo.root, ctx.mode.nextest_profile());
    let found = reports(&dir);
    if found.is_empty() {
        return Err(format!(
            "no nextest JUnit report under {}: run `cargo gate full --only tests` first",
            dir.display()
        ));
    }
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
    checked_tree::copy_tracked(&ctx.repo.root, &steps::tracked_paths(ctx)?, &tree)?;
    let tree = tree.to_string_lossy().into_owned();
    let merged = merged.to_string_lossy().into_owned();
    let online = ["--yes", "--effects", "online", "--format", "json"];
    for runner in [
        &["rust-cargo"][..],
        &["rust-source"],
        &["rust-test", "--report", &merged],
    ] {
        let mut args = vec!["collect", "--path", &tree, "--runner"];
        args.extend(runner);
        args.extend(online);
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
    let anchors = json(
        ctx,
        specforge,
        &[
            "anchors",
            "--path",
            &tree,
            "--file",
            "extensions",
            "--format",
            "json",
        ],
    )?;
    let (lines, failed) = judge::requirement_verdict(&export, &anchored(&anchors));
    for line in &lines {
        ctx.log(line);
    }
    if failed {
        // The first line is the counts; the problems follow it.
        Err(lines.get(1).or(lines.first()).cloned().unwrap_or_default())
    } else {
        Ok(())
    }
}

/// `<root>/target/nextest/<profile>`: where nextest writes the JUnit report of a run under `profile` (its store is
/// the workspace root's `target/`, whatever the target directory).
pub(crate) fn report_dir(root: &Path, profile: &str) -> PathBuf {
    root.join("target/nextest").join(profile)
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

/// The entity ids `specforge anchors --file … --format json` lists.
fn anchored(anchors: &Value) -> BTreeSet<String> {
    anchors["entities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e["entity_id"].as_str().map(String::from))
        .collect()
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
