//! What a typed edit may change in the graph (ADR 0198): each edit of a list declares it, and the plan is refused
//! when the edited project's graph delta holds anything beyond it, as a migration keeps only its steps' declared
//! changes (ADR 0186).

use super::Edit;
use specforge_graph::{GraphDelta, NodeAspect};
use std::collections::{BTreeMap, BTreeSet};

/// What a list of edits may change: per entity, the fields and the aspects its edits name; and the entities a create
/// or a delete names, about which anything may change.
#[derive(Debug, Default)]
pub(crate) struct Declared {
    changes: BTreeMap<String, Change>,
    whole: BTreeSet<String>,
    /// The files a rename of the plan rewrites: it changes the graph by a substitution only (ADR 0198), so every
    /// entity of these files may read differently afterwards.
    renamed: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct Change {
    fields: BTreeSet<String>,
    aspects: BTreeSet<NodeAspect>,
}

impl Declared {
    /// What `edit` about the entity at `path` declares: `set`, `unset`, `append` and `remove` their field (a `field.entry` or `field[i].member` path's field), `link` and `unlink` their
    /// field, `retitle` the title, `add_verify` and `remove_verify` the obligations, `create` and `delete` their
    /// entity whole.
    pub(crate) fn edit(&mut self, edit: &Edit, path: &str) {
        match edit {
            Edit::Set { field, .. }
            | Edit::Unset { field, .. }
            | Edit::Link { field, .. }
            | Edit::Unlink { field, .. }
            | Edit::Append { field, .. }
            | Edit::Remove { field, .. } => self.field(path, super::path::top(field)),
            Edit::Retitle { .. } => self.aspect(path, NodeAspect::Title),
            Edit::AddVerify { .. } | Edit::RemoveVerify { .. } => {
                self.aspect(path, NodeAspect::Obligations);
            }
            Edit::Create { .. } | Edit::Delete { .. } => {
                self.whole.insert(path.to_string());
            }
        }
    }

    /// The entity at `old` is now at `new` (a move, ADR 0284): both ids are declared whole.
    pub(crate) fn moved(&mut self, old: &str, new: &str) {
        self.whole.insert(old.to_string());
        self.whole.insert(new.to_string());
    }

    /// A rename of the plan rewrites `files`.
    pub(crate) fn renamed<'a>(&mut self, files: impl IntoIterator<Item = &'a str>) {
        self.renamed.extend(files.into_iter().map(String::from));
    }

    /// `field` of `id` may change (also a delete's referrer, ADR 0163).
    pub(crate) fn field(&mut self, id: &str, field: &str) {
        self.changes
            .entry(id.to_string())
            .or_default()
            .fields
            .insert(field.to_string());
    }

    pub(crate) fn aspect(&mut self, id: &str, aspect: NodeAspect) {
        self.changes
            .entry(id.to_string())
            .or_default()
            .aspects
            .insert(aspect);
    }

    /// The part of `delta` nothing declares: a node added or removed that no create or delete names; a modified
    /// node's fields and aspects (its edges aside) that its edits do not name; an edge whose source has no declared
    /// field and neither of whose ends is created or deleted. Empty exactly when the edits change only what they say.
    pub(crate) fn beyond(&self, mut delta: GraphDelta) -> GraphDelta {
        delta.added_nodes.retain(|n| !self.whole.contains(&n.id));
        delta.removed_nodes.retain(|n| !self.whole.contains(&n.id));
        let no_change = Change::default();
        delta.modified_nodes.retain_mut(|node| {
            if self.whole.contains(&node.id) || self.renamed.contains(&node.file) {
                return false;
            }
            let declared = self.changes.get(&node.id).unwrap_or(&no_change);
            node.changed_fields.retain(|f| !declared.fields.contains(f));
            // the edges are judged one by one below
            node.changed_aspects
                .retain(|a| *a != NodeAspect::Edges && !declared.aspects.contains(a));
            !node.changed_fields.is_empty() || !node.changed_aspects.is_empty()
        });
        // shortcut: an edge is declared by its source's declared field, whatever its label (a derived field labels
        // it, ADR 0198 D4); upgrade if a stray edge from an edited entity is ever written.
        let declared_edge = |e: &specforge_graph::EdgeChange| {
            self.whole.contains(&e.source)
                || self.whole.contains(&e.target)
                || self
                    .changes
                    .get(&e.source)
                    .is_some_and(|c| !c.fields.is_empty())
        };
        delta.added_edges.retain(|e| !declared_edge(e));
        delta.removed_edges.retain(|e| !declared_edge(e));
        let mut files: BTreeSet<String> = BTreeSet::new();
        files.extend(delta.added_nodes.iter().map(|n| n.file.clone()));
        files.extend(delta.removed_nodes.iter().map(|n| n.file.clone()));
        files.extend(delta.modified_nodes.iter().map(|n| n.file.clone()));
        delta.affected_files = files.into_iter().collect();
        delta
    }
}

#[cfg(test)]
mod tests;
