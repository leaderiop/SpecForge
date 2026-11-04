//! What one run of the values phase may spend (ADR 0336 D4): what is over it is not run, and counts toward I206 as
//! `budget`, so nothing goes red for it and a later run, answered from the journal, gets further.

use std::time::Duration;

use specforge_protocol_types::OutcomeData;

/// What the scope of a not-performed effect is when the budget stopped it (I206).
pub(super) const OVER: &str = "budget";

/// What one run of the values phase may spend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueBudget {
    /// The most probes one call of a checker may ask; a value that asks more is not judged.
    pub probes_per_value: usize,
    /// How long the run's probes may take: none starts past it, and one started runs to its own timeout.
    pub time: Duration,
    /// The most an answer's body, read or listing may add up to, over the whole run.
    pub bytes: u64,
    /// The most distinct file and git probes it reads.
    pub local_probes: usize,
}

impl ValueBudget {
    /// A run a person or CI asked for.
    pub const RUN: ValueBudget = ValueBudget {
        probes_per_value: 16,
        time: Duration::from_secs(120),
        bytes: 64 * 1024 * 1024,
        local_probes: 50_000,
    };

    /// An editor's: its file probes run on every check, between two keystrokes.
    pub const EDITOR: ValueBudget = ValueBudget {
        probes_per_value: 16,
        time: Duration::from_secs(2),
        bytes: 8 * 1024 * 1024,
        local_probes: 2_000,
    };
}

impl Default for ValueBudget {
    fn default() -> Self {
        Self::RUN
    }
}

/// What `data` costs the run's byte budget.
pub(super) fn cost(data: &OutcomeData) -> u64 {
    match data {
        OutcomeData::Bytes { bytes, .. } => bytes.len() as u64,
        OutcomeData::Http { body, .. } => body.len() as u64,
        OutcomeData::Walk { entries, .. } => entries.iter().map(|e| e.len() as u64).sum(),
        _ => 0,
    }
}
