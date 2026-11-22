//! What every plan pass reads the same way: the plans, and the phase each one is in.
//!
//! Also ports gsd-core bin/lib modules: plan-*.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

pub fn of_kind<'a>(input: &'a PassInput, kind: &'a str) -> impl Iterator<Item = &'a PassEntity> {
    input.entities.iter().filter(move |e| e.kind == kind)
}

/// The tasks of each plan that has one, a task being its plan's by the scope of its id, in [`id_order`]: the one count
/// a summary's `actuals.tasks`, a plan's `estimate_error` and its weight on the critical path read, and the one order
/// of the critical path's tasks.
///
/// [`id_order`]: crate::commands::id_order
pub fn tasks_by_plan(input: &PassInput) -> BTreeMap<&str, Vec<&str>> {
    let mut tasks: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for task in of_kind(input, "task") {
        tasks
            .entry(crate::scope::scope_of(&task.id))
            .or_default()
            .push(&task.id);
    }
    for of_plan in tasks.values_mut() {
        of_plan.sort_by_key(|id| crate::commands::id_order(id));
    }
    tasks
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

/// The entity a reference field resolves to (its edge), `None` when unwritten or unresolved. A reference is compared
/// as the id it resolves to, never as written: `01-01/TR1` and `v1.0/01/01-01/TR1` are one target.
pub fn link<'a>(input: &'a PassInput, entity: &PassEntity, field: &str) -> Option<&'a str> {
    input
        .edges
        .iter()
        .find(|e| e.source == entity.id && e.label == field)
        .map(|e| e.target.as_str())
}

/// The entities a list-of-references field resolves to, in edge order.
pub fn links<'a>(input: &'a PassInput, entity: &PassEntity, field: &str) -> Vec<&'a str> {
    input
        .edges
        .iter()
        .filter(|e| e.source == entity.id && e.label == field)
        .map(|e| e.target.as_str())
        .collect()
}

/// The trackable decisions of a phase's context (`origin context`), every one of them, in id order: the ones its
/// plans must cite.
pub fn context_decisions<'a>(input: &'a PassInput, phase: &str) -> Vec<&'a PassEntity> {
    children(input, phase, "decision")
        .into_iter()
        .filter(|d| text(d, "origin") == Some("context"))
        .collect()
}

/// The targets of the `label` edges of `source`.
pub fn targets<'a>(input: &'a PassInput, source: &str, label: &str) -> BTreeSet<&'a str> {
    input
        .edges
        .iter()
        .filter(|e| e.source == source && e.label == label)
        .map(|e| e.target.as_str())
        .collect()
}

/// The target among `targets`, the edges of one field of `holder`, that `written`, an item of that field as written,
/// names. The graph resolves a reference by its holder's scope chain alone (ADR 0284 D4): in the holder itself, each
/// enclosing scope, then the project, and a rooted `/path` in the project only; never by a suffix. So the target is
/// the first spelling of that chain among `targets`: a nearer entity the field admits would have answered the item.
/// An item the graph did not resolve (E003) names none, whatever target ends with it.
pub fn written_target<'a>(
    targets: &BTreeSet<&'a str>,
    holder: &str,
    written: &str,
) -> Option<&'a str> {
    let (path, levels): (&str, Vec<&str>) = match written.strip_prefix('/') {
        Some(path) => (path, vec![""]),
        None => (
            written,
            std::iter::successors(Some(holder), |scope| {
                (!scope.is_empty()).then(|| crate::scope::scope_of(scope))
            })
            .collect(),
        ),
    };
    levels.into_iter().find_map(|level| {
        let id = if level.is_empty() {
            path.to_string()
        } else {
            format!("{level}/{path}")
        };
        targets.get(id.as_str()).copied()
    })
}

/// The last `/`-segment of an id: its name.
pub fn name_of(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}

