//! The `codes` describe category (protocol 1.7.0): an extension's catalog of
//! the diagnostic codes it reports (ADR 0286).

use serde::{Deserialize, Serialize};

/// One diagnostic code an extension reports: its local code (`E101`, never
/// qualified; the host adds the namespace), a title, what it means and how
/// to fix it, and an optional absolute `http(s)` documentation URL.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeDescriptor {
    pub code: String,
    pub title: String,
    pub explanation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
}
