//! `<crate_dir>/wasm/inputs.json`: what one committed blob (a builtin under `extensions/`, a fixture under
//! `fixtures/`, an example project's own extension under `examples/<name>/extension`, or `@gsd/core` and
//! `@gsd/import` under `gsd/core` and `gsd/import`, the two builtins outside `extensions/`) was built from.
//! `build-blobs` writes and checks it through this module; a quick run asks it which blobs a change set feeds. A blob is named by its crate directory from the
//! repository root. The JSON is unchanged byte for byte.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::change_set::ChangeSet;

pub const FILE: &str = "inputs.json";

/// The rustc every committed blob is built with (ADR 0373 D1). The stager and `revendor` run every cargo and rustc of
/// a blob build on it ([`on_blob_rustc`]), `built_with` records its version, and `build-blobs --verify` fails on a
/// blob another rustc built. Changing it re-vendors every blob in the same commit; the bootstrap job installs it.
pub const BLOB_RUSTC: &str = "1.99.1";

/// The command that installs [`BLOB_RUSTC`] with the blobs' target, for a message that names the fix.
pub fn blob_rustc_install() -> String {
    format!("rustup toolchain install {BLOB_RUSTC} --profile minimal --target wasm32-wasip2")
}

/// The `--version` of `rustc` (the rustc program, run here on [`BLOB_RUSTC`]), or an error naming the install command
/// when it does not run. Whether it is the pin is [`pin_problem`]'s question.
pub fn blob_rustc_version(mut rustc: Command) -> Result<String, String> {
    let output = on_blob_rustc(&mut rustc)
        .arg("--version")
        .output()
        .map_err(|e| format!("spawn rustc: {e}"))?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || version.is_empty() {
        return Err(format!(
            "the pinned rustc {BLOB_RUSTC} does not run ({}); install it: {}",
            String::from_utf8_lossy(&output.stderr).trim(),
            blob_rustc_install()
        ));
    }
    Ok(version)
}

/// Why `version` (a `rustc --version`) is not the pinned rustc, if it is not.
pub fn pin_problem(version: &str) -> Option<String> {
    (!version.starts_with(&format!("rustc {BLOB_RUSTC} "))).then(|| {
        format!(
            "the rustc a blob build runs is {version}, not the pinned {BLOB_RUSTC} \
             (gate::blob_inputs::BLOB_RUSTC); install it: {}",
            blob_rustc_install()
        )
    })
}

/// The `--version` of `rustc` on [`BLOB_RUSTC`], refused unless it is the pin (no rustup selects it, or the toolchain
/// is missing), the refusal naming the install command.
pub fn pinned_rustc(rustc: Command) -> Result<String, String> {
    let version = blob_rustc_version(rustc)?;
    match pin_problem(&version) {
        Some(problem) => Err(problem),
        None => Ok(version),
    }
}

