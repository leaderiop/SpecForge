//! The project block (ADR 0360 D1, ADR 0372 D8): the target's `specforge.json` and `spec` block are init's, written
//! fresh with the source's name, version and spec root. The entries this module records are the source's values that
//! scaffold replaces: each is `unconverted`, with the one outcome [`WRITTEN_FRESH`], and they are the only
//! `unconverted` entries a run ledgers. A version the target was given because the source had none is `filled`.

use serde_json::Value;
use specforge_graph::field_form;
use specforge_project::CompiledProject;
use std::collections::BTreeMap;

use super::ledger::{Ledger, Why};

/// The ledger's kind for a `specforge.json` member, and for a `spec` block's field.
const CONFIG: &str = "specforge.json";
const SPEC: &str = "spec";
/// What a `specforge.json` member is written as: `$schema` names the format, never the project.
const SCHEMA: &str = "$schema";
/// The outcome of every source value the target's scaffold, written fresh, does not hold (ADR 0372 D8).
pub const WRITTEN_FRESH: &str = "written fresh by absorb";

/// Ledger what `old`'s project block (named `name`) holds that `new`'s does not: each `specforge.json` member but
/// `$schema` whose value differs or is not there (an `exclude`, a setting), each item of a list the target's does not
/// hold (an old extension), a version the target was given because the source had none, and each `spec` block whose id or a field the target's block does not
/// hold as it is.
pub fn ledger(old: &CompiledProject, new: &CompiledProject, name: &str, ledger: &mut Ledger) {
    let config = |project: &CompiledProject| {
        project
            .environment()
            .config()
            .raw
            .clone()
            .unwrap_or(Value::Null)
    };
    let (was, now) = (config(old), config(new));
    for (key, value) in was.as_object().into_iter().flatten() {
        let held = now.get(key);
        // A list (the extensions) is compared item by item: the target may load some of the old ones.
        let lost: Vec<&Value> = match (value, held) {
            _ if key == SCHEMA => Vec::new(),
            (Value::Array(items), Some(Value::Array(new))) => {
                items.iter().filter(|item| !new.contains(item)).collect()
            }
            (value, held) if held != Some(value) => vec![value],
            _ => Vec::new(),
        };
        for value in lost {
            let why = Why::Unconverted(WRITTEN_FRESH.to_string());
            ledger.record(CONFIG, name, key, value.clone(), why);
        }
    }
    if was.get("version").is_none()
        && let Some(version) = now.get("version")
    {
        let why = Why::Filled {
            field: format!("{CONFIG}.version"),
            value: version.clone(),
        };
        ledger.record(CONFIG, name, "version", Value::Null, why);
    }
    let target = blocks(new).into_iter().next();
    for (id, fields) in blocks(old) {
        let (new_id, new_fields) = match &target {
            Some((new_id, new_fields)) => (new_id.as_str(), Some(new_fields)),
            None => ("", None),
        };
        let fresh = || Why::Unconverted(WRITTEN_FRESH.to_string());
        if new_id != id {
            ledger.record(SPEC, &id, "id", Value::String(id.clone()), fresh());
        }
        for (field, value) in fields {
            if new_fields.and_then(|f| f.get(&field)) != Some(&value) {
                ledger.record(SPEC, &id, &field, value, fresh());
            }
        }
    }
}

/// Each `spec` block of `project`: its id and its fields in the one wire form every output writes.
fn blocks(project: &CompiledProject) -> Vec<(String, BTreeMap<String, Value>)> {
    project
        .graph()
        .nodes()
        .into_iter()
        .filter(|node| node.kind.raw.as_str() == SPEC)
        .map(|node| {
            let fields = node
                .fields
                .entries()
                .iter()
                .map(|entry| {
                    let value = field_form::write(&entry.value).as_value().clone();
                    (entry.key.to_string(), value)
                })
                .collect();
            (node.id.raw.to_string(), fields)
        })
        .collect()
}
