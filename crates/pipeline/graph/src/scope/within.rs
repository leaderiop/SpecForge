//! What a scope holds, at any depth (ADR 0291): the entities a gate scoped to an entity looks at.

use super::Scopes;
use specforge_common::Sym;

impl Scopes {
    /// The entities placed inside the scope of the entity at `path`, at any depth (its members, their members, and
    /// so on), sorted; empty when `path` scopes nothing or is not placed. `path` itself is not among them.
    pub fn within(&self, path: &str) -> Vec<Sym> {
        let mut found = Vec::new();
        let mut open = vec![Sym::new(path)];
        while let Some(scope) = open.pop() {
            let Some(members) = self.members.get(&Some(scope)) else {
                continue;
            };
            for &member in members.values() {
                // A scope cycle is refused when the graph is built; the guard keeps a malformed index finite.
                if !found.contains(&member) && member != Sym::new(path) {
                    found.push(member);
                    open.push(member);
                }
            }
        }
        found.sort();
        found
    }
}
