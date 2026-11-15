//! What every plan pass reads the same way: the plans, and the phase each one is in.
//!
//! Also ports gsd-core bin/lib modules: plan-*.

use std::collections::{BTreeMap, BTreeSet};

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

/// The decisions the plans `plans` (ids) list in `decisions`, each resolved id once: what E310, W317, W506 and E312
/// read as covered, shipped or decided.
pub fn listed_decisions<'a, 'b>(
    input: &'a PassInput,
    plans: impl IntoIterator<Item = &'b str>,
) -> BTreeSet<&'a str> {
    plans
        .into_iter()
        .flat_map(|plan| {
            input
                .edges
                .iter()
                .filter(move |e| e.source == plan && e.label == "decisions")
                .map(|e| e.target.as_str())
        })
        .collect()
}

/// The decisions no longer in force, so no plan must list them: each one a decision lists in `supersedes` with force.
/// - A settled decision, locked with the user, leaves force only through a decision taken with the user too (`origin`
///   `context` or `project`); an agent's `summary` or `checkpoint` decision cannot retire it.
/// - A decision that supersedes itself, directly or round a cycle of such edges, is reversed by nothing outside the
///   cycle: an edge between two decisions of one cycle retires neither (the graph's W061 reports the cycle).
pub fn superseded(input: &PassInput) -> BTreeSet<&str> {
    let decisions: BTreeMap<&str, &PassEntity> = of_kind(input, "decision")
        .map(|d| (d.id.as_str(), d))
        .collect();
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in input.edges.iter().filter(|e| e.label == "supersedes") {
        let (Some(by), Some(earlier)) = (
            decisions.get(edge.source.as_str()),
            decisions.get(edge.target.as_str()),
        ) else {
            continue;
        };
        let with_the_user = matches!(text(by, "origin"), Some("context" | "project"));
        if text(earlier, "settled") != Some("true") || with_the_user {
            graph.entry(&by.id).or_default().push(&earlier.id);
        }
    }
    let cycle_of: BTreeMap<&str, usize> = super::cycles::cycles(&graph)
        .into_iter()
        .enumerate()
        .flat_map(|(n, members)| members.into_iter().map(move |m| (m, n)))
        .collect();
    graph
        .iter()
        .flat_map(|(by, earlier)| earlier.iter().map(move |e| (*by, *e)))
        .filter(|(by, earlier)| {
            cycle_of
                .get(by)
                .is_none_or(|n| cycle_of.get(earlier) != Some(n))
        })
        .map(|(_, earlier)| earlier)
        .collect()
}

/// The trackable decisions of a phase's context (`origin context`) still in force (none [`superseded`]), in id order:
/// the ones its plans must list.
pub fn context_decisions<'a>(input: &'a PassInput, phase: &str) -> Vec<&'a PassEntity> {
    let superseded = superseded(input);
    children(input, phase, "decision")
        .into_iter()
        .filter(|d| text(d, "origin") == Some("context") && !superseded.contains(d.id.as_str()))
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

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `\b<needle>\b` in `haystack` (JavaScript's ASCII word boundary), case-insensitively for ASCII letters when `fold`.
/// The needle starts and ends with a word character (an id, a phrase).
pub fn has_word(haystack: &str, needle: &str, fold: bool) -> bool {
    let (haystack, needle) = if fold {
        (haystack.to_ascii_lowercase(), needle.to_ascii_lowercase())
    } else {
        (haystack.to_string(), needle.to_string())
    };
    let bytes = haystack.as_bytes();
    haystack.match_indices(&needle).any(|(at, found)| {
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(at + found.len()).copied();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
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
/// when the host sends one, the field text otherwise. The pass input carries no title, so a title is not here.
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
