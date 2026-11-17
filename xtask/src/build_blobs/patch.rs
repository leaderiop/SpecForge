//! The SDK patch of a crate's own workspace, replayed in its stage (ADR 0366 D2).
//!
//! The gsd workspace builds against this checkout's SDK through the `[patch.crates-io]` table of
//! `gsd/.cargo/config.toml`, whose paths are relative to `gsd/`. The stage mirrors the repository's
//! layout from its root, so each entry is re-rooted at the repository and written into the stage's
//! `Cargo.toml`. Nothing is hard-coded: deleting that config file is still the only change the
//! split of the gsd workspace into its own repository needs.

use std::path::{Component, Path};

/// The `[patch.crates-io]` table for the stage, read from `config` (a `.cargo/config.toml` that
/// lies in `base/.cargo/`, `base` from the repository root): each entry is an inline table holding
/// only a `path`, re-rooted from `base` to the repository root. An entry with another key, a
/// patched registry other than crates.io, or a path that is absolute or leaves the repository is an
/// error naming it. A config without the table gives an empty string.
pub(super) fn stage_patch(config: &Path, base: &str) -> Result<String, String> {
    let shown = config.display();
    let text = std::fs::read_to_string(config).map_err(|e| format!("read {shown}: {e}"))?;
    let table: toml::Table = text.parse().map_err(|e| format!("{shown}: {e}"))?;
    let Some(patch) = table.get("patch") else {
        return Ok(String::new());
    };
    let patch = patch
        .as_table()
        .ok_or_else(|| format!("{shown}: `patch` is not a table"))?;
    let mut out = String::new();
    for (registry, entries) in patch {
        if registry != "crates-io" {
            return Err(format!(
                "{shown}: [patch.{registry}] patches a registry the stage does not replay"
            ));
        }
        let entries = entries
            .as_table()
            .ok_or_else(|| format!("{shown}: [patch.crates-io] is not a table"))?;
        out.push_str("[patch.crates-io]\n");
        for (name, entry) in entries {
            let path = entry
                .as_table()
                .filter(|fields| fields.len() == 1)
                .and_then(|fields| fields.get("path"))
                .and_then(toml::Value::as_str)
                .ok_or_else(|| {
                    format!("{shown}: the patch of {name} is not a lone `path`: {entry}")
                })?;
            let rerooted = reroot(base, path).ok_or_else(|| {
                format!("{shown}: the patch of {name} ({path}) leaves the repository")
            })?;
            out.push_str(&format!(
                "{name} = {{ path = {} }}\n",
                toml::Value::String(rerooted)
            ));
        }
    }
    Ok(out)
}

/// `path` (relative to `base`, both `/`-separated from the repository root) from the repository
/// root, normalised; `None` when it is absolute or climbs above the root.
fn reroot(base: &str, path: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for component in Path::new(base).join(path).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}
