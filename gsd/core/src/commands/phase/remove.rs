//! `phase --remove` (D-r41-8): what may be removed, what a person wrote that blocks it, and the one plan that deletes
//! the phase and renumbers the later ones, children included.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::{Number, Phase, find};
use crate::commands::support::answer;

/// The prefix of the names of a phase's own children: `05-01`, `05-CONTEXT`.
pub(super) fn prefix(number: Number) -> String {
    format!("{}-", number.name())
}

/// The entities a person wrote that sit in, or point into, one of `phases` (the ones deleted or renamed).
pub(super) fn human_blocker<'a>(
    call: &CommandCall<'a>,
    phases: &[&Phase<'_>],
) -> Option<(&'static str, &'a GraphNode)> {
    call.graph()
        .nodes()
        .iter()
        .filter(|n| n.kind == "override")
        .find_map(|n| {
            let inside = |text: &str| {
                phases
                    .iter()
                    .any(|p| text.starts_with(&format!("{}/", p.node.id)))
            };
            if inside(&n.id) {
                Some(("id", n))
            } else if n.text("truth").is_some_and(inside) {
                Some(("truth", n))
            } else {
                None
            }
        })
}

/// A phase has started: one of its plans has a summary, or a commit names one.
fn started(call: &CommandCall<'_>, phase: &Phase<'_>) -> bool {
    let under = format!("{}/", phase.node.id);
    call.graph().nodes().iter().any(|n| {
        (n.kind == "summary" && n.id.starts_with(&under))
            || (n.kind == "commit" && n.text("plan").is_some_and(|p| p.starts_with(&under)))
    })
}

/// The number each later phase takes once `removed` is gone.
fn renumbered(removed: Number, later: Number) -> Option<Number> {
    match removed.decimal {
        None if later.whole > removed.whole => Some(Number {
            whole: later.whole - 1,
            ..later
        }),
        Some(k) if later.whole == removed.whole && later.decimal > Some(k) => Some(Number {
            decimal: later.decimal.map(|d| d - 1),
            ..later
        }),
        _ => None,
    }
}

/// The order a subtree is deleted in: an entity goes before what holds it (its path) and before what it refers to,
/// since a reference inside a block list cannot be unlinked by the host. The deepest of those free goes first; a cycle
/// is broken the same way.
fn deletion_order<'a>(call: &CommandCall<'_>, mut left: Vec<&'a GraphNode>) -> Vec<&'a GraphNode> {
    let edges = call.graph().edges();
    let depth = |n: &GraphNode| n.id.matches('/').count();
    let mut order = Vec::with_capacity(left.len());
    while !left.is_empty() {
        let blocked = |x: &GraphNode| {
            left.iter().any(|y| {
                y.id != x.id
                    && (y.id.starts_with(&format!("{}/", x.id))
                        || edges.iter().any(|e| e.source == y.id && e.target == x.id))
            })
        };
        let pick = left
            .iter()
            .enumerate()
            .filter(|(_, n)| !blocked(n))
            .max_by_key(|(_, n)| depth(n))
            .or_else(|| left.iter().enumerate().max_by_key(|(_, n)| depth(n)))
            .map_or(0, |(i, _)| i);
        order.push(left.remove(pick));
    }
    order
}

/// Renames the number-prefixed children of `phase` (`05-01`, `05-CONTEXT`, and what they hold) once the phase itself is
/// renamed to `number`, whose id is `new_id`.
pub(super) fn rename_children(
    call: &CommandCall<'_>,
    plan: &mut PlanBuilder,
    phase: &Phase<'_>,
    number: Number,
    new_id: &str,
) {
    let (old, new) = (prefix(phase.number), prefix(number));
    let children = format!("{}/", phase.node.id);
    let mut below: Vec<&str> = call
        .graph()
        .nodes()
        .iter()
        .filter_map(|n| n.id.strip_prefix(&children))
        .collect();
    // Shallow first: a child is renamed before what it holds, whose path then carries the new name.
    below.sort_by_key(|short| short.matches('/').count());
    for short in below {
        let swap = |seg: &str| match seg.strip_prefix(&old) {
            Some(rest) => format!("{new}{rest}"),
            None => seg.to_string(),
        };
        let (parent, last) = short.rsplit_once('/').unwrap_or(("", short));
        if last.starts_with(&old) {
            let parent: Vec<String> = parent
                .split('/')
                .filter(|s| !s.is_empty())
                .map(swap)
                .collect();
            let at = [new_id]
                .into_iter()
                .chain(parent.iter().map(String::as_str))
                .chain([last])
                .collect::<Vec<_>>()
                .join("/");
            plan.rename(&at, &swap(last));
        }
    }
}

