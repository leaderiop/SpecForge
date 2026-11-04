//! The user's consent store (ADR 0335 D2.5, ADR 0193): every approval this machine gives, for every project, in
//! `~/.specforge/consent.json`. It lives outside every project, so a cloned repository cannot approve its own
//! effects. Each change is read, changed and written under an exclusive lock on the sidecar `consent.json.lock`,
//! and written privately and atomically (`replace_private`: 0600, a new parent 0700, a rename). A store that cannot
//! be read or parsed, or that a newer specforge wrote, is never written over: nothing in it counts (W186).

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use specforge_common::fresh::{open_private_lock, private_parent, replace_private};
use specforge_common::{Diagnostic, codes};
use specforge_effects::{Approvals, Consent, ConsentError};

/// Overrides the user-level consent store (tests, custom homes). Refused when it lies inside the project.
pub const CONSENT_FILE_ENV: &str = "SPECFORGE_CONSENT_FILE";

/// The store format this specforge reads and writes.
const VERSION: u64 = 1;

#[derive(Serialize, Deserialize)]
struct Stored {
    version: u64,
    #[serde(flatten)]
    approvals: Approvals,
}

/// The consent store in a file.
#[derive(Debug, Clone)]
pub struct UserConsent {
    store: PathBuf,
    /// Why the store may not be used at all, decided when it was opened.
    refused: Option<String>,
}

impl UserConsent {
    /// The user's store for the project at `project`: `$SPECFORGE_CONSENT_FILE`, else [`UserConsent::of_home`]. A
    /// store named inside the project, or by a relative path, is refused: a repository could approve its own effects.
    pub fn user(project: &Path) -> Self {
        match std::env::var_os(CONSENT_FILE_ENV) {
            Some(path) => Self::at(PathBuf::from(path)).within(project),
            None => Self::of_home(),
        }
    }

    /// `$HOME/.specforge/consent.json` (`USERPROFILE`). With no home there is no store: nothing is approved and W186
    /// says why. Never a shared directory, where another user could create the store first and fill it.
    pub fn of_home() -> Self {
        Self::under(std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")))
    }

    fn under(home: Option<OsString>) -> Self {
        match home.map(PathBuf::from).filter(|home| home.is_absolute()) {
            Some(home) => Self::at(home.join(".specforge").join("consent.json")),
            None => UserConsent {
                store: PathBuf::from("~/.specforge/consent.json"),
                refused: Some(format!(
                    "there is no home directory (HOME); set {CONSENT_FILE_ENV} to a file outside the project"
                )),
            },
        }
    }

    /// The store at `store`.
    pub fn at(store: PathBuf) -> Self {
        UserConsent {
            store,
            refused: None,
        }
    }

    /// This store, refused when it is named by a relative path or resolves under the canonical `project`.
    pub fn within(mut self, project: &Path) -> Self {
        if self.store.is_relative() {
            self.refused = Some(format!(
                "a relative path could lie inside any project; point {CONSENT_FILE_ENV} at an absolute path"
            ));
        } else if resolved(&self.store).starts_with(resolved(project)) {
            self.refused = Some(format!(
                "it is inside the project, where a repository could approve its own effects; point \
                 {CONSENT_FILE_ENV} outside the project"
            ));
        }
        self
    }

    /// The store's file.
    pub fn path(&self) -> &Path {
        &self.store
    }

    fn error(&self, why: impl Into<String>) -> ConsentError {
        ConsentError {
            store: self.store.display().to_string(),
            why: why.into(),
        }
    }

    fn read(&self) -> Result<Approvals, ConsentError> {
        if let Some(why) = &self.refused {
            return Err(self.error(why.clone()));
        }
        let raw = match std::fs::read_to_string(&self.store) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Approvals::default()),
            Err(e) => return Err(self.error(format!("it cannot be read ({e})"))),
        };
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| self.error(format!("it is not JSON ({e}); fix it or move it away")))?;
        match value.get("version").and_then(serde_json::Value::as_u64) {
            Some(v) if v > VERSION => {
                return Err(self.error(format!(
                    "a newer specforge wrote it (store version {v}); update specforge"
                )));
            }
            Some(_) => {}
            None => return Err(self.error("it names no store version; fix it or move it away")),
        }
        serde_json::from_value::<Stored>(value)
            .map(|stored| stored.approvals)
            .map_err(|e| {
                self.error(format!(
                    "it is not a consent store ({e}); fix it or move it away"
                ))
            })
    }
}

/// `path` as the disk resolves it: its nearest existing ancestor canonical (every link and spelling followed), then
/// the rest lexically, since a name that does not exist yet is no link.
fn resolved(path: &Path) -> PathBuf {
    let Some((real, ancestor)) = path
        .ancestors()
        .find_map(|ancestor| Some((ancestor.canonicalize().ok()?, ancestor)))
    else {
        return path.to_path_buf();
    };
    let mut resolved = real;
    for part in path
        .strip_prefix(ancestor)
        .unwrap_or(Path::new(""))
        .components()
    {
        match part {
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(name) => resolved.push(name),
            _ => {}
        }
    }
    resolved
}

/// W186: the store `error` names cannot be used; nothing in it is approved and nothing is written over it.
pub fn unusable_store(error: &ConsentError) -> Diagnostic {
    Diagnostic::new(
        codes::W186,
        format!(
            "consent store {} is not usable: {}; nothing in it is approved here",
            error.store, error.why
        ),
    )
}

impl Consent for UserConsent {
    fn store(&self) -> String {
        self.store.display().to_string()
    }

    fn list(&self) -> Result<Approvals, ConsentError> {
        self.read()
    }

    fn update(&self, change: &mut dyn FnMut(&mut Approvals)) -> Result<(), ConsentError> {
        if let Some(why) = &self.refused {
            return Err(self.error(why.clone()));
        }
        private_parent(&self.store)
            .map_err(|e| self.error(format!("its directory cannot be made ({e})")))?;
        let mut lock_path = self.store.as_os_str().to_owned();
        lock_path.push(".lock");
        let lock = open_private_lock(Path::new(&lock_path))
            .map_err(|e| self.error(format!("its lock cannot be opened ({e})")))?;
        lock.lock()
            .map_err(|e| self.error(format!("it cannot be locked ({e})")))?;
        let mut approvals = self.read()?;
        change(&mut approvals);
        let json = serde_json::to_string_pretty(&Stored {
            version: VERSION,
            approvals,
        })
        .expect("approvals serialize");
        // The lock is released when `lock` drops, after the rename.
        replace_private(&self.store, json.as_bytes())
            .map_err(|e| self.error(format!("it cannot be written ({e})")))
    }
}

#[cfg(test)]
mod tests;
