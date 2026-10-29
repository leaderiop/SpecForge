//! `phase --remove` (D-r41-8): what may be removed, what a person wrote that blocks it, and the one plan that deletes
//! the phase and renumbers the later ones, children included.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::{Number, Phase, target};
use crate::commands::support::answer;

/// The prefix of the names of a phase's own children: `05-01`, `05-CONTEXT`.
fn prefix(number: Number) -> String {
    format!("{}-", number.name())
}

/// The entities a person wrote that sit in, or point into, one of `phases` (the ones deleted or renamed).
fn human_blocker<'a>(
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

pub(super) fn remove(
    call: &CommandCall<'_>,
    ms: &GraphNode,
    all: &[Phase<'_>],
    name: &str,
) -> Result<CommandOutput, String> {
    let gone = target(all, name)?;
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
    let mut affected = vec![gone];
    affected.extend(moves.iter().map(|(p, _)| *p));
    if let Some((field, entity)) = human_blocker(call, &affected) {
        return Err(format!(
            "human_only: '{field}' of override '{}' was written by a person: delete or move it in your editor, then run `/gsd-phase --remove {}` again",
            entity.id,
            gone.number.name()
        ));
    }
    let mut plan = call.plan();
    let under = format!("{}/", gone.node.id);
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
    for (phase, number) in &moves {
        let (old, new) = (prefix(phase.number), prefix(*number));
        plan.rename(&phase.node.id, &number.name());
        let new_id = format!("{}/{}", ms.id, number.name());
        let children = format!("{}/", phase.node.id);
        for child in call
            .graph()
            .nodes()
            .iter()
            .filter(|n| n.id.starts_with(&children))
        {
            let short = &child.id[children.len()..];
            if let Some(rest) = short.strip_prefix(&old).filter(|_| !short.contains('/')) {
                plan.rename(&format!("{new_id}/{short}"), &format!("{new}{rest}"));
            }
        }
        renamed.push(json!({ "from": phase.number.name(), "to": number.name() }));
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
