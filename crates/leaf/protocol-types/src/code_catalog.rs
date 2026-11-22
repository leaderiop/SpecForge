//! The `codes` describe category (protocol 1.7.0): an extension's catalog of
//! the diagnostic codes it reports (ADR 0286).

use serde::{Deserialize, Serialize};

/// One diagnostic code an extension reports: its local code (`E101`, never
/// qualified; the host adds the namespace), a title, what it means and how
/// to fix it, an optional absolute `http(s)` documentation URL, whether this machine decides it (2.0.0), the owner
/// state its findings wait for (2.2.0), and whether a later step of the authoring settles its findings (2.3.0).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeDescriptor {
    pub code: String,
    pub title: String,
    pub explanation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    /// A finding of this code is decided by the machine that runs the check (its approvals, its network or its
    /// effects journal), not by the project: `--strict` never promotes it, and the gate's warning baseline never
    /// counts it (protocol 2.0.0, ADR 0337 D7). Serialized only when set.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub machine: bool,
    /// The owner state (ADR 0329 D4) a finding of this code speaks in (2.2.0, ADR 0363): `complete` or `pending`, an
    /// [`OwnerState`](crate::OwnerState) word kept a string, so an unknown one costs the entry (E030). The host keeps
    /// a finding of it only on an entity in that state, read on the snapshot the compute passes' values are merged
    /// into, and a write that moves the entity into that state reveals the finding, which is no error the write adds
    /// (E080). Absent: in either state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_while: Option<String>,
    /// A finding of this code names a state a later step of the authoring settles (2.3.0, ADR 0385): a typed edit
    /// whose plan creates the finding's subject, or an entity under it, and alters nothing held there adds no error by
    /// it (E080), so a workflow may create the entity before the step that settles it. Its severity does not change:
    /// `check`, `--strict` and every gate read it. Serialized only when set.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub settled_later: bool,
}
