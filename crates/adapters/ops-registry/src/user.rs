//! The user's registry files: where they are, and what a registry call carries from them (ADR 0178).

use crate::{ConfiguredRegistry, TOKEN_VARIABLE, failure, unusable_credential};
use specforge_common::Diagnostic;
use specforge_common::codes;
use specforge_effects::SharedGit;
use specforge_effects::git::Located;
use specforge_ops::registry::{NOT_AUTHENTICATED, UNUSABLE_SIGNING_KEY};
use specforge_ops::{OpError, OpErrorKind};
use specforge_registry_client::CredentialStore;
pub use specforge_registry_client::credential_health::{
    CredentialHealth, CredentialLevel, CredentialLine,
};
use specforge_registry_client::credentials::CREDENTIALS_FILE;
use specforge_registry_client::credentials::TokenReference;
use specforge_registry_client::signing::load_or_create_signing_key_at;
use specforge_registry_client::{RegistryConfig, RegistryCredential, SigningKey};
pub use specforge_registry_wire::RegistryRoot;
use std::path::PathBuf;

/// The publisher key's file in the user's directory.
const SIGNING_KEY: &str = "signing-key.json";

/// What to do about a user file a repository tracks.
const MOVE_HOME: &str = "point HOME at a directory outside every repository, or untrack the file";

/// The user SpecForge runs as, as a registry sees them: the directory that
/// holds their registry files (`credentials.json`, `signing-key.json`,
/// `known-keys.json`) and the token their environment gives.
#[derive(Clone)]
pub struct User {
    dir: PathBuf,
    token: Option<String>,
    /// Asks whether the git worktree `dir` lies in tracks a file of it; none for files a caller names.
    git: Option<SharedGit>,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

impl User {
    /// `$HOME/.specforge` (`USERPROFILE` on Windows), and `SPECFORGE_REGISTRY_TOKEN` when it is set, each file
    /// refused when `git` says the worktree it lies in tracks it. The one place SpecForge reads `HOME` for registry
    /// files.
    pub fn current(git: SharedGit) -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        Self::at(home.join(".specforge"), std::env::var(TOKEN_VARIABLE).ok()).tracked_by(git)
    }

    /// Registry files in `dir`, and `token` as the environment's token (a
    /// test, a custom home).
    pub fn at(dir: impl Into<PathBuf>, token: Option<String>) -> Self {
        Self {
            dir: dir.into(),
            token,
            git: None,
        }
    }

    /// These files, each refused when `git` says the worktree it lies in tracks it.
    pub fn tracked_by(mut self, git: SharedGit) -> Self {
        self.git = Some(git);
        self
    }

    /// Why the user file `name` is refused: the git worktree it lies in holds it at `HEAD`, so it came with a
    /// repository (the home is a project, or lies in one) and records nothing of this user's. `None` when git
    /// says otherwise or cannot be asked.
    fn tracked(&self, name: &str) -> Option<String> {
        let git = self.git.as_ref()?;
        let path = self.dir.join(name);
        if !path.is_file() {
            return None;
        }
        let Located::Repo(place) = git.locate(&self.dir) else {
            return None;
        };
        let inside = path.canonicalize().ok()?;
        let inside = inside.strip_prefix(&place.toplevel).ok()?.to_path_buf();
        let head = git.commit(&place, "HEAD").ok()??;
        let sizes = git
            .sizes(&place, &head, std::slice::from_ref(&inside))
            .ok()?;
        sizes.first()?.1.map(|_| {
            format!(
                "{} is tracked by the git repository at {}: a repository's file is no record of this user's",
                path.display(),
                place.toplevel.display()
            )
        })
    }

    /// The credential store, read now (R012 when it can't be read, or the worktree it lies in tracks it).
    pub(crate) fn store(&self) -> Result<CredentialStore, Diagnostic> {
        if let Some(why) = self.tracked(CREDENTIALS_FILE) {
            return Err(Diagnostic::new(codes::R012, why).with_suggestion(MOVE_HOME));
        }
        CredentialStore::read(&self.dir)
    }

    pub(crate) fn signing_key(&self) -> PathBuf {
        self.dir.join(SIGNING_KEY)
    }

