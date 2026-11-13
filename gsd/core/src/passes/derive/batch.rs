//! The computed values of a quick batch (`spec/execute.md`): its status and its eligible items, from its items' plan
//! statuses. An item is eligible while its plan has not started, halted or blocked and every plan it depends on is
//! complete; the batch is complete once every item is.

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use super::progress::Statuses;
use crate::passes::plans::{links, of_kind};

/// What a status says of an item: `complete`, or still to run (`eligible` needs its dependencies too).
fn waiting(status: &str) -> bool {
    !matches!(status, "executing" | "complete" | "halted" | "blocked")
}

pub fn values(input: &PassInput, statuses: &Statuses) -> Vec<PassValue> {
    let status = |plan: &str| statuses.get(plan).copied().unwrap_or("ready");
    let mut out = Vec::new();
    for batch in of_kind(input, "quick_batch") {
        let items = links(input, batch, "items");
        let eligible: Vec<&str> = items
            .iter()
            .copied()
            .filter(|item| waiting(status(item)))
            .filter(|item| {
                input
                    .entities
                    .iter()
                    .find(|e| e.id == *item)
                    .is_none_or(|plan| {
                        links(input, plan, "depends_on")
                            .iter()
                            .all(|d| status(d) == "complete")
                    })
            })
            .collect();
        let done = !items.is_empty() && items.iter().all(|i| status(i) == "complete");
        out.push(value(
            &batch.id,
            "status",
            if done { "complete" } else { "open" },
        ));
        out.push(value(&batch.id, "eligible", eligible));
    }
    out
}

#[cfg(test)]
mod tests;
