//! What every plan pass reads the same way: the plans, and the phase each one is in.

use specforge_extension_sdk::prelude::*;

pub fn of_kind<'a>(input: &'a PassInput, kind: &'a str) -> impl Iterator<Item = &'a PassEntity> {
    input.entities.iter().filter(move |e| e.kind == kind)
}

/// The id of the phase a plan is in, through its resolved `phase` edge; `None` for a quick plan (and for a `phase`
/// that names nothing, which the host reports).
pub fn phase_of<'a>(input: &'a PassInput, plan: &str) -> Option<&'a str> {
    input
        .edges
        .iter()
        .find(|e| e.source == plan && e.label == "phase")
        .map(|e| e.target.as_str())
}

/// The last `/`-segment of an id: its name.
pub fn name_of(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}