    /// The known-keys store's path, R-TRUST-006 when the worktree it lies in tracks it.
    pub(crate) fn known_keys(&self) -> Result<PathBuf, Diagnostic> {
        const KNOWN_KEYS: &str = "known-keys.json";
        match self.tracked(KNOWN_KEYS) {
            Some(why) => Err(Diagnostic::new(codes::R_TRUST_006, why).with_suggestion(MOVE_HOME)),
            None => Ok(self.dir.join(KNOWN_KEYS)),
        }
    }

    /// The environment's token unless it is blank.
    pub(crate) fn token(&self) -> Option<&str> {
        self.token.as_deref().filter(|t| !t.trim().is_empty())
    }
}

impl ConfiguredRegistry {
    /// The user's credential store, read once, on the first call that needs it (ADR 0045 as amended):
    /// never for a publish whose environment gives the token.
    pub(crate) fn store(&self) -> Result<&CredentialStore, Diagnostic> {
        self.store
            .get_or_init(|| self.user.store())
            .as_ref()
            .map_err(Clone::clone)
    }

    /// The credential the user keeps for `registry`, for a read: the stored one
    /// ([`specforge_registry_client::CredentialStore::credential`]), `None` when there is none. The
    /// environment's `SPECFORGE_REGISTRY_TOKEN` is not used: it is the token of the registry a publish
    /// goes to, and a read may ask every registry (search). A stored credential that can't be used
    /// refuses the read before any request.
    pub(crate) fn read_credential(
        &self,
        registry: &RegistryConfig,
    ) -> Result<Option<RegistryCredential>, OpError> {
        self.stored_credential(registry)
            .map_err(unusable_credential)
    }

    /// [`ConfiguredRegistry::read_credential`] as the diagnostic that refuses it.
    pub(crate) fn stored_credential(
        &self,
        registry: &RegistryConfig,
    ) -> Result<Option<RegistryCredential>, Diagnostic> {
        self.store()?.credential(&registry.url)
    }

    /// The credential a publish to `registry` authenticates with: the
    /// environment's token when set and not blank (the store is not read),
    /// else the one stored for the registry's root.
    pub(crate) fn credential_for(
        &self,
        registry: &RegistryConfig,
    ) -> Result<RegistryCredential, OpError> {
        if let Some(token) = self.user.token() {
            return Ok(RegistryCredential::new(registry.url.clone(), token));
        }
        self.stored_credential(registry)
            .map_err(unusable_credential)?
            .ok_or_else(|| {
                OpError::coded(
                    OpErrorKind::PreconditionFailed,
                    NOT_AUTHENTICATED,
                    format!("no credential for registry '{}'", registry.alias),
                )
                .with_suggestion(format!(
                    "log in with `{}`, or set {TOKEN_VARIABLE}",
                    specforge_common::remedy::command(
                        "specforge login --registry {} --token <TOKEN>",
                        &[&registry.alias]
                    )
                ))
            })
    }

    /// The user's publisher key, and whether it was created just now.
    pub(crate) fn signing_key(&self) -> Result<(SigningKey, bool), OpError> {
        let path = self.user.signing_key();
        if let Some(why) = self.user.tracked(SIGNING_KEY) {
            return Err(
                OpError::coded(OpErrorKind::PreconditionFailed, UNUSABLE_SIGNING_KEY, why)
                    .with_suggestion(MOVE_HOME),
            );
        }
        load_or_create_signing_key_at(&path).map_err(|why| {
            OpError::coded(OpErrorKind::PreconditionFailed, UNUSABLE_SIGNING_KEY, why)
                .with_suggestion(format!(
                    "move {} aside: the next publish creates a new key, and whoever pinned the old one then sees a changed key (R-TRUST-003)",
                    path.display()
                ))
        })
    }
}

/// Where the token to log in with comes from.
#[derive(Debug, Clone)]
pub enum TokenSource {
    /// `--token`: kept as a secret.
    Given(String),
    /// `--token-env`: the variable's name is kept.
    Env(String),
    /// `--token-file`: the file's path is kept.
    File(PathBuf),
}

/// How a login kept the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    /// A 0600 secret file.
    Secret,
    /// A reference to an environment variable.
    Env,
    /// A reference to a token file.
    File,
}

/// What a login kept, and for which registry.
#[derive(Debug, Clone)]
pub struct LoggedIn {
    pub alias: String,
    pub root: RegistryRoot,
    pub kept: Kept,
}