/// Put `command` (a cargo or rustc of a blob build) on [`BLOB_RUSTC`]: `RUSTUP_TOOLCHAIN` names it, which replaces
/// the toolchain a parent `cargo run` sets for its child, and `RUSTC` and `CARGO_BUILD_RUSTC` are removed, since
/// either would override the rustc rustup selects (ADR 0373 D1).
pub fn on_blob_rustc(command: &mut Command) -> &mut Command {
    command
        .env("RUSTUP_TOOLCHAIN", BLOB_RUSTC)
        .env_remove("RUSTC")
        .env_remove("CARGO_BUILD_RUSTC")
}
/// The key for the root manifest's tables a staged crate inherits: `[workspace.package]` (edition, version,
/// license), `[workspace.dependencies]` (dependency specs) the two `[workspace.lints]` tables (the lint levels
/// its `[lints] workspace = true` reads) and `[profile.blob]` (how it is optimised). Only these can change how a staged crate builds.
pub const WORKSPACE_INHERITED: &str = "Cargo.toml#workspace-inherited";
pub const INHERITED_TABLES: &[&str] = &[
    "[workspace.package]",
    "[workspace.dependencies]",
    "[workspace.lints.rust]",
    "[workspace.lints.clippy]",
    "[profile.blob]",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRecord {
    pub artifact: String,
    pub artifact_sha256: String,
    /// The `rustc --version` of [`BLOB_RUSTC`] the blob was built with.
    pub built_with: String,
    /// Recorded input key -> SHA-256 of what the key names.
    pub inputs: BTreeMap<String, String>,
}

impl BlobRecord {
    pub fn read(repo_root: &Path, crate_dir: &str) -> Result<BlobRecord, String> {
        let path = record_path(repo_root, crate_dir);
        let bytes = std::fs::read(&path)
            .map_err(|_| format!("{} is missing or unreadable", shown(crate_dir)))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| format!("{} is malformed: {e}", shown(crate_dir)))
    }

    /// Pretty JSON and a final newline, as `build-blobs --install` wrote it at the base.
    pub fn write(&self, repo_root: &Path, crate_dir: &str) -> Result<(), String> {
        let mut text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        text.push('\n');
        let path = record_path(repo_root, crate_dir);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

fn record_path(repo_root: &Path, crate_dir: &str) -> PathBuf {
    repo_root.join(shown(crate_dir))
}

fn shown(crate_dir: &str) -> String {
    format!("{crate_dir}/wasm/{FILE}")
}

/// The repo path an input key names: `Cargo.toml` for [`WORKSPACE_INHERITED`], else the key.
pub fn path_of(key: &str) -> &str {
    if key == WORKSPACE_INHERITED {
        "Cargo.toml"
    } else {
        key
    }
}

/// The tables of the root manifest a staged crate inherits.
pub fn inherited_tables(manifest: &str) -> String {
    let mut out = String::new();
    let mut keep = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            keep = INHERITED_TABLES.contains(&trimmed);
        }
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// The crates that commit a blob besides the builtins under `extensions/` ([`crate::repo::EXTENSION_CRATES`]), as
/// `repo::expand` patterns: the fixtures, the example projects' extensions, `@gsd/core` (ADR 0366 D2) and
/// `@gsd/import` (ADR 0391 D12), so a `gsd/` input feeds the `builtins` check once its record exists.
pub const BLOB_CRATES: &[&str] = &[
    "fixtures/*",
    "examples/*/extension",
    "gsd/core",
    "gsd/import",
];

/// The gsd workspace, from the repository root: one Cargo workspace with one lock and its own `.cargo/config.toml`.
const GSD_WORKSPACE: &str = "gsd";

/// Whether `crate_dir` is a crate of the gsd workspace, which builds as that workspace does (ADR 0366 D2).
fn in_gsd(crate_dir: &str) -> bool {
    crate_dir
        .strip_prefix(GSD_WORKSPACE)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// The lock a crate's blob resolves with, from the repository root: `gsd/Cargo.lock` for a crate of the gsd workspace,
/// else the crate's own `<crate_dir>/Cargo.lock` (ADR 0366 D2).
pub fn lock_of(crate_dir: &str) -> String {
    if in_gsd(crate_dir) {
        format!("{GSD_WORKSPACE}/Cargo.lock")
    } else {
        format!("{crate_dir}/Cargo.lock")
    }
}

/// The directory cargo runs in for a crate, from the repository root (`.` for the root): cargo reads the
/// `.cargo/config.toml` of its working directory, not of `--manifest-path`, so a crate of the gsd workspace runs in
/// `gsd` to take that workspace's SDK patch (ADR 0366 D2).
pub fn cargo_dir(crate_dir: &str) -> &'static str {
    if in_gsd(crate_dir) {
        GSD_WORKSPACE
    } else {
        "."
    }
}

/// The cargo config a crate's build reads besides the repository root's, from the repository root:
/// `gsd/.cargo/config.toml` for a crate of the gsd workspace (its `[patch.crates-io]` decides which SDK the blob
/// compiles), else none (ADR 0366 D2).
pub fn cargo_config_of(crate_dir: &str) -> Option<String> {
    in_gsd(crate_dir).then(|| format!("{GSD_WORKSPACE}/.cargo/config.toml"))
}

/// The workspace files a crate's blob is recorded as built from, besides what rustc reads: its lock and, for a crate
/// of the gsd workspace, that workspace's cargo config (ADR 0366 D2).
pub fn workspace_inputs(crate_dir: &str) -> Vec<String> {
    std::iter::once(lock_of(crate_dir))
        .chain(cargo_config_of(crate_dir))
        .collect()
}

/// Every committed blob with a record: the crate directories (`extensions/<dir>`, `extensions/<family>/<pack>`,
/// `fixtures/<dir>`, `examples/<dir>/extension`, `gsd/core`, `gsd/import`, sorted) whose `wasm/inputs.json` exists;
/// an extension library has none, so it is no blob.
pub fn blobs(repo_root: &Path) -> Result<Vec<String>, String> {
    let suffix = format!("/wasm/{FILE}");
    let mut found: Vec<String> = crate::repo::EXTENSION_CRATES
        .iter()
        .chain(BLOB_CRATES)
        .flat_map(|pattern| crate::repo::expand(repo_root, &format!("{pattern}{suffix}")))
        .filter_map(|record| record.strip_suffix(&suffix).map(str::to_owned))
        .collect();
    found.sort();
    Ok(found)
}

/// The blobs (crate directories, sorted) whose recorded inputs hold a changed path.
pub fn fed_by(repo_root: &Path, changes: &ChangeSet) -> Result<Vec<String>, String> {
    let mut fed = Vec::new();
    for crate_dir in blobs(repo_root)? {
        let record = BlobRecord::read(repo_root, &crate_dir)?;
        if changes
            .paths()
            .any(|path| record.inputs.keys().any(|key| path_of(key) == path))
        {
            fed.push(crate_dir);
        }
    }
    Ok(fed)
}

#[cfg(test)]
mod tests;
