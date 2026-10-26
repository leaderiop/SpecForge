//! `<crate_dir>/wasm/inputs.json`: what one committed blob (a builtin under `extensions/`, a fixture under
//! `fixtures/` or an example project's own extension under `examples/<name>/extension`) was built from. `build-blobs` writes and checks it through this module; a quick run asks it which
//! blobs a change set feeds. A blob is named by its crate directory from the repository root. The JSON is unchanged
//! byte for byte.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::change_set::ChangeSet;

pub const FILE: &str = "inputs.json";
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

/// The crates that commit a blob, as `repo::expand` patterns: the builtins, the fixtures and the example projects'
/// extensions.
pub const BLOB_CRATES: &[&str] = &["extensions/*", "fixtures/*", "examples/*/extension"];

/// Every committed blob with a record: the crate directories (`extensions/<dir>`, `fixtures/<dir>`,
/// `examples/<dir>/extension`, sorted) whose `wasm/inputs.json` exists.
pub fn blobs(repo_root: &Path) -> Result<Vec<String>, String> {
    let suffix = format!("/wasm/{FILE}");
    let mut found: Vec<String> = BLOB_CRATES
        .iter()
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
