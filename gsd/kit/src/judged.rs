//! Judged rules as data (ADR 0377 D5): an extension keeps one table of its judged rules, and `declare` turns it into
//! SDK builder calls, so a `@gsd` capability extension declares its own rules the way `@gsd/core` does. Each rule
//! names the agent or prompt that judges it, the outcomes a verdict may record, and what it reads of each target.
//! [`record`] builds the command that writes the table's verdicts.

use specforge_extension_sdk::prelude::*;

pub mod record;

/// When a target owes a verdict (ADR 0362): once its owner is complete, or while it is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owed {
    /// Judged once its owner is complete (`once_complete`).
    Complete,
    /// Judged while its owner is open (`while_open`).
    Open,
}

/// What a rule reads of the entities of one kind.
pub struct Target {
    pub kind: &'static str,
    /// `(name, selector)`, the named inputs of the verdict.
    pub inputs: &'static [(&'static str, &'static str)],
    /// An entity that writes any of these fields is not judged.
    pub unless_set: &'static [&'static str],
    /// The owner state in which an entity owes a verdict; none, in either.
    pub judged_while: Option<Owed>,
}

/// One judged rule.
pub struct Judged {
    /// The local code: `W311`.
    pub code: &'static str,
    /// The diagnostic's message; the host adds `: no verdict`, `: stale …` or the verdict's outcome.
    pub message: &'static str,
    /// An agent of this extension, or `prompt:<id>`.
    pub judge: &'static str,
    pub outcomes: &'static [&'static str],
    pub passing: &'static [&'static str],
    pub targets: &'static [Target],
    /// `kind.field`s set from the verdict's `values`.
    pub values: &'static [&'static str],
    /// `kind.field`s that hold the verdict's outcome.
    pub outcome_fields: &'static [&'static str],
}

/// The judged rule of `table` with this local code.
pub fn get(table: &'static [Judged], code: &str) -> Option<&'static Judged> {
    table.iter().find(|j| j.code == code)
}

/// Declares every rule of `table`, each with the severity `severity` gives its local code.
pub fn declare(
    c: &mut ContributionsBuilder,
    table: &'static [Judged],
    severity: fn(&str) -> ValidationSeverity,
) {
    for j in table {
        c.rule(j.code, |r| {
            r.severity(severity(j.code))
                .message_template(j.message)
                .judged(j.judge, |b| {
                    b.outcomes(j.outcomes).passing(j.passing);
                    for t in j.targets {
                        b.target(t.kind, |tb| {
                            for (name, selector) in t.inputs {
                                tb.input(name, selector);
                            }
                            if !t.unless_set.is_empty() {
                                tb.unless_set(t.unless_set);
                            }
                            match t.judged_while {
                                Some(Owed::Complete) => {
                                    tb.once_complete();
                                }
                                Some(Owed::Open) => {
                                    tb.while_open();
                                }
                                None => {}
                            }
                        });
                    }
                    if !j.values.is_empty() {
                        b.values(j.values);
                    }
                    if !j.outcome_fields.is_empty() {
                        b.outcome_fields(j.outcome_fields);
                    }
                });
        });
    }
}

#[cfg(test)]
mod tests;
