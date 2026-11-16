//! What reaches the sample (ADR 0360 D17), derived from what its run builds and reads rather than listed by hand: the
//! files it reads beside the workspace ([`INPUTS`]), and every source cargo builds into one of its [`PROGRAMS`] or
//! into a test it cites, read from the workspace's dependency graph. A path dependency outside `crates/` (the test
//! support under `integrations/`), a module the converter gains, or a library it links (`xtask`'s own) reaches it
//! as soon as cargo builds it in.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use super::GOLDENS;
use crate::change_set::ChangeSet;
use crate::checks::Selection;
use crate::repo::{DependencyKind, Member, Repo};

/// What the run reads beside the members' sources: the spec and manifest it converts, the `@gsd` workspace its wasm is
/// built from, the builtins the CLI embeds, the toolchain and the build's configuration.
pub const INPUTS: &[&str] = &[
    "spec/**",
    "specforge.json",
    "gsd/**",
    "extensions/**",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/config.toml",
    ".config/nextest.toml",
];

/// The programs the run builds and calls, as `(package, binary)`: the CLI, the converter, and the gate that runs it.
pub const PROGRAMS: [(&str, &str); 3] = [
    ("specforge-cli", "specforge"),
    ("xtask", "absorb"),
    ("gate", "gate"),
];

/// The check's trigger: everything when a changed path is one of [`INPUTS`] or a source the run builds
/// ([`built_into_the_run`]), else nothing.
pub fn reached(repo: &Repo, changes: &ChangeSet) -> Result<Selection, String> {
    if !changes.matching(INPUTS).is_empty() {
        return Ok(Selection::Everything);
    }
    let cited = cited_packages(&repo.root.join(GOLDENS).join("ledger.json"))?;
    let built = built_from(repo, &cited);
    Ok(
        if changes
            .paths()
            .any(|path| built_into_the_run(repo, &built, &cited, path))
        {
            Selection::Everything
        } else {
            Selection::Skip
        },
    )
}

/// The packages holding a test the sample cites, read from the ledger golden: every `rust:<binary id>#<test>` it
/// holds, at any depth, names its package before the binary id's first `::`.
pub fn cited_packages(ledger: &Path) -> Result<BTreeSet<String>, String> {
    let text = std::fs::read_to_string(ledger)
        .map_err(|e| format!("the sample's ledger golden {}: {e}", ledger.display()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| format!("the sample's ledger golden {}: {e}", ledger.display()))?;
    let mut found = BTreeSet::new();
    let mut todo = vec![&value];
    while let Some(value) = todo.pop() {
        match value {
            Value::String(id) => {
                if let Some((binary, _)) = id.strip_prefix("rust:").and_then(|r| r.split_once('#'))
                {
                    let package = binary
                        .split_once("::")
                        .map_or(binary, |(package, _)| package);
                    found.insert(package.to_string());
                }
            }
            Value::Array(items) => todo.extend(items),
            Value::Object(fields) => todo.extend(fields.values()),
            _ => {}
        }
    }
    Ok(found)
}

/// The members the run builds from: each program's package and each cited test's, with their normal and build
/// dependencies at any depth; a cited test's package adds its dev-dependencies, which its tests build with.
pub fn built_from(repo: &Repo, cited: &BTreeSet<String>) -> BTreeSet<String> {
    let mut todo: Vec<&str> = PROGRAMS.iter().map(|(package, _)| *package).collect();
    for package in cited {
        todo.push(package);
        if let Some(member) = member(repo, package) {
            todo.extend(member.dependencies.iter().map(|d| d.name.as_str()));
        }
    }
    let mut seen = BTreeSet::new();
    while let Some(name) = todo.pop() {
        if !seen.insert(name.to_string()) {
            continue;
        }
        if let Some(member) = member(repo, name) {
            todo.extend(
                member
                    .dependencies
                    .iter()
                    .filter(|d| d.kind != DependencyKind::Dev)
                    .map(|d| d.name.as_str()),
            );
        }
    }
    seen
}

/// Whether `path` is a source cargo builds into the run: a file of a member in `built`, unless it belongs to a binary
/// the run never builds. A dependency is built as its library; a cited test's package builds its binaries too (its
/// tests run them); a program's package builds that program. A binary's files are its root and `src/<name>/`, where
/// the repository keeps a binary's modules (`#[path]`); a binary whose modules sit beside the library's reaches
/// through them, never less.
pub fn built_into_the_run(
    repo: &Repo,
    built: &BTreeSet<String>,
    cited: &BTreeSet<String>,
    path: &str,
) -> bool {
    let Some(member) = repo.member_of(path) else {
        return false;
    };
    if !built.contains(&member.name) {
        return false;
    }
    let runs =
        |bin: &str| cited.contains(&member.name) || PROGRAMS.contains(&(member.name.as_str(), bin));
    !member.bins.iter().any(|(bin, root)| {
        !runs(bin)
            && (path == root
                || root
                    .strip_suffix(".rs")
                    .is_some_and(|dir| path.starts_with(&format!("{dir}/"))))
    })
}

fn member<'a>(repo: &'a Repo, name: &str) -> Option<&'a Member> {
    repo.members.iter().find(|m| m.name == name)
}

#[cfg(test)]
mod tests;
