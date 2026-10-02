//! Renaming an entity ID: the MCP `specforge.rename` tool and the LSP's
//! `textDocument/rename` plan the same edits under the same rules. MCP
//! applies them ([`apply`]) and recompiles; the LSP hands them to the
//! editor.

use crate::OpError;
use specforge_graph::Graph;
use specforge_graph::rename::{RenameEdit, apply_edits, identifier_edits};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::Path;

/// The new ID is not a legal entity ID.
pub const INVALID_ID: &str = "invalid_name";
/// The entity to rename does not exist.
pub const NOT_FOUND: &str = "entity_not_found";
/// The new ID already names an entity.
pub const TAKEN: &str = "entity_exists";
/// A file the rename must edit cannot be read (or written).
pub const UNREADABLE: &str = "file_unreadable";

/// The shortest and longest entity IDs (E014).
pub const ID_LENGTH: std::ops::RangeInclusive<usize> = 2..=60;

/// Whether `id` is a legal entity ID: what the grammar's identifier accepts
/// (an ASCII letter or `_`, then letters, digits and `_`) within E014's
/// 2-60 characters.
pub fn validate_id(id: &str) -> Result<(), OpError> {
    let mut chars = id.chars();
    let starts = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    let rest = chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if starts && rest && ID_LENGTH.contains(&id.len()) {
        return Ok(());
    }
    Err(OpError::new(
        INVALID_ID,
        format!(
            "invalid entity ID '{id}': it must be 2-60 characters, letters, digits and \
             underscores, starting with a letter or underscore"
        ),
    ))
}

/// The edits that rename one entity: its declaration and every reference.
#[derive(Debug, Clone)]
pub struct RenamePlan {
    pub old_id: String,
    pub new_id: String,
    /// One per occurrence; files are relative to the spec root.
    pub edits: Vec<RenameEdit>,
}

impl RenamePlan {
    /// The files the edits touch, sorted.
    pub fn affected_files(&self) -> BTreeSet<&str> {
        self.edits.iter().map(|e| e.file.as_str()).collect()
    }
}

/// Plan renaming `old_id` to `new_id` in `graph`, reading each file the
/// rename touches through `text_of` (a path relative to the spec root).
/// A rename is all or nothing: one it cannot read every file of is
/// refused, rather than leaving a reference behind.
pub fn plan(
    graph: &Graph,
    old_id: &str,
    new_id: &str,
    text_of: impl Fn(&str) -> Option<String>,
) -> Result<RenamePlan, OpError> {
    validate_id(new_id)?;
    if graph.node(old_id).is_none() {
        return Err(OpError::new(
            NOT_FOUND,
            format!("Entity not found: {old_id}"),
        ));
    }
    if graph.node(new_id).is_some() {
        return Err(OpError::new(
            TAKEN,
            format!("cannot rename '{old_id}': '{new_id}' exists"),
        ));
    }
    let unreadable = RefCell::new(BTreeSet::new());
    let edits = identifier_edits(graph, old_id, new_id, |file| {
        let text = text_of(file);
        if text.is_none() {
            unreadable.borrow_mut().insert(file.to_string());
        }
        text
    })
    .unwrap_or_default();
    let unreadable = unreadable.into_inner();
    if !unreadable.is_empty() {
        let files: Vec<String> = unreadable.into_iter().collect();
        return Err(OpError::new(
            UNREADABLE,
            format!("cannot rename '{old_id}': cannot read {}", files.join(", ")),
        ));
    }
    Ok(RenamePlan {
        old_id: old_id.to_string(),
        new_id: new_id.to_string(),
        edits,
    })
}

