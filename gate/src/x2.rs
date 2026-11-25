//! X2 (ADR 0388): of a project's `item` facts that carry a `test_id`, the share whose test id is the id of a
//! `test` fact, of any status. X2 is a floor every project the `dogfood` check collects must reach: 100%, and never
//! `0/0` (ADR 0393 D8). Pure functions the check calls; each is tested on literal input.

use std::collections::BTreeSet;

use serde_json::Value;

/// X2 over one project: `present` of `total` items with a test id have a test fact; `missing` are the others' ids.
pub(crate) struct X2 {
    pub present: usize,
    pub total: usize,
    pub missing: Vec<String>,
}

/// X2 over `items` (each an item id and its `test_id`) and the ids of the `test` facts: the missing item ids come
/// out sorted, once each.
pub(crate) fn ratio(items: &[(String, String)], tests: &BTreeSet<String>) -> X2 {
    let present = items
        .iter()
        .filter(|(_, test)| tests.contains(test))
        .count();
    let missing: BTreeSet<String> = items
        .iter()
        .filter(|(_, test)| !tests.contains(test))
        .map(|(id, _)| id.clone())
        .collect();
    X2 {
        present,
        total: items.len(),
        missing: missing.into_iter().collect(),
    }
}

/// What [`read`] reads of an export: each item id with its `test_id`, and the ids of the `test` facts.
pub(crate) type Read = (Vec<(String, String)>, BTreeSet<String>);

/// The `item` nodes of `export` (`specforge export --format graph --kinds item,test`) that carry a `test_id`, each
/// with it, and the ids of its `test` nodes, whatever their status. An error when the export is of another shape (no
/// `nodes` array, a node with no string `id` or `kind`): it would read as `0/0`, the line of a project whose items
/// carry no test id, and hide the change.
pub(crate) fn read(export: &Value) -> Result<Read, String> {
    let nodes = export["nodes"]
        .as_array()
        .ok_or("the export has no `nodes` array: its shape changed")?;
    let mut items = Vec::new();
    let mut tests = BTreeSet::new();
    for (n, node) in nodes.iter().enumerate() {
        let (Some(id), Some(kind)) = (node["id"].as_str(), node["kind"].as_str()) else {
            return Err(format!(
                "the export's nodes[{n}] has no string `id` and `kind`: its shape changed"
            ));
        };
        match kind {
            "item" => {
                if let Some(test) = node["fields"]["test_id"].as_str() {
                    items.push((id.to_string(), test.to_string()));
                }
            }
            "test" => {
                tests.insert(id.to_string());
            }
            _ => {}
        }
    }
    Ok((items, tests))
}

/// `X2 <project>: n/m (p%)`, the percent rounded down to one decimal so that `100.0%` means no item is missing, or
/// `X2 <project>: 0/0 (no item has a test_id)` when no item has one, then each missing item id on its own line,
/// indented two spaces.
pub(crate) fn lines(project: &str, x2: &X2) -> Vec<String> {
    // Integer tenths of a percent: a float's 100 * 29 / 100 is 28.999…, which would floor to 28.9.
    let head = match x2.present.saturating_mul(1000).checked_div(x2.total) {
        None => format!("X2 {project}: 0/0 (no item has a test_id)"),
        Some(tenths) => format!(
            "X2 {project}: {}/{} ({}.{}%)",
            x2.present,
            x2.total,
            tenths / 10,
            tenths % 10
        ),
    };
    std::iter::once(head)
        .chain(x2.missing.iter().map(|id| format!("  {id}")))
        .collect()
}

/// How many missing item ids [`floor`]'s error names; the log lists them all.
pub(crate) const MISSING_NAMED: usize = 10;

/// The floor (ADR 0393 D8): Ok when every one of at least one item with a test id has a test fact. Otherwise the head
/// of [`lines`] (its rounded-down percent) and the first [`MISSING_NAMED`] missing ids, with how many more there are;
/// `X2 <project>: no item has a test_id` on `0/0`.
pub(crate) fn floor(project: &str, x2: &X2) -> Result<(), String> {
    if x2.total == 0 {
        return Err(format!("X2 {project}: no item has a test_id"));
    }
    if x2.missing.is_empty() {
        return Ok(());
    }
    let head = lines(project, x2).swap_remove(0);
    let named = x2.missing[..x2.missing.len().min(MISSING_NAMED)].join(", ");
    let more = x2.missing.len().saturating_sub(MISSING_NAMED);
    Err(if more == 0 {
        format!("{head}: missing {named}")
    } else {
        format!("{head}: missing {named}, and {more} more (the log lists all)")
    })
}

#[cfg(test)]
mod tests;
