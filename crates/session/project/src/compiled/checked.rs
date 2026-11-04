//! The outcome class of each value the checks checked (ADR 0337 §6, amending ADR 0290): what a judged verdict
//! digests of an entity besides its authored values, as the values phase gave them.

use specforge_effects::{OutcomeClass, Redirected, Unreachable};
use std::collections::BTreeMap;

use super::CompiledProject;

/// What checking one value gave: its outcome class, or `Unknown` when this machine did not perform the check (I206).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueOutcome {
    Class(OutcomeClass),
    Unknown,
}

impl ValueOutcome {
    /// The bytes a verdict digests for this outcome (ADR 0337 §6), spelled here and never through `Display`, so a
    /// change to how an outcome class is shown never stales a recorded verdict.
    pub fn tag(&self) -> String {
        let class = match self {
            ValueOutcome::Unknown => return "unknown".to_owned(),
            ValueOutcome::Class(class) => class,
        };
        let plain = match class {
            OutcomeClass::Ok => "ok",
            OutcomeClass::Status(code) => return format!("status:{code}"),
            OutcomeClass::Exit(code) => return format!("exit:{code}"),
            OutcomeClass::Signal => "signal",
            OutcomeClass::Deadline => "deadline",
            OutcomeClass::Capped => "capped",
            OutcomeClass::Missing => "missing",
            OutcomeClass::Denied => "denied",
            OutcomeClass::Outside => "outside",
            OutcomeClass::NotFound => "not_found",
            OutcomeClass::CannotStart => "cannot_start",
            OutcomeClass::RangeIgnored => "range:ignored",
            OutcomeClass::Unreachable(why) => {
                let why = match why {
                    Unreachable::Dns => "dns",
                    Unreachable::Connect => "connect",
                    Unreachable::Tls => "tls",
                    Unreachable::Timeout => "timeout",
                    Unreachable::Reset => "reset",
                    Unreachable::Proxy => "proxy",
                    Unreachable::RefusedAddress => "refused_address",
                };
                return format!("unreachable:{why}");
            }
            OutcomeClass::Redirect(why) => {
                let why = match why {
                    Redirected::Limit => "limit",
                    Redirected::Downgrade => "downgrade",
                    Redirected::Refused => "refused",
                };
                return format!("redirect:{why}");
            }
        };
        plain.to_owned()
    }
}

/// One checked value of an entity: the field, the item of a list (0 for a scalar), the checker, its outcome. When it
/// was observed is never part of it, so a recheck that agrees changes nothing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckedValue {
    pub field: String,
    pub item: usize,
    pub checker: String,
    pub outcome: ValueOutcome,
}

/// Every entity's checked values, by entity id, each entity's sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckedValues(BTreeMap<String, Vec<CheckedValue>>);

impl CheckedValues {
    /// Add `value` to `id`'s checked values.
    pub fn insert(&mut self, id: &str, value: CheckedValue) {
        let values = self.0.entry(id.to_string()).or_default();
        values.push(value);
        values.sort();
    }

    /// `id`'s checked values, sorted; empty when none was checked.
    pub fn of(&self, id: &str) -> &[CheckedValue] {
        self.0.get(id).map_or(&[], Vec::as_slice)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl CompiledProject {
    /// The outcome class of each value the last run of the checks checked, by entity.
    pub fn checked_values(&self) -> &CheckedValues {
        &self.checked_values
    }

    /// What the value checks found at the last run of the checks (ADR 0336), by entity.
    pub fn value_checks(&self) -> &crate::values::ValueChecks {
        &self.value_checks
    }

    /// This project as its checks run with `values` as the values phase's (tests: a project with no checker keeps them).
    #[cfg(feature = "testing")]
    pub fn with_checked_values(mut self, values: CheckedValues) -> Self {
        self.checked_values = std::sync::Arc::new(values);
        self.check(&mut crate::ledger::Unrecorded::plain());
        self
    }
}
