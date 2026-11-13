//! The computed status of a lesson (`spec/project.md`): what its graduation says. A lesson with none is `open`; a
//! deferral is `deferred` until the phase it names is complete, then `open` again, so the next graduation sees it.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use super::verify::Verified;
use crate::passes::plans::of_kind;

/// A lesson's status from its graduation's `disposition` and whether the deferral's phase is complete.
pub fn status(disposition: Option<&str>, until_complete: bool) -> &'static str {
    match disposition {
        Some("promoted") => "promoted",
        Some("dismissed") => "dismissed",
        Some("deferred") if !until_complete => "deferred",
        _ => "open",
    }
}

pub fn values(input: &PassInput, verified: &BTreeMap<String, Verified>) -> Vec<PassValue> {
    of_kind(input, "lesson")
        .map(|lesson| {
            let graduated = lesson.values.get("graduated");
            let member = |name: &str| graduated.and_then(|g| g.get(name)).and_then(|v| v.as_str());
            // a phase is complete once its verification passed (the phase ladder's first rung)
            let complete = member("until")
                .and_then(|phase| verified.get(phase))
                .is_some_and(|v| v.status == "passed");
            value(
                &lesson.id,
                "status",
                status(member("disposition"), complete),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