/// The entities of `kind` scoped by `plan` (a task, a checkpoint or a must-have of it), in id order.
pub fn children<'a>(input: &'a PassInput, plan: &str, kind: &'a str) -> Vec<&'a PassEntity> {
    let mut found: Vec<&PassEntity> = of_kind(input, kind)
        .filter(|e| crate::scope::scope_of(&e.id) == plan)
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

/// The text of a JSON scalar, as the author wrote it.
fn scalar<V: std::fmt::Display>(value: &V, as_str: Option<&str>) -> String {
    as_str.map_or_else(|| value.to_string(), str::to_string)
}

/// The items of a list of strings (`values`): empty when the field is unwritten or not a list.
pub fn strings(entity: &PassEntity, field: &str) -> Vec<String> {
    entity
        .values
        .get(field)
        .and_then(|v| v.as_array())
        .map(|items| items.iter().map(|i| scalar(i, i.as_str())).collect())
        .unwrap_or_default()
}

/// The paths a plan names, `files_modified ∪ files_deleted`, as exact strings (`partitionByFileOverlap` compares them
/// as written).
pub fn touched(plan: &PassEntity) -> BTreeSet<String> {
    let mut paths: BTreeSet<String> = strings(plan, "files_modified").into_iter().collect();
    paths.extend(strings(plan, "files_deleted"));
    paths
}

/// The rows of a block list (`values`), each member as its text; a member that is itself a list or block is its
/// compact JSON.
pub fn rows(entity: &PassEntity, field: &str) -> Vec<BTreeMap<String, String>> {
    entity
        .values
        .get(field)
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.as_object())
                .map(|row| {
                    row.iter()
                        .map(|(name, v)| (name.clone(), scalar(v, v.as_str())))
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The members of a block (`values`) as their text; empty when the field is unwritten.
pub fn block(entity: &PassEntity, field: &str) -> BTreeMap<String, String> {
    entity
        .values
        .get(field)
        .and_then(|v| v.as_object())
        .map(|members| {
            members
                .iter()
                .map(|(name, v)| (name.clone(), scalar(v, v.as_str())))
                .collect()
        })
        .unwrap_or_default()
}

/// A written, non-blank text field.
pub fn text<'a>(entity: &'a PassEntity, field: &str) -> Option<&'a str> {
    entity
        .fields
        .get(field)
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
}

/// What a JSON value holds, for [`texts`]: a string, children, or nothing to read.
enum Shape<'a, V> {
    Text(&'a str),
    Children(Vec<&'a V>),
    Nothing,
}

fn walk<'a, V>(value: &'a V, shape: &impl Fn(&'a V) -> Shape<'a, V>, out: &mut Vec<String>) {
    match shape(value) {
        Shape::Text(text) => out.push(text.to_string()),
        Shape::Children(children) => children.into_iter().for_each(|c| walk(c, shape, out)),
        Shape::Nothing => {}
    }
}

/// Every string an entity writes, field by field: the structured value of a list or block (its strings, recursively)
/// when the host sends one, the field text otherwise. An entity's title is no field, so it is not here.
pub fn texts(entity: &PassEntity) -> Vec<String> {
    let mut out = Vec::new();
    for (name, text) in &entity.fields {
        match entity.values.get(name) {
            Some(value) => walk(
                value,
                &|v| {
                    if let Some(text) = v.as_str() {
                        Shape::Text(text)
                    } else if let Some(items) = v.as_array() {
                        Shape::Children(items.iter().collect())
                    } else if let Some(members) = v.as_object() {
                        Shape::Children(members.values().collect())
                    } else {
                        Shape::Nothing
                    }
                },
                &mut out,
            ),
            None => out.push(text.clone()),
        }
    }
    out
}

/// The reference fields the unit tests write as full ids; [`with_field_edges`] resolves them as the host would.
#[cfg(test)]
const REFERENCES: &[&str] = &[
    "phase",
    "plan",
    "verification",
    "validation",
    "truth",
    "task",
    "uat",
    "session",
    "test",
    "refactor",
    "milestone",
    "for",
    "requirement",
];

/// A pass input as JSON, plus one edge per reference field an entity writes (test inputs write full ids, so each
/// resolves to itself).
#[cfg(test)]
pub fn with_field_edges(mut input: serde_json::Value) -> serde_json::Value {
    let mut edges: Vec<serde_json::Value> = input
        .get("edges")
        .and_then(|e| e.as_array().cloned())
        .unwrap_or_default();
    for entity in input["entities"].as_array().into_iter().flatten() {
        for (name, value) in entity["fields"].as_object().into_iter().flatten() {
            if let (true, Some(target)) = (REFERENCES.contains(&name.as_str()), value.as_str()) {
                let edge =
                    serde_json::json!({ "source": entity["id"], "target": target, "label": name });
                if !edges.contains(&edge) {
                    edges.push(edge);
                }
            }
        }
    }
    for entity in input["entities"].as_array().into_iter().flatten() {
        for name in ["gap_closure", "requirements"] {
            for target in entity["values"][name].as_array().into_iter().flatten() {
                let edge =
                    serde_json::json!({ "source": entity["id"], "target": target, "label": name });
                if !edges.contains(&edge) {
                    edges.push(edge);
                }
            }
        }
    }
    input["edges"] = edges.into();
    input
}

#[cfg(test)]
mod tests;
