//! The values compute passes gave (ADR 0287): held beside the graph, never in a node, a graph delta or a `.spec`
//! file. Merged into the entity snapshot (`EntitySnapshot::with_computed`) before the rules read it.

use std::collections::BTreeMap;

use crate::FieldJson;

/// Every computed value of a compiled project, by entity id and the written path of the field or member
/// (`status`, `checks[0].status`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Computed(BTreeMap<String, BTreeMap<String, FieldJson>>);

impl Computed {
    /// A value an entity's field has; a later one for the same path replaces it.
    pub fn insert(&mut self, id: &str, path: &str, value: FieldJson) {
        self.0
            .entry(id.to_string())
            .or_default()
            .insert(path.to_string(), value);
    }

    /// The entity's values by written path, in path order.
    pub fn of(&self, id: &str) -> impl Iterator<Item = (&str, &FieldJson)> {
        self.0
            .get(id)
            .into_iter()
            .flat_map(|values| values.iter().map(|(path, value)| (path.as_str(), value)))
    }

    /// The value at `path` of `id`.
    pub fn get(&self, id: &str, path: &str) -> Option<&FieldJson> {
        self.0.get(id)?.get(path)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
