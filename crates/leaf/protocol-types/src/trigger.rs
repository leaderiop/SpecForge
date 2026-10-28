//! Why the host runs a guard pass (ADR 0291, protocol 1.10.0): an edit plan it is applying, or a gate it is evaluating.

use serde::{Deserialize, Serialize};

/// What made the host run a pass. A pass input without one reads [`TriggerKind::Write`], so a guest written against
/// 1.9.0 is asked as it always was.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger {
    pub kind: TriggerKind,
    /// A gate only: the command (its id) or prompt whose gate is evaluated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// A gate only: the full path of the entity the gate is scoped to, when it has a subject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

/// The two reasons a guard pass runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerKind {
    /// The host is applying an edit plan (protocol 1.9.0).
    #[default]
    Write,
    /// The host is evaluating the gate of a command or prompt; nothing is written.
    Gate,
}

impl Trigger {
    /// A gate of `command`, scoped to `subject`.
    pub fn gate(command: impl Into<String>, subject: Option<String>) -> Self {
        Trigger {
            kind: TriggerKind::Gate,
            command: Some(command.into()),
            subject,
        }
    }
}
