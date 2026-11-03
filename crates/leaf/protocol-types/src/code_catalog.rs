//! The `codes` describe category (protocol 1.7.0): an extension's catalog of
//! the diagnostic codes it reports (ADR 0286).

use serde::{Deserialize, Serialize};

/// One diagnostic code an extension reports: its local code (`E101`, never
/// qualified; the host adds the namespace), a title, what it means and how
/// to fix it, an optional absolute `http(s)` documentation URL, and whether this machine decides it (2.0.0).
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
}
