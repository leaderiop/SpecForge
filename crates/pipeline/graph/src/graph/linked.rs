//! Which of an entity's fields have their link (ADR 0232): the one answer the trace's missing
//! links and the outgoing edge rules read.

use std::collections::BTreeSet;

use specforge_common::Sym;

use super::{Graph, Node};
use crate::FieldForms;

impl Graph {
    /// The fields `node` gives their link: each it writes a reference in, as
    /// [`Reading::references`](crate::Reading::references) reads the stored entry (an inherited
    /// `defaults` value is written, an empty list is not), resolved or not, for a reference that
    /// does not resolve is the linker's E003 or I004 and never a missing link; and the label of
    /// each edge from it (a derived field writes nothing and has its edges, ADR 0204).
    pub fn linked_fields(&self, forms: &FieldForms, node: &Node) -> BTreeSet<Sym> {
        let kind = node.kind.raw.as_str();
        // A reference inside a typed block gives its member path the link (ADR 0285).
        let written = node.fields.entries().iter().flat_map(|top| {
            forms
                .readings(kind, top)
                .into_iter()
                .filter(|(_, reading, entry)| !reading.references(entry).is_empty())
                .map(|(path, _, _)| Sym::new(&path.declared()))
        });
        written
            .chain(
                self.edges_from(node.id.raw.as_str())
                    .into_iter()
                    .map(|edge| edge.label),
            )
            .collect()
    }
}
