//! What the import refuses before it reads a line (ADR 0391 D4, D9, D10): a project that already holds `@gsd/core`
//! entities, a spec root other than `spec`, no `.planning/`, and a host read that stopped early or failed on a file
//! that maps to entities.

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use crate::kinds::CORE_KINDS;
use crate::read::Served;
use crate::table;

/// What `specforge.json` says that the import needs, read as text (D11: not a registry query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// `extensions` lists `@gsd/security`.
    pub security: bool,
    /// The extensions it lists, each by the name its text gives ([`entry_name`]).
    pub loaded: Vec<String>,
}

/// The manifest the `manifest` read served; refused when it is missing, not JSON, or names a spec root other than
/// `spec` (D4: the twins and the fixtures use `spec`, and the spec root is no input a command is given). An absent
/// spec root and `.` both mean the project root to the host (`ProjectConfig::sources`), which holds `spec/gsd/`: both
/// are accepted.
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
    match json.get("spec_root") {
        None => {}
        Some(Value::String(root)) if root == "spec" || root == "." => {}
        Some(Value::String(root)) => {
            return Err(format!(
                "specforge.json names the spec root '{}': the import writes spec/gsd/ and needs the spec root 'spec'",
                crate::report::escape(root)
            ));
        }
        Some(other) => {
            return Err(format!(
                "specforge.json: spec_root is not a string: {other}"
            ));
        }
    }
    let loaded: Vec<String> = json
        .get("extensions")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .filter_map(entry_name)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Ok(Manifest {
        security: loaded.iter().any(|name| name == "@gsd/security"),
        loaded,
    })
}

/// The extension one `extensions` entry names by its text, read as the host's `ExtensionEntry::parse` reads it (the
/// guest cannot depend on the host crate): trimmed; an entry ending `.wasm` is a file, named by the text before `=`
/// (which the load holds equal to the component's name) and by nothing when bare; any other is `name` or the legacy
/// `name@requirement` (`split_requirement`: the text before the last `@` but a leading one).
fn entry_name(entry: &str) -> Option<&str> {
    let entry = entry.trim();
    if entry.ends_with(".wasm") {
        return entry.split_once('=').map(|(name, _)| name.trim());
    }
    match entry.rfind('@') {
        Some(at) if at > 0 => Some(&entry[..at]),
        _ => Some(entry),
    }
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
        if let (Some(error), true) = (&file.error, table::maps(rel)) {
            return Err(format!(
                "{} was not read ({error}): its entities would be missing, so nothing is imported",
                crate::report::escape(&file.path)
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_is_named_as_the_host_names_it() {
        for (entry, name) in [
            ("@gsd/security", Some("@gsd/security")),
            ("@gsd/security@^1.0", Some("@gsd/security")),
            ("@gsd/security@1.0.0", Some("@gsd/security")),
            (" @gsd/security ", Some("@gsd/security")),
            ("@gsd/security = ../s.wasm", Some("@gsd/security")),
            ("../s.wasm", None),
            // `=` splits a `.wasm` entry only: `x=y` names the extension `x=y`.
            ("x=y", Some("x=y")),
            ("plain", Some("plain")),
        ] {
            assert_eq!(entry_name(entry), name, "{entry:?}");
        }
    }

    fn read(text: &str) -> Result<Manifest, String> {
        manifest(&[Served {
            path: "specforge.json".to_string(),
            text: Some(text.to_string()),
            error: None,
        }])
    }

    #[test]
    fn a_legacy_or_padded_entry_loads_security() {
        let text = r#"{"extensions": ["@gsd/core@^1.1", " @gsd/security@^1.0 "]}"#;
        let manifest = read(text).expect("a manifest");
        assert!(manifest.security);
        assert_eq!(manifest.loaded, ["@gsd/core", "@gsd/security"]);
    }

    #[test]
    fn a_spec_root_holding_spec_gsd_is_accepted_and_any_other_refused() {
        // Absent and `.` are the project root to the host, which holds spec/gsd/ as `spec` does.
        for text in ["{}", r#"{"spec_root": "spec"}"#, r#"{"spec_root": "."}"#] {
            assert!(read(text).is_ok(), "{text}");
        }
        for (text, says) in [
            (r#"{"spec_root": "specs"}"#, "the spec root 'specs'"),
            (r#"{"spec_root": 1}"#, "spec_root is not a string"),
        ] {
            let why = read(text).expect_err("refused");
            assert!(why.contains(says), "{text}: {why}");
        }
    }
}
