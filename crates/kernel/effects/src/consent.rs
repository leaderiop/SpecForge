//! The machine's approvals (ADR 0335 D2.4): the `Consent` port and what it keeps. An approval is keyed by the
//! project, the extension and its provenance (a builtin by name, a third-party extension by its blob's digest, O6)
//! and the scope's name, and stores the granted scope whole: it covers a narrower grant, a wider one asks again. A
//! named root outside the project is approved per project by its canonical target (O5). The adapter, the user's
//! private store, is `specforge_ops::UserConsent`; the fake is `testing::MemoryConsent`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specforge_package::Sha256;
use specforge_protocol_types::{Operation, PortScope};

use crate::Provenance;
use crate::scope::rules;

/// What an approval is keyed by of a provenance: a builtin's name alone, a third-party blob's digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyedBy {
    Builtin,
    Digest(String),
}

impl KeyedBy {
    pub fn of(provenance: &Provenance) -> Self {
        match provenance {
            Provenance::Builtin { .. } => KeyedBy::Builtin,
            Provenance::ThirdParty { digest } => KeyedBy::Digest(digest.clone()),
        }
    }
}

/// A project as the store keys it: its canonical root ([`crate::grant::project_key`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectKey(pub String);

/// When this machine stored an approval (RFC 3339); audit, never matched. Unset until it is stored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApprovedAt(pub String);

impl std::ops::Deref for ProjectKey {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

/// One capability approved on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityApproval {
    pub project: ProjectKey,
    pub extension: String,
    pub provenance: KeyedBy,
    /// The capability's scope name.
    pub scope: String,
    /// The scope as the ceiling left it when approved.
    pub granted: PortScope,
    /// The call kinds it was approved for; empty: every kind the gate allows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub call_kinds: Vec<Operation>,
    /// The environment variable it sends as a header (ADR 0335 D6): another variable asks again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_env: Option<String>,
    /// Audit, never matched: the blob's digest, the specforge that stored it and when.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Sha256>,
    #[serde(default)]
    pub specforge: String,
    #[serde(default)]
    pub approved_at: ApprovedAt,
}

impl CapabilityApproval {
    /// Whether this stored approval allows `wanted`: the same project, extension, provenance and scope name, every
    /// call kind it asks, and a granted scope that covers it.
    pub fn covers(&self, wanted: &CapabilityApproval) -> bool {
        self.same_key(wanted)
            && (self.call_kinds.is_empty()
                || (!wanted.call_kinds.is_empty()
                    && wanted
                        .call_kinds
                        .iter()
                        .all(|k| self.call_kinds.contains(k))))
            && self.secret_env == wanted.secret_env
            && rules(&self.granted).covers(&wanted.granted)
    }

    /// What it approved, in the line a person approves ([`approval_line`]).
    pub fn summary(&self) -> String {
        approval_line(&self.granted, self.secret_env.as_deref())
    }

    /// The same project, extension, provenance and scope name: a new approval replaces it.
    pub fn same_key(&self, other: &CapabilityApproval) -> bool {
        self.project == other.project
            && self.extension == other.extension
            && self.provenance == other.provenance
            && self.scope == other.scope
    }
}

/// The line a person approves: `scope` in one line, and the variable it sends as a header, by name.
pub fn approval_line(scope: &PortScope, secret_env: Option<&str>) -> String {
    let header = match scope {
        PortScope::Http(http) => http.secret_header.as_ref().map(|h| h.header.as_str()),
        _ => None,
    };
    match (secret_env, header) {
        (Some(env), Some(header)) => {
            format!(
                "{}, sends `${env}` as the `{header}` header",
                rules(scope).summary()
            )
        }
        _ => rules(scope).summary(),
    }
}

/// One named root outside a project, approved on this machine by its canonical target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootApproval {
    pub project: ProjectKey,
    pub root: String,
    pub target: String,
    #[serde(default)]
    pub approved_at: ApprovedAt,
}

/// Every approval a store holds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approvals {
    #[serde(default)]
    pub capabilities: Vec<CapabilityApproval>,
    #[serde(default)]
    pub roots: Vec<RootApproval>,
}

/// One approval, to store or to look for.
#[expect(clippy::large_enum_variant, reason = "a handful per run, built once")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Approval {
    Capability(CapabilityApproval),
    Root(RootApproval),
}

