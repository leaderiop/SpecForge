//! The small JSON state an extension's hooks and commands keep (ADR 0292): `.specforge/state/<extension>/project.json`
//! and `.../sessions/<session>.json`, each an object of at most 64 KiB. The host writes it, as merge patches (RFC 7396)
//! under the write lock, after any plan; a patch that would break the cap is refused whole. State is never project
//! truth: no rule reads it and a compile never depends on it. Sessions untouched for 30 days go at every write.

use std::path::Path;
use std::time::{Duration, SystemTime};

use serde_json::{Map, Value};
use specforge_protocol_types::{StatePatch, StateView};

use crate::asking::Asking;
use crate::change::locked;
use crate::view::ProjectView;
use crate::{Caller, FILE_UNWRITABLE, OpError, OpErrorKind};

/// What a state document may hold, serialized.
pub const CAP: usize = 64 * 1024;

/// A session's state goes after this much idle time.
const SESSION_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// The code of a patch the host refuses.
pub const STATE_REFUSED: &str = "state_refused";

/// Where `extension` keeps its state, under the project root: `@t/core` is `t__core`.
fn directory(extension: &str) -> String {
    format!(
        ".specforge/state/{}",
        extension.trim_start_matches('@').replace('/', "__")
    )
}

/// The document `name` (`project` or `sessions/<id>`) of `extension`, from the project root.
pub fn path(extension: &str, name: &str) -> String {
    format!("{}/{name}.json", directory(extension))
}

/// A session id that cannot leave its directory.
fn session_name(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !id.starts_with('.')
}

fn refused(message: String) -> OpError {
    OpError::new(OpErrorKind::InvalidInput, STATE_REFUSED, message)
}

/// The object at `name`; empty when it is missing, unreadable, over the cap, a link or not an object: state is never
/// what a run depends on. A session read is a use: it keeps the session from being pruned.
fn load(view: &ProjectView<'_>, root: &Path, extension: &str, name: &str) -> Map<String, Value> {
    let relative = path(extension, name);
    if super::link_in(root, &relative).is_some() {
        return Map::new();
    }
    let Some(held) = view.containment().resolve(&root.join(&relative)).ok() else {
        return Map::new();
    };
    let Some(bytes) = held
        .read_if_there()
        .ok()
        .flatten()
        .filter(|b| b.len() <= CAP)
    else {
        return Map::new();
    };
    if name.starts_with("sessions/") {
        // shortcut: the use is the file's mtime; a run that cannot write the file leaves it to age.
        let _ = std::fs::File::options()
            .write(true)
            .open(&held.real)
            .and_then(|file| file.set_modified(SystemTime::now()));
    }
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// What `extension` has kept, for a run of the host `session` (none: a CLI command).
pub fn read(view: &ProjectView<'_>, extension: &str, session: Option<&str>) -> StateView {
    let Some(root) = view.root() else {
        return StateView::default();
    };
    StateView {
        project: load(view, root, extension, "project"),
        session: session
            .filter(|id| session_name(id))
            .map(|id| load(view, root, extension, &format!("sessions/{id}"))),
    }
}

/// RFC 7396: `patch` merged into `target`.
fn merge(target: &mut Value, patch: &Value) {
    let Value::Object(patch) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    if let Value::Object(map) = target {
        for (key, value) in patch {
            if value.is_null() {
                map.remove(key);
            } else {
                merge(map.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
    }
}

/// `patch` applied to `extension`'s state, as one journaled change under the write lock: both documents are checked
/// before either is written, so a patch that leaves a document over [`CAP`] or not an object, or names a session when
/// there is none, changes nothing, and a failure in the middle is put back like any write.
pub fn apply(
    view: &ProjectView<'_>,
    extension: &str,
    session: Option<&str>,
    patch: &StatePatch,
) -> Result<(), OpError> {
    let Some(root) = view.root() else {
        return Ok(());
    };
    let mut documents: Vec<(String, &Value)> = Vec::new();
    if let Some(p) = &patch.project {
        documents.push(("project".into(), p));
    }
    if let Some(p) = &patch.session {
        let Some(id) = session.filter(|id| session_name(id)) else {
            return Err(refused(format!(
                "{extension}: the state patch names a session, and this run has none"
            )));
        };
        documents.push((format!("sessions/{id}"), p));
    }
    if documents.is_empty() {
        return Ok(());
    }
    let caller = Caller::new(view.containment(), Asking::Nobody);
    let sources = view.env().sources().clone();
    locked::commit(
        &sources,
        "cannot write the extension state",
        caller,
        true,
        || Ok(()),
        |stager| {
            for (name, p) in &documents {
                let relative = path(extension, name);
                let held = stager.held(&relative)?;
                let before = held.read_if_there().map_err(unwritable)?;
                let mut merged = Value::Object(
                    before
                        .as_deref()
                        .filter(|b| b.len() <= CAP)
                        .and_then(|b| serde_json::from_slice::<Value>(b).ok())
                        .and_then(|v| match v {
                            Value::Object(map) => Some(map),
                            _ => None,
                        })
                        .unwrap_or_default(),
                );
                merge(&mut merged, p);
                if !merged.is_object() {
                    return Err(refused(format!(
                        "{extension}: the state '{name}' must stay a JSON object"
                    )));
                }
                let bytes = serde_json::to_vec(&merged).expect("a JSON value serializes");
                if bytes.len() > CAP {
                    return Err(refused(format!(
                        "{extension}: the state '{name}' would be {} bytes, over the 64 KiB cap; nothing was written",
                        bytes.len()
                    )));
                }
                stager.write(&relative, before, bytes)?;
            }
            Ok(())
        },
    )?;
    prune(view, root);
    Ok(())
}

fn unwritable(error: std::io::Error) -> OpError {
    OpError::new(
        OpErrorKind::Internal,
        FILE_UNWRITABLE,
        format!("extension state: {error}"),
    )
}

/// Remove the session documents no run used for [`SESSION_AGE`]. Best effort, and only what this store wrote: a
/// regular `.json` file in a real `sessions` directory of a real extension directory, never through a link.
fn prune(view: &ProjectView<'_>, root: &Path) {
    let base = root.join(".specforge/state");
    let real_dir =
        |path: &Path| std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir());
    if !real_dir(&base) {
        return;
    }
    let Ok(extensions) = std::fs::read_dir(&base) else {
        return;
    };
    let now = SystemTime::now();
    for extension in extensions.flatten() {
        let sessions = extension.path().join("sessions");
        if !real_dir(&extension.path()) || !real_dir(&sessions) {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&sessions) else {
            continue;
        };
        for file in files.flatten() {
            let regular = file.file_type().is_ok_and(|t| t.is_file());
            let json = file.path().extension().is_some_and(|e| e == "json");
            let old = file
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age > SESSION_AGE);
            if regular
                && json
                && old
                && let Ok(held) = view.containment().resolve(&file.path())
            {
                let _ = held.remove();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::merge;

    #[test]
    fn merge_follows_rfc_7396() {
        let mut target = json!({"a": "b", "c": {"d": "e", "f": "g"}});
        merge(&mut target, &json!({"a": "z", "c": {"f": null}}));
        assert_eq!(target, json!({"a": "z", "c": {"d": "e"}}));
        merge(&mut target, &json!({"a": ["x"]}));
        assert_eq!(target["a"], json!(["x"]));
    }
}