pub(super) fn remove(
    call: &CommandCall<'_>,
    ms: &GraphNode,
    all: &[Phase<'_>],
    name: &str,
) -> Result<CommandOutput, String> {
    let gone = find(all, name)?;
    if !call.flag("force") && started(call, gone) {
        return Err(format!(
            "phase {} has started (a plan has a summary, or a commit names one): --force removes it anyway",
            gone.number.name()
        ));
    }
    if gone.number.decimal.is_none()
        && all
            .iter()
            .any(|p| p.number.whole == gone.number.whole && p.number.decimal.is_some())
    {
        return Err(format!(
            "phase {} has decimal phases: remove them first",
            gone.number.name()
        ));
    }
    let moves: Vec<(&Phase<'_>, Number)> = all
        .iter()
        .filter(|p| !p.number.backlog())
        .filter_map(|p| renumbered(gone.number, p.number).map(|n| (p, n)))
        .collect();
    if !call.flag("force")
        && let Some((phase, _)) = moves.iter().find(|(p, _)| started(call, p))
    {
        return Err(format!(
            "phase {} has started and would be renamed (its plans, summaries and commits carry its number): --force renames it anyway",
            phase.number.name()
        ));
    }
    let mut affected = vec![gone];
    affected.extend(moves.iter().map(|(p, _)| *p));
    if let Some((field, entity)) = human_blocker(call, &affected) {
        return Err(format!(
            "human_only: '{field}' of override '{}' was written by a person: delete or move it in your editor, then run `/gsd-phase --remove {}` again",
            entity.id,
            gone.number.name()
        ));
    }
    let under = format!("{}/", gone.node.id);
    // A tier is a person's (`truth.verification` is `U`, D-K20a-4), so deleting its truth is the host's `human_only`:
    // say which line to delete. Upstream deletes regardless (REVIEW-k20a M3).
    if let Some(truth) = call
        .graph()
        .nodes()
        .iter()
        .find(|n| n.kind == "truth" && n.id.starts_with(&under) && n.has_field("verification"))
    {
        return Err(format!(
            "human_only: 'verification' of truth '{}' was written by a person: delete that line in your editor, then run `/gsd-phase --remove {}` again",
            truth.id,
            gone.number.name()
        ));
    }
    let mut plan = call.plan();
    let subtree: Vec<&GraphNode> = call
        .graph()
        .nodes()
        .iter()
        .filter(|n| n.id.starts_with(&under))
        .collect();
    let subtree = deletion_order(call, subtree);
    for entity in &subtree {
        plan.delete(&entity.id, true);
    }
    plan.delete(&gone.node.id, true);
    let mut renamed = Vec::new();
    let milestone = format!("{}/", ms.id);
    let new_of = |id: &str| {
        moves.iter().find(|(p, _)| p.node.id == id).map_or_else(
            || id.to_string(),
            |(_, n)| format!("{milestone}{}", n.name()),
        )
    };
    for (phase, number) in &moves {
        plan.rename(&phase.node.id, &number.name());
        let new_id = format!("{milestone}{}", number.name());
        rename_children(call, &mut plan, phase, *number, &new_id);
        renamed.push(json!({ "from": phase.number.name(), "to": number.name() }));
    }
    // A phase that depended on the removed one inherits its dependencies, so the chain stays linear.
    let edges = call.graph().edges();
    let deps = |id: &str| -> Vec<&str> {
        edges
            .iter()
            .filter(|e| e.label == "depends_on" && e.source == id)
            .map(|e| e.target.as_str())
            .collect()
    };
    let inherited = deps(&gone.node.id);
    if !inherited.is_empty() {
        for p in all.iter().filter(|p| p.node.id != gone.node.id) {
            let own = deps(&p.node.id);
            if own.contains(&gone.node.id.as_str()) {
                let mut now: Vec<String> = Vec::new();
                for d in own.iter().filter(|d| **d != gone.node.id).chain(&inherited) {
                    let d = new_of(d);
                    if !now.contains(&d) {
                        now.push(d);
                    }
                }
                plan.set(&new_of(&p.node.id), "depends_on", json!(now));
            }
        }
    }
    Ok(answer(
        call,
        json!({ "removed": gone.node.id, "deleted": subtree.len() + 1, "renumbered": renamed }),
        format!(
            "phase {} removed with {} entities; {} later phases renumbered\n",
            gone.number.name(),
            subtree.len(),
            moves.len()
        ),
        Some(&plan),
    ))
}
