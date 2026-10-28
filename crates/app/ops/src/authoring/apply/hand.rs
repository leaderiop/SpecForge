//! What a hand edit changes of a person's own (ADR 0292, D-r33-10): the entities of a human-only kind and the
//! human-only top-level fields, compared between the project and the project as an edit leaves it. A person writes
//! them, so an agent's raw write of one is the spec guard's to deny.

use std::collections::BTreeMap;

use serde_json::Value;
use specforge_protocol_types::FieldWriter;

use crate::extension::command::graph_export;
use crate::view::ProjectView;

/// The graph export's entities by id, without what a move of the text changes (`file`, `line`) or a compile derives
/// from other entities (`coverage_status`, `judgements`).
fn entities(view: &ProjectView<'_>) -> BTreeMap<String, Value> {
    let export: Value = serde_json::from_str(&graph_export(view)).unwrap_or(Value::Null);
    let mut out = BTreeMap::new();
    for mut node in export["nodes"].as_array().cloned().unwrap_or_default() {
        if let Some(map) = node.as_object_mut() {
            for key in ["file", "line", "coverage_status", "judgements"] {
                map.remove(key);
            }
        }
        if let Some(id) = node["id"].as_str().map(str::to_string) {
            out.insert(id, node);
        }
    }
    out
}

/// What `after` changes of a person's: one sentence per entity added, removed or changed in a human-only kind, and per
/// human-only top-level field set, changed or unset on another.
// shortcut: a human-only member inside a block field is not compared, only a top-level field; compare members when
// a project declares one.
pub(crate) fn human_changes(before: &ProjectView<'_>, after: &ProjectView<'_>) -> Vec<String> {
    let (old, new) = (entities(before), entities(after));
    let registries = after.registries();
    let mut changes = Vec::new();
    for id in old
        .keys()
        .chain(new.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let (was, is) = (old.get(id), new.get(id));
        let kind = is
            .or(was)
            .and_then(|n| n["kind"].as_str())
            .unwrap_or_default();
        let human_kind = registries
            .kinds()
            .get(kind)
            .is_some_and(|entry| entry.is_human_only());
        if human_kind {
            if was != is {
                let how = match (was, is) {
                    (None, _) => "added",
                    (_, None) => "removed",
                    _ => "changed",
                };
                changes.push(format!(
                    "{kind} '{id}' is written by a human, and this write {how} it"
                ));
            }
            continue;
        }
        let (Some(was), Some(is)) = (was, is) else {
            // A new entity's human fields are checked below against an empty entity.
            if let Some(node) = is {
                changes.extend(human_fields(after, kind, id, &Value::Null, node));
            }
            continue;
        };
        changes.extend(human_fields(after, kind, id, was, is));
    }
    changes
}

fn human_fields(
    view: &ProjectView<'_>,
    kind: &str,
    id: &str,
    was: &Value,
    is: &Value,
) -> Vec<String> {
    let mut keys: Vec<&String> = Vec::new();
    for node in [was, is] {
        if let Some(fields) = node["fields"].as_object() {
            keys.extend(fields.keys());
        }
    }
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|key| {
            view.registries()
                .fields()
                .get(kind, key)
                .is_some_and(|entry| entry.form().writer() == Some(FieldWriter::Human))
                && was["fields"][key.as_str()] != is["fields"][key.as_str()]
        })
        .map(|key| {
            format!("'{key}' of {kind} '{id}' is written by a human, and this write changes it")
        })
        .collect()
}
