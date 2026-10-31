//! What a command's input says the recorded tests prove.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// What a command's input says the recorded tests prove. On the wire an
/// object tagged by `state`: `{"state":"recorded","entities":{...}}`,
/// `{"state":"unreadable","reason":"..."}`; `none` is the field's absence.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CommandEvidence {
    /// No test report is recorded (or the host is older than the field).
    #[default]
    None,
    /// The recorded report, scored: one entry per entity that counts toward
    /// coverage (ADR 0004 D2-b), keyed by id.
    Recorded {
        entities: BTreeMap<String, EntityEvidence>,
    },
    /// A report is recorded and could not be read; `reason` says why.
    Unreadable { reason: String },
}

impl CommandEvidence {
    /// Whether the input carries no evidence state at all.
    pub fn is_none(&self) -> bool {
        matches!(self, CommandEvidence::None)
    }

    /// The scored entities, when a report was recorded and read.
    pub fn entities(&self) -> Option<&BTreeMap<String, EntityEvidence>> {
        match self {
            CommandEvidence::Recorded { entities } => Some(entities),
            _ => None,
        }
    }
}

/// One entity's proof, as the coverage rule scores it: the obligations it
/// declares, how many a passing test names, and how many of its recorded
/// tests fail.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityEvidence {
    pub obligations: usize,
    pub proven: usize,
    #[serde(default)]
    pub failing: usize,
}

impl EntityEvidence {
    /// The coverage rule's "proven" (ADR 0004 D2-a): at least one
    /// obligation, every one proven, no failing test.
    pub fn is_proven(&self) -> bool {
        self.obligations > 0 && self.proven == self.obligations && self.failing == 0
    }
}