/// Write `plan`'s edits to the files under `spec_root`: every file is
/// read and edited first, then written; if a write fails, the files
/// already written, and the one that failed part-way, get their old text
/// back.
pub fn apply(plan: &RenamePlan, spec_root: &Path) -> Result<(), OpError> {
    let mut changes = Vec::new();
    for file in plan.affected_files() {
        let path = spec_root.join(file);
        let old = std::fs::read_to_string(&path).map_err(|e| {
            OpError::new(
                UNREADABLE,
                format!("failed to read {}: {e}", path.display()),
            )
        })?;
        let new = apply_edits(&old, plan.edits.iter().filter(|e| e.file == file));
        changes.push((path, old, new));
    }
    for (i, (path, _, new)) in changes.iter().enumerate() {
        if let Err(e) = std::fs::write(path, new) {
            for (written, old, _) in &changes[..=i] {
                let _ = std::fs::write(written, old);
            }
            return Err(OpError::new(
                UNREADABLE,
                format!("failed to write {}: {e}", path.display()),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entity_id_follows_the_grammar_and_e014() {
        for ok in ["ab", "_x", "auth_token", "A1", &"a".repeat(60)] {
            assert!(validate_id(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "a",
            "1abc",
            "has-dash",
            "has space",
            "émile",
            &"a".repeat(61),
        ] {
            assert_eq!(validate_id(bad).unwrap_err().code, INVALID_ID, "{bad}");
        }
    }

    fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, Graph) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("specforge.json"),
            r#"{"name":"r","extensions":["@specforge/software"]}"#,
        )
        .unwrap();
        for (name, text) in files {
            std::fs::write(dir.path().join(name), text).unwrap();
        }
        let graph = specforge_project::CompiledProject::compile(dir.path(), None)
            .into_context()
            .graph;
        (dir, graph)
    }

    const LIMIT: &str = "invariant session_limit \"Limit\" {\n  guarantee \"x\"\n}\n";
    const LOGIN: &str = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n";

    #[test]
    fn a_plan_renames_the_declaration_and_every_reference() {
        let (dir, graph) = project(&[("limit.spec", LIMIT), ("login.spec", LOGIN)]);
        let read = |f: &str| std::fs::read_to_string(dir.path().join(f)).ok();

        let plan = plan(&graph, "session_limit", "session_cap", read).unwrap();
        assert_eq!(
            plan.affected_files().into_iter().collect::<Vec<_>>(),
            ["limit.spec", "login.spec"]
        );
        apply(&plan, dir.path()).unwrap();
        let login = std::fs::read_to_string(dir.path().join("login.spec")).unwrap();
        assert!(login.contains("invariants [session_cap]"), "{login}");
        let limit = std::fs::read_to_string(dir.path().join("limit.spec")).unwrap();
        assert!(limit.starts_with("invariant session_cap "), "{limit}");
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_write_puts_every_file_back() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, graph) = project(&[("limit.spec", LIMIT), ("login.spec", LOGIN)]);
        let read = |f: &str| std::fs::read_to_string(dir.path().join(f)).ok();
        let plan = plan(&graph, "session_limit", "session_cap", read).unwrap();
        // limit.spec is written first; login.spec cannot be.
        let login = dir.path().join("login.spec");
        std::fs::set_permissions(&login, std::fs::Permissions::from_mode(0o444)).unwrap();
        if std::fs::OpenOptions::new().write(true).open(&login).is_ok() {
            return; // permissions don't bind this user (root)
        }

        let error = apply(&plan, dir.path()).unwrap_err();

        assert_eq!(error.code, UNREADABLE);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("limit.spec")).unwrap(),
            LIMIT
        );
        assert_eq!(std::fs::read_to_string(&login).unwrap(), LOGIN);
    }

    #[test]
    fn a_plan_refuses_what_it_cannot_do_whole() {
        let (dir, graph) = project(&[("limit.spec", LIMIT), ("login.spec", LOGIN)]);
        let read = |f: &str| std::fs::read_to_string(dir.path().join(f)).ok();
        let code = |r: Result<RenamePlan, OpError>| r.unwrap_err().code;

        assert_eq!(code(plan(&graph, "session_limit", "x", read)), INVALID_ID);
        assert_eq!(code(plan(&graph, "nope", "fine_name", read)), NOT_FOUND);
        assert_eq!(code(plan(&graph, "session_limit", "login", read)), TAKEN);
        let without_limit = |f: &str| {
            (f != "limit.spec").then(|| std::fs::read_to_string(dir.path().join(f)).unwrap())
        };
        assert_eq!(
            code(plan(&graph, "session_limit", "session_cap", without_limit)),
            UNREADABLE
        );
    }
}
