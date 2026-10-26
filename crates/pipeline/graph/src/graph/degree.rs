//! The edges that link an entity to *other* entities, and how many (CONTEXT.md "Unconnected entity", ADR 0270).

use super::{Direction, Edge, Graph};
use specforge_common::Sym;

/// An entity's edges to and from *other* entities. An edge from the entity
/// to itself is in neither count; the graph holds no edge to a reference
/// that does not resolve (E003, I004 report those), so none is counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Degree {
    /// Edges from another entity to this one.
    pub incoming: usize,
    /// Edges from this entity to another one.
    pub outgoing: usize,
}

impl Degree {
    /// Every edge linking the entity to another one, both ways.
    pub fn total(self) -> usize {
        self.incoming + self.outgoing
    }

    /// No edge links the entity to another entity.
    pub fn is_unconnected(self) -> bool {
        self.total() == 0
    }

    /// How many more entities it references than reference it
    /// (`outgoing - incoming`): the starting points' rank.
    pub fn lead(self) -> isize {
        self.outgoing as isize - self.incoming as isize
    }
}

impl Graph {
    /// The edges linking `id` to another entity, one way, in the order the graph holds them. An edge from `id`
    /// to itself links it to nothing else and is none of them (CONTEXT.md "Unconnected entity", ADR 0270);
    /// the graph still holds it, for the cycle rules and W061. The one self-edge rule: the degree counts these
    /// edges and the entity snapshot types and counts them.
    pub fn edges_with_others(
        &self,
        id: &str,
        direction: Direction,
    ) -> impl Iterator<Item = &Edge> + '_ {
        let index = match direction {
            Direction::Incoming => &self.target_index,
            Direction::Outgoing => &self.source_index,
        };
        index
            .get(&Sym::new(id))
            .into_iter()
            .flatten()
            .map(|&i| &self.edges[i])
            .filter(|edge| edge.source != edge.target)
    }

    /// `id`'s degree, from the graph's index: O(its edges). Counts the edges at `id` whether or not a node declares
    /// it (the zero degree when there are none).
    pub fn degree(&self, id: &str) -> Degree {
        Degree {
            incoming: self.edges_with_others(id, Direction::Incoming).count(),
            outgoing: self.edges_with_others(id, Direction::Outgoing).count(),
        }
    }
}
