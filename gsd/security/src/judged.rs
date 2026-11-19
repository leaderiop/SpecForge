//! The judged rules of `@gsd/security` (ADR 0377): one table, read by `declare` and by `record_security_verdict`, in
//! the shared form of `gsd_kit::judged`. Each is judged by the prompt `secure_phase`, whose orchestrator is the single
//! writer: the auditor returns its findings and never records (`spec/README.md`, "Judged rules' inputs").

use gsd_kit::judged::{Judged, Owed, Target};
use specforge_extension_sdk::prelude::*;

/// A closed threat claims its disposition holds: `mitigate`, the mitigation is in its files; `accept`, an accepted
/// risk records it; `transfer`, the transfer is documented. So the verdict reads the threat, its plan, its own files,
/// the risks that accept it and the VALIDATION.md rows that secure it, and is owed once the threat is closed.
const W302: &[Target] = &[Target {
    kind: "threat",
    inputs: &[
        ("threat", "self"),
        ("plan", "self.plan"),
        ("files", "file(self.files)"),
        ("accepted", "self<accepted_risk.threat"),
        ("rows", "self<validation_row.threat"),
    ],
    unless_set: &[],
    judged_while: Some(Owed::Complete),
}];

pub static JUDGED: &[Judged] = &[Judged {
    code: "W302",
    message: "threat '{id}' has no fresh mitigation verdict",
    judge: "prompt:secure_phase",
    outcomes: &["holds", "broken"],
    passing: &["holds"],
    targets: W302,
    values: &[],
    outcome_fields: &[],
}];

/// A model's verdict on a mitigation is not authoritative: every judged rule of the extension is a warning, as W301 is.
fn severity(_code: &str) -> ValidationSeverity {
    ValidationSeverity::Warning
}

pub fn declare(c: &mut ContributionsBuilder) {
    gsd_kit::judged::declare(c, JUDGED, severity);
}
