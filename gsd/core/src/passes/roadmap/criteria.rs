//! W103: a phase has 2 to 5 success criteria (the roadmapper's "2-5 observable behaviors"). It counts the list's
//! items, which the kernel hands a pass as `values` (K13): the joined field text cannot tell an item that holds a
//! comma from two.

use specforge_extension_sdk::prelude::*;

use super::of_kind;

const LEAST: usize = 2;
const MOST: usize = 5;

pub fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "phase")
        .filter_map(|phase| {
            // A phase that writes none is the host's required-field error, not W103.
            let count = phase.values.get("success_criteria")?.as_array()?.len();
            if (LEAST..=MOST).contains(&count) {
                return None;
            }
            Some(
                PassDiagnostic::new(
                    "W103",
                    PassSeverity::Warning,
                    format!(
                        "phase '{}' has {count} success criteria; a phase has {LEAST} to {MOST}",
                        phase.id
                    ),
                )
                .with_suggestion(if count < LEAST {
                    "add the observable behaviours that show the phase is done"
                } else {
                    "merge criteria, or split the phase"
                })
                .with_entity(&phase.id),
            )
        })
        .collect()
}
