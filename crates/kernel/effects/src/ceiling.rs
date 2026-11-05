//! The project's effects ceiling (ADR 0335 D2.3): what `specforge.json` `effects` leaves of each port. It can only
//! narrow a declared capability, never widen one, and it is never an approval: the file is committed.

use specforge_protocol_types::{FsAccess, GitOp, HttpMethod, PortScope};

use crate::scope::rules;

/// What the ceiling leaves of one port.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Narrow<T> {
    /// Not narrowed.
    #[default]
    Open,
    /// The port is switched off.
    Off,
    /// Only what it lists.
    Only(T),
}

/// `effects.http`: each list kept, when written, is all that is left.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpCeiling {
    pub hosts: Option<Vec<String>>,
    pub schemes: Option<Vec<String>>,
    pub methods: Option<Vec<HttpMethod>>,
}

/// `effects.fs`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FsCeiling {
    pub roots: Option<Vec<String>>,
    pub ops: Option<Vec<FsAccess>>,
}

/// `effects.git`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitCeiling {
    pub ops: Option<Vec<GitOp>>,
}

/// The project's effects ceiling. The default narrows nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectsCeiling {
    pub http: Narrow<HttpCeiling>,
    pub fs: Narrow<FsCeiling>,
    pub git: Narrow<GitCeiling>,
    /// A process is all or nothing: `Only` is never written.
    pub process: Narrow<()>,
}

/// What the ceiling left of a declared scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Narrowed {
    /// `None`: nothing is left, the capability grants nothing.
    pub scope: Option<PortScope>,
    /// What it removed, each with the `effects` key that removed it (`effects.http.hosts`).
    pub removed: Vec<Removed>,
}

/// One thing the ceiling removed from a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    /// The ceiling's key: `effects.process`, `effects.http.hosts`.
    pub key: String,
    /// What it took, for a person.
    pub what: String,
}

/// The roots a file scope that names none reaches.
pub const DEFAULT_ROOTS: [&str; 2] = ["spec", "project"];

impl EffectsCeiling {
    /// What it leaves of `scope`.
    pub fn narrow(&self, scope: &PortScope) -> Narrowed {
        let mut removed = Vec::new();
        let scope = rules(scope).narrowed(self, &mut removed);
        Narrowed { scope, removed }
    }
}

/// Whether host pattern `pattern` allows every host `other` (a host or a pattern) allows, in any case:
/// `*.example.com` covers `a.example.com` and `*.a.example.com`, not `example.com`.
pub fn host_covers(pattern: &str, other: &str) -> bool {
    let (pattern, other) = (pattern.to_ascii_lowercase(), other.to_ascii_lowercase());
    match pattern.strip_prefix("*.") {
        Some(suffix) => match other.strip_prefix("*.") {
            Some(under) => under == suffix || under.ends_with(&format!(".{suffix}")),
            None => other.ends_with(&format!(".{suffix}")),
        },
        None => pattern == other,
    }
}