impl Approvals {
    /// Whether a stored approval allows `wanted`.
    pub fn covers(&self, wanted: &Approval) -> bool {
        match wanted {
            Approval::Capability(wanted) => self.capabilities.iter().any(|a| a.covers(wanted)),
            Approval::Root(wanted) => self.roots.iter().any(|a| {
                a.project == wanted.project && a.root == wanted.root && a.target == wanted.target
            }),
        }
    }

    /// Store `approval`, replacing one of the same key.
    pub fn add(&mut self, approval: Approval) {
        match approval {
            Approval::Capability(approval) => {
                self.capabilities.retain(|a| !a.same_key(&approval));
                self.capabilities.push(approval);
            }
            Approval::Root(approval) => {
                self.roots
                    .retain(|a| !(a.project == approval.project && a.root == approval.root));
                self.roots.push(approval);
            }
        }
    }

    /// Drop what `which` names; how many went.
    pub fn remove(&mut self, which: &Revoke) -> usize {
        let before = self.capabilities.len() + self.roots.len();
        match which {
            Revoke::Scopes {
                project,
                extension,
                scopes,
            } => self.capabilities.retain(|a| {
                !(&a.project == project
                    && &a.extension == extension
                    && (scopes.is_empty() || scopes.contains(&a.scope)))
            }),
            Revoke::Root { project, root } => self
                .roots
                .retain(|a| !(&a.project == project && &a.root == root)),
            Revoke::Project { project } => {
                self.capabilities.retain(|a| &a.project != project);
                self.roots.retain(|a| &a.project != project);
            }
        }
        before - self.capabilities.len() - self.roots.len()
    }
}

/// What a revoke drops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revoke {
    /// `extension`'s approvals in `project`, of `scopes` (every one when empty).
    Scopes {
        project: ProjectKey,
        extension: String,
        scopes: Vec<String>,
    },
    /// The approval of `project`'s named root `root`.
    Root { project: ProjectKey, root: String },
    /// Every approval of `project`.
    Project { project: ProjectKey },
}

/// Why a store cannot be used (W186): nothing in it is approved, and nothing is written over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentError {
    /// The store, as a person finds it.
    pub store: String,
    pub why: String,
}

impl std::fmt::Display for ConsentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.store, self.why)
    }
}

impl std::error::Error for ConsentError {}

/// The machine's approvals.
pub trait Consent: Send + Sync {
    /// Where it keeps them, as a person finds it.
    fn store(&self) -> String;

    /// Every approval it holds.
    fn list(&self) -> Result<Approvals, ConsentError>;

    /// Change what it holds with `change`, read and written as one step: no other change is lost.
    fn update(&self, change: &mut dyn FnMut(&mut Approvals)) -> Result<(), ConsentError>;

    /// Whether a held approval allows `wanted`.
    fn approved(&self, wanted: &Approval) -> Result<bool, ConsentError> {
        Ok(self.list()?.covers(wanted))
    }

    /// Hold `approval`, replacing one of the same key.
    fn approve(&self, approval: Approval) -> Result<(), ConsentError> {
        let mut approval = Some(approval);
        self.update(&mut |held| {
            if let Some(approval) = approval.take() {
                held.add(approval);
            }
        })
    }

    /// Drop what `which` names; how many went.
    fn revoke(&self, which: &Revoke) -> Result<usize, ConsentError> {
        let mut gone = 0;
        self.update(&mut |held| gone = held.remove(which))?;
        Ok(gone)
    }
}

/// A consent store as a surface shares it.
pub type SharedConsent = Arc<dyn Consent>;

/// A store that holds nothing and keeps nothing: for what a grant's declarations and ceiling decide, whatever this
/// machine approved.
pub struct NoConsent;

impl Consent for NoConsent {
    fn store(&self) -> String {
        "no store".to_string()
    }

    fn list(&self) -> Result<Approvals, ConsentError> {
        Ok(Approvals::default())
    }

    fn update(&self, _change: &mut dyn FnMut(&mut Approvals)) -> Result<(), ConsentError> {
        Err(ConsentError {
            store: self.store(),
            why: "it keeps nothing".to_string(),
        })
    }
}
