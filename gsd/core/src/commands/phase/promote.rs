//! `phase --promote` (D-r43-13): a backlog phase (999.x) becomes the next numbered phase of its milestone, in one plan:
//! the phase is renamed, so are its number-prefixed children, and it depends on the phase before it.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::remove::{human_blocker, rename_children};
use super::{Phase, find, next_number};
use crate::commands::support::answer;

pub(super) fn promote(
    call: &CommandCall<'_>,
    ms: &GraphNode,
    all: &[Phase<'_>],
    name: &str,
) -> Result<CommandOutput, String> {
    let from = find(all, name)?;
    if !from.number.backlog() {
        return Err(format!(
            "phase '{name}' is not a backlog phase: --promote takes 999.x"
        ));
    }
    if let Some((field, entity)) = human_blocker(call, &[from]) {
        return Err(format!(
            "human_only: '{field}' of override '{}' was written by a person: delete or move it in your editor, then run `/gsd-phase --promote {name}` again",
            entity.id
        ));
    }
    let (last, number) = next_number(all);
    let mut plan = call.plan();
    plan.rename(&from.node.id, &number.name());
    let new_id = format!("{}/{}", ms.id, number.name());
    rename_children(call, &mut plan, from, number, &new_id);
    plan.set(
        &new_id,
        "depends_on",
        last.map_or(json!(null), |p| json!([p.node.id])),
    );
    Ok(answer(
        call,
        json!({ "promoted": from.node.id, "to": new_id }),
        format!("phase {name} promoted to {} in {}\n", number.name(), ms.id),
        Some(&plan),
    ))
}
