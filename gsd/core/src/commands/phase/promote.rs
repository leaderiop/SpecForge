//! `phase --promote` (D-r43-13): a backlog phase (999.x) becomes the next numbered phase of a milestone, in one plan:
//! the phase is renamed, so are its number-prefixed children, and it depends on the phase before it. The backlog is
//! roadmap-wide (upstream's `## Backlog`), so the phase may sit in another milestone: it moves under the target one.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::remove::{human_blocker, rename_children};
use super::{Number, Phase, find, next_number};
use crate::commands::support::answer;

/// The backlog phase `name` (`999.1`, or `v1.0/999.1` to say which milestone): in the target milestone's own phases
/// first, else the only one of that name in the project.
fn locate<'a>(
    call: &CommandCall<'a>,
    ms: &GraphNode,
    all: &[Phase<'a>],
    name: &str,
) -> Result<Phase<'a>, String> {
    let (scope, short) = name.rsplit_once('/').unwrap_or(("", name));
    let number = Number::parse(short).ok_or_else(|| format!("'{short}' is not a phase number"))?;
    if !number.backlog() {
        return Err(format!(
            "phase '{name}' is not a backlog phase: --promote takes 999.x"
        ));
    }
    if (scope.is_empty() || scope == ms.id)
        && let Ok(own) = find(all, short)
    {
        return Ok(Phase {
            node: own.node,
            number: own.number,
        });
    }
    let found: Vec<&GraphNode> = call
        .graph()
        .nodes()
        .iter()
        .filter(|n| n.kind == "phase" && n.id.rsplit('/').next() == Some(short))
        .filter(|n| scope.is_empty() || n.text("milestone") == Some(scope))
        .collect();
    match found.as_slice() {
        [one] => Ok(Phase { node: one, number }),
        [] => Err(format!("no backlog phase '{name}' in the project")),
        many => Err(format!(
            "name the backlog phase with its milestone: {}",
            many.iter()
                .map(|n| n.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

pub(super) fn promote(
    call: &CommandCall<'_>,
    ms: &GraphNode,
    all: &[Phase<'_>],
    name: &str,
) -> Result<CommandOutput, String> {
    let from = locate(call, ms, all, name)?;
    if let Some((field, entity)) = human_blocker(call, &[&from]) {
        return Err(format!(
            "human_only: '{field}' of override '{}' was written by a person: delete or move it in your editor, then run `/gsd-phase --promote {name}` again",
            entity.id
        ));
    }
    let (last, number) = next_number(all);
    let mut plan = call.plan();
    let new_id = format!("{}/{}", ms.id, number.name());
    let old_id = from.node.id.clone();
    if from.node.text("milestone") == Some(ms.id.as_str()) {
        plan.rename(&old_id, &number.name());
    } else {
        // another milestone's backlog: the phase moves under this one (its id follows its milestone), then takes its number
        plan.set(&old_id, "milestone", json!(ms.id));
        let moved = format!(
            "{}/{}",
            ms.id,
            from.node.id.rsplit('/').next().unwrap_or_default()
        );
        plan.rename(&moved, &number.name());
    }
    rename_children(call, &mut plan, &from, number, &new_id);
    if let Some(previous) = last {
        plan.set(&new_id, "depends_on", json!([previous.node.id]));
    }
    Ok(answer(
        call,
        json!({ "promoted": old_id, "to": new_id }),
        format!("phase {name} promoted to {} in {}\n", number.name(), ms.id),
        Some(&plan),
    ))
}
