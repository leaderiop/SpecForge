//! What the import refuses before it reads a line (ADR 0391 D4, D9, D10): a project that already holds `@gsd/core`
//! entities, a spec root other than `spec`, no `.planning/`, and a host read that stopped early or failed on a file
//! that maps to entities.

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use crate::kinds::CORE_KINDS;
use crate::read::{self, Served};

/// What `specforge.json` says that the import needs, read as text (D11: not a registry query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub spec_root: String,
    /// `extensions` lists `@gsd/security`.
    pub security: bool,
    /// The extensions it lists, each by name (a `name=path` entry gives its name).
    pub loaded: Vec<String>,
}

/// The manifest the `manifest` read served; refused when it is missing, not JSON, or names a spec root other than
/// `spec` (D4: the twins and the fixtures use `spec`, and the spec root is no input a command is given).
pub fn manifest(served: &[Served]) -> Result<Manifest, String> {
    let file = served
        .iter()
        .find(|f| f.path == "specforge.json")
        .ok_or("specforge.json was not read: the import needs the project's manifest")?;
    let text = file.text.as_deref().ok_or_else(|| {
        format!(
            "specforge.json was not read ({})",
            file.error.as_deref().unwrap_or("no text")
        )
    })?;
    let json: Value =
        serde_json::from_str(text).map_err(|e| format!("specforge.json is not JSON: {e}"))?;
    let spec_root = match json.get("spec_root") {
        None => "spec".to_string(),
        Some(Value::String(root)) => root.clone(),
        Some(other) => {
            return Err(format!(
                "specforge.json: spec_root is not a string: {other}"
            ));
        }
    };
    if spec_root != "spec" {
        return Err(format!(
            "specforge.json names the spec root '{}': the import writes spec/gsd/ and needs the spec root 'spec'",
            spec_root.escape_debug()
        ));
    }
    let loaded: Vec<String> = json
        .get("extensions")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(|entry| {
                    entry
                        .split_once('=')
                        .map_or(entry, |(name, _)| name)
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Manifest {
        spec_root,
        security: loaded.iter().any(|name| name == "@gsd/security"),
        loaded,
    })
}

/// Refuses a graph that holds any `@gsd/core` entity (D10), naming the first by id: its kind, path and file.
pub fn refuse_reimport(graph: &CommandGraph) -> Result<(), String> {
    match graph
        .nodes()
        .iter()
        .find(|node| CORE_KINDS.contains(&node.kind.as_str()))
    {
        None => Ok(()),
        Some(node) => Err(format!(
            "the project already holds @gsd/core entities, such as {} {} in {}: an import starts from none; delete them (by default spec/gsd/) and run the import again",
            node.kind,
            node.id,
            node.file.as_deref().unwrap_or("an unknown file")
        )),
    }
}

/// Refuses a `.planning/` read the import cannot trust whole (D9): none at all, one that stopped early, or one that
/// failed on a file that maps to entities. A failed read of any other file is the reader's to report.
pub fn check_reads(planning: &[Served], truncated: bool) -> Result<(), String> {
    if truncated {
        return Err(format!(
            "truncated at {FILE_READ_MAX}: the host's read of .planning/ stopped early (more than {FILE_READ_MAX} files, or a walk past {GLOB_VISITS_MAX} entries); nothing is imported"
        ));
    }
    if planning.is_empty() {
        return Err("no .planning/ at the project root".to_string());
    }
    for file in planning {
        let rel = file.path.strip_prefix(".planning/").unwrap_or(&file.path);
        if let (Some(error), true) = (&file.error, read::maps(rel)) {
            return Err(format!(
                "{} was not read ({error}): its entities would be missing, so nothing is imported",
                file.path.escape_debug()
            ));
        }
    }
    Ok(())
}