/// What a logout forgot.
#[derive(Debug, Clone)]
pub struct LoggedOut {
    /// The alias the registry was named by, when one was.
    pub alias: Option<String>,
    pub root: RegistryRoot,
    /// Whether an entry was kept for `root`.
    pub removed: bool,
}

impl User {
    /// Forget what is kept for `root`: the entry first (the store written), then its secret, so a store
    /// that can't be written keeps its secret. R012 before anything is removed.
    pub fn log_out(&self, root: &RegistryRoot) -> Result<LoggedOut, OpError> {
        let mut store = self.store().map_err(unusable_credential)?;
        let removed = store.forget(root);
        if removed {
            store
                .write()
                .map_err(|diagnostic| OpError::of(OpErrorKind::Internal, diagnostic))?; // R013
        }
        store.forget_secret(root);
        Ok(LoggedOut {
            alias: None,
            root: root.clone(),
            removed,
        })
    }

    /// What doctor reports about these files.
    pub fn credential_health(&self) -> CredentialHealth {
        let mut health = specforge_registry_client::credential_health::credential_health_now(
            &self.dir,
            &self.signing_key(),
        );
        if let Some(why) = self.tracked(CREDENTIALS_FILE) {
            // Every reader refuses it (R012): one unusable credential, whatever it holds.
            health.lines = vec![CredentialLine {
                level: CredentialLevel::Error,
                text: format!("{why} — {MOVE_HOME}"),
            }];
            health.failures = 1;
        }
        health
    }
}

impl ConfiguredRegistry {
    /// Validate `source`'s token against the registry `alias` names (else the default one) and keep it for
    /// that registry's root. Refuses, each before anything after it: E063/E067/E069 (configuration), E063
    /// (alias), R010/R011 (reference), R012 (store; left as it is), then the registry's answer (R001, R002,
    /// R009, R004/R005), then R-LOGIN-002/R013 (writing). Nothing is written unless the registry accepted
    /// the token.
    pub fn log_in(&self, alias: Option<&str>, source: TokenSource) -> Result<LoggedIn, OpError> {
        let registry = self.registries()?.named(alias)?.clone();
        let root = registry.url.clone();
        // Resolve the token before any request: a reference that can't be read is R010/R011.
        let referenced = |reference: TokenReference, kept: Kept| -> Result<_, OpError> {
            let mut probe = CredentialStore::empty(&self.user.dir);
            probe.keep_reference(root.clone(), reference.clone());
            match probe.credential(&root).map_err(unusable_credential)? {
                Some(credential) => Ok((credential, Some(reference), kept)),
                None => unreachable!("a reference was just kept"),
            }
        };
        let (credential, reference, kept) = match source {
            TokenSource::Given(token) => (
                RegistryCredential::new(root.clone(), token),
                None,
                Kept::Secret,
            ),
            TokenSource::Env(var) => referenced(TokenReference::Env(var), Kept::Env)?,
            TokenSource::File(path) => referenced(TokenReference::File(path), Kept::File)?,
        };

        // An unreadable store is refused before any request, as publish refuses it (ADR 0045).
        let mut store = self.user.store().map_err(unusable_credential)?;
        let expires_at = self
            .client
            .authenticate(&registry, &credential)
            .map_err(failure)?;

        match reference {
            Some(reference) => store.keep_reference(root.clone(), reference),
            None => store
                .keep_secret(root.clone(), credential.token().to_string(), expires_at)
                .map_err(|message| {
                    OpError::coded(OpErrorKind::PreconditionFailed, codes::R_LOGIN_002, message)
                })?,
        }
        store
            .write()
            .map_err(|diagnostic| OpError::of(OpErrorKind::Internal, diagnostic))?; // R013
        Ok(LoggedIn {
            alias: registry.alias,
            root,
            kept,
        })
    }

    /// [`User::log_out`] of the root the registry `alias` names (else the default one) has in the
    /// project's configuration.
    pub fn log_out(&self, alias: Option<&str>) -> Result<LoggedOut, OpError> {
        let registry = self.registries()?.named(alias)?;
        Ok(LoggedOut {
            alias: Some(registry.alias.clone()),
            ..self.user.log_out(&registry.url)?
        })
    }
}
