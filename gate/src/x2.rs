//! X2 (ADR 0388): of a project's `item` facts that carry a `test_id`, the share whose test id is the id of a
//! `test` fact, of any status. Pure functions the `dogfood` check calls; each is tested on literal input.

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

/// The `item` nodes of `export` (`specforge export --format graph --kinds item,test`) that carry a `test_id`, each
/// with it, and the ids of its `test` nodes, whatever their status.
pub(crate) fn read(export: &Value) -> (Vec<(String, String)>, BTreeSet<String>) {
    let nodes = export["nodes"].as_array().into_iter().flatten();
    let mut items = Vec::new();
    let mut tests = BTreeSet::new();
    for node in nodes {
        let Some(id) = node["id"].as_str() else {
            continue;
        };
        match node["kind"].as_str() {
            Some("item") => {
                if let Some(test) = node["fields"]["test_id"].as_str() {
                    items.push((id.to_string(), test.to_string()));
                }
            }
            Some("test") => {
                tests.insert(id.to_string());
            }
            _ => {}
        }
    }
    (items, tests)
}

/// `X2 <project>: n/m (p%)`, or `X2 <project>: 0/0 (no item has a test_id)` when no item has one, then each missing
/// item id on its own line, indented two spaces.
pub(crate) fn lines(project: &str, x2: &X2) -> Vec<String> {
    let head = if x2.total == 0 {
        format!("X2 {project}: 0/0 (no item has a test_id)")
    } else {
        let percent = 100.0 * x2.present as f64 / x2.total as f64;
        format!("X2 {project}: {}/{} ({percent:.1}%)", x2.present, x2.total)
    };
    std::iter::once(head)
        .chain(x2.missing.iter().map(|id| format!("  {id}")))
        .collect()
}

#[cfg(test)]
mod tests;
