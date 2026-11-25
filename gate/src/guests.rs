//! The `guests` check (ADR 0393 D1): each extension guest's tests run under `cargo nextest run`, one run per guest
//! manifest (each guest is its own workspace, ADR 0366 D2), and each run writes its JUnit report under the repository
//! root's `target/nextest/<profile>/junit-guests/<slug>/`, and a guest whose sources hold a runnable doctest runs it
//! under `cargo test --doc` (nextest runs none); then every guest is clippy-clean for wasm and native, and
//! each example project's extension for wasm.

use std::path::Path;

use crate::checks::Selection;
use crate::dogfood::{GUESTS, clear_reports, report_dir};
use crate::repo::{EXTENSION_CRATES, Repo};
use crate::run::Ctx;
use crate::steps::finish;

/// The guests' nextest profiles, without a store or a JUnit path: [`config`] adds both per run.
const BASE: &str = ".config/nextest-guests.toml";

pub fn run(ctx: &Ctx, _: &Selection) -> Result<(), String> {
    let target = ctx.ext_target();
    let profile = ctx.mode.nextest_profile();
    clear_reports(&report_dir(&ctx.repo.root, profile).join(GUESTS))?;
    let base = std::fs::read_to_string(ctx.repo.root.join(BASE))
        .map_err(|e| format!("reading {BASE}: {e}"))?;
    let store = ctx.repo.root.join("target/nextest");
    let configs = ctx.repo.target_dir.join("gate/nextest-guests");
    std::fs::create_dir_all(&configs).map_err(|e| format!("{}: {e}", configs.display()))?;
    // (what, cargo arguments before the manifest, after it)
    let clippy: [(&str, &[&str], &[&str]); 2] = [
        (
            "wasm clippy",
            &["clippy", "-q", "--target", "wasm32-wasip2"],
            &["--", "-D", "warnings"],
        ),
        (
            "native clippy",
            &["clippy", "-q", "--all-targets"],
            &["--", "-D", "warnings"],
        ),
    ];
    let mut failures = Vec::new();
    let cargo = |args: &[&str], manifest: &str, after: &[&str]| {
        let mut cmd = ctx.cmd("cargo");
        // What `build-blobs` does: an incremental guest is another build than CI's.
        cmd.env("CARGO_TARGET_DIR", &target)
            .env_remove("CARGO_INCREMENTAL");
        cmd.args(args)
            .args(["--manifest-path", manifest])
            .args(after);
        cmd
    };
    for (manifest, slug) in slugged(guests_of(ctx.repo))? {
        let file = configs.join(format!("{slug}.toml"));
        std::fs::write(&file, config(&base, &store, profile, &slug)?)
            .map_err(|e| format!("{}: {e}", file.display()))?;
        let file = file.to_string_lossy();
        let tests = [
            "nextest",
            "run",
            "--no-fail-fast",
            "--no-tests=pass",
            "--profile",
            profile,
            "--config-file",
            &file,
        ];
        if let Err(e) = ctx.sh(&mut cargo(&tests, &manifest, &[])) {
            failures.push(format!("{manifest} tests: {e}"));
        }
        // nextest runs no doctest, and the `doctests` check covers root members only.
        let dir = manifest.strip_suffix("/Cargo.toml").unwrap_or(&manifest);
        if crate::checks::member_has_doctest(ctx.repo, dir)
            && let Err(e) = ctx.sh(&mut cargo(&["test", "-q", "--doc"], &manifest, &[]))
        {
            failures.push(format!("{manifest} doctests: {e}"));
        }
        for &(what, before, after) in &clippy {
            if let Err(e) = ctx.sh(&mut cargo(before, &manifest, after)) {
                failures.push(format!("{manifest} {what}: {e}"));
            }
        }
    }
    // An example project's extension gets the wasm clippy pass only: it has no tests and no native use.
    for manifest in crate::repo::expand(&ctx.repo.root, "examples/*/extension/Cargo.toml") {
        let (what, before, after) = clippy[0];
        if let Err(e) = ctx.sh(&mut cargo(before, &manifest, after)) {
            failures.push(format!("{manifest} {what}: {e}"));
        }
    }
    finish(failures)
}

/// One guest's nextest config: a `[store]` at `store` (a TOML basic string, its `\` and `"` escaped, so any path
/// holds, a Windows one or one with an apostrophe), the committed `base`, then the JUnit path
/// `junit-guests/<slug>/junit.xml` of `profile`. Refuses a store holding a control character and a slug outside ASCII
/// lowercase letters, digits and `-`.
pub(crate) fn config(
    base: &str,
    store: &Path,
    profile: &str,
    slug: &str,
) -> Result<String, String> {
    let store = store.to_string_lossy();
    if store.contains(char::is_control) {
        return Err(format!(
            "the nextest store {store:?} holds a control character"
        ));
    }
    let store = store.replace('\\', "\\\\").replace('"', "\\\"");
    if slug.is_empty()
        || !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(format!(
            "the guest slug {slug:?} is not ASCII lowercase letters, digits and `-`"
        ));
    }
    let end = if base.ends_with('\n') { "" } else { "\n" };
    Ok(format!(
        "[store]\ndir = \"{store}\"\n\n{base}{end}[profile.{profile}.junit]\npath = \"{GUESTS}/{slug}/junit.xml\"\n"
    ))
}

/// The guest's directory under `extensions/`, each `/` a `-`: `extensions/rust/test/Cargo.toml` is `rust-test`.
pub(crate) fn slug(manifest: &str) -> String {
    let dir = manifest.strip_suffix("/Cargo.toml").unwrap_or(manifest);
    dir.strip_prefix("extensions/")
        .unwrap_or(dir)
        .replace('/', "-")
}

/// Each manifest with its [`slug`], in order; an error naming both manifests when two share a slug, since the second
/// run would overwrite the first's config and JUnit report.
pub(crate) fn slugged(manifests: Vec<String>) -> Result<Vec<(String, String)>, String> {
    let mut out: Vec<(String, String)> = Vec::new();
    for manifest in manifests {
        let slug = slug(&manifest);
        if let Some((first, _)) = out.iter().find(|(_, s)| *s == slug) {
            return Err(format!(
                "{first} and {manifest} share the guest slug `{slug}`: one would overwrite the other's JUnit report"
            ));
        }
        out.push((manifest, slug));
    }
    Ok(out)
}

/// The guests' and the extension libraries' manifests: `extensions/<dir>/Cargo.toml` and
/// `extensions/<family>/<pack>/Cargo.toml` (ADR 0328 D3).
fn guests_of(repo: &Repo) -> Vec<String> {
    EXTENSION_CRATES
        .iter()
        .flat_map(|dirs| crate::repo::expand(&repo.root, &format!("{dirs}/Cargo.toml")))
        .collect()
}

#[cfg(test)]
mod tests;
