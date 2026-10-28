//! The plan-shape pass: the rules that read one plan. `gsd/W323` (a plan's name) first; the rest land with their
//! tickets.

use specforge_extension_sdk::prelude::*;

use super::plans::{name_of, of_kind, phase_of};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("plan_shape", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    names(input)
}

/// GSD's quick id: `YYMMDD-xxx`, six digits, a dash, three base-36 digits (`src/init.cts`, lowercase).
fn is_quick_id(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 10
        && bytes[..6].iter().all(u8::is_ascii_digit)
        && bytes[6] == b'-'
        && bytes[7..]
            .iter()
            .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase())
}

/// A phase plan is `<phase>-<NN>`: the phase's name, a dash, and two or more digits (`02.1-01` in phase `02.1`).
fn is_phase_plan_name(name: &str, phase: &str) -> bool {
    name.strip_prefix(phase)
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|number| number.len() >= 2 && number.bytes().all(|b| b.is_ascii_digit()))
}

/// W323: a plan is named `<phase>-<NN>` in its phase, and by its quick id without one. A forgotten `phase` would
/// otherwise silently make a quick plan (the trap `gsd/E206` closes for a research).
fn names(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "plan")
        .filter_map(|plan| {
            let name = name_of(&plan.id);
            match phase_of(input, &plan.id) {
                Some(phase) if !is_phase_plan_name(name, name_of(phase)) => Some(
                    PassDiagnostic::new(
                        "W323",
                        PassSeverity::Warning,
                        format!(
                            "plan '{}' is in phase '{phase}' but is not named {}-NN",
                            plan.id,
                            name_of(phase)
                        ),
                    )
                    .with_suggestion(format!(
                        "name it {}-NN (two or more digits), or put it in the phase its name says",
                        name_of(phase)
                    )),
                ),
                Some(_) => None,
                None if plan.fields.contains_key("phase") || is_quick_id(name) => None,
                None => Some(
                    PassDiagnostic::new(
                        "W323",
                        PassSeverity::Warning,
                        format!(
                            "plan '{}' has no phase and is not named by a quick id (YYMMDD-xxx)",
                            plan.id
                        ),
                    )
                    .with_suggestion("add its phase, or name it by its quick id"),
                ),
            }
            .map(|d| d.with_entity(&plan.id))
        })
        .collect()
}

#[cfg(test)]
mod tests;
