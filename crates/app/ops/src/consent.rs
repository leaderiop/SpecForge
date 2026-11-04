//! The user's consent store (ADR 0335 D2.5, ADR 0193): every approval this machine gives, for every project, in
//! `~/.specforge/consent.json`. It lives outside every project, so a cloned repository cannot approve its own
//! effects. Each change is read, changed and written under an exclusive lock on the sidecar `consent.json.lock`,
//! and written privately and atomically (`replace_private`: 0600, a new parent 0700, a rename). A store that cannot
//! be read or parsed, or that a newer specforge wrote, is never written over: nothing in it counts (W186).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specforge_common::fresh::replace_private;
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
    /// The user's store for the project at `project`: `$SPECFORGE_CONSENT_FILE`, else
    /// `$HOME/.specforge/consent.json` (`USERPROFILE`, then the temp dir). A store named inside the project is
    /// refused: a repository could approve its own effects.
    pub fn user(project: &Path) -> Self {
        match std::env::var_os(CONSENT_FILE_ENV) {
            Some(path) => Self::at(PathBuf::from(path)).within(project),
            None => Self::at(default_store()),
        }
    }

    /// The store at `store`.
    pub fn at(store: PathBuf) -> Self {
        UserConsent {
            store,
            refused: None,
        }
    }

    /// This store, refused when its canonical parent lies under the canonical `project`.
    pub fn within(mut self, project: &Path) -> Self {
        let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        let parent = self.store.parent().map(canonical).unwrap_or_default();
        if parent.starts_with(canonical(project)) {
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

/// `$HOME/.specforge/consent.json` (`USERPROFILE`, then the temp dir).
pub fn default_store() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(".specforge")
        .join("consent.json")
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
    fn list(&self) -> Result<Approvals, ConsentError> {
        self.read()
    }

    fn update(&self, change: &mut dyn FnMut(&mut Approvals)) -> Result<(), ConsentError> {
        if let Some(why) = &self.refused {
            return Err(self.error(why.clone()));
        }
        if let Some(parent) = self.store.parent() {
            let mut dirs = std::fs::DirBuilder::new();
            dirs.recursive(true);
            #[cfg(unix)]
            std::os::unix::fs::DirBuilderExt::mode(&mut dirs, 0o700);
            dirs.create(parent)
                .map_err(|e| self.error(format!("its directory cannot be made ({e})")))?;
        }
        let mut lock_path = self.store.as_os_str().to_owned();
        lock_path.push(".lock");
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(PathBuf::from(lock_path))
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
