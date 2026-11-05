//! The one selector evaluator (ADR 0290, ADR 0330): the entities and the files a selector chain picks from one entity,
//! for a judged rule's inputs and for a trace route's steps. Resolution is type-directed like the registry's check
//! (`reference_end`): a block field's next word is a member, a reference followed by more words begins the next hop.
//! Every reference is read as the full path it resolves to.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use serde_json::Value;
use specforge_graph::{FieldForms, Graph, Node, wire};
use specforge_protocol_types::{FieldType, Root};
use specforge_registry::judged::{Chain, Head, Step};
use specforge_registry::{CheckRegistry, FieldRegistry, RegistryBuild};

/// A file a chain names as written, and the root a bare path starts at: a `<root>/` prefix in `path` overrides it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FileRef {
    pub root: Root,
    pub path: String,
}

/// For a `(kind, field path)`: each target the field of a `kind` entity names, with the entities that name it.
type Referrers = RefCell<BTreeMap<(String, String), Rc<BTreeMap<String, Vec<String>>>>>;

/// Selector chains evaluated over one graph. Its referrer index is built on first use and shared by every chain it
/// evaluates, so one `Selection` serves a whole check or a whole trace request, never one per entity.
pub struct Selection<'a> {
    graph: &'a Graph,
    fields: &'a FieldRegistry,
    forms: &'a FieldForms,
    checks: &'a CheckRegistry,
    referrers: Referrers,
}

impl<'a> Selection<'a> {
    /// A selection over `graph`, read through the fields and forms `registries` declare.
    pub fn of(graph: &'a Graph, registries: &'a RegistryBuild) -> Self {
        Self {
            graph,
            fields: registries.fields(),
            forms: registries.field_forms(),
            checks: registries.checks(),
            referrers: Referrers::default(),
        }
    }

    /// The entities `chain` picks from `this`, by path.
    pub fn entities(&self, this: &str, chain: &Chain) -> BTreeSet<String> {
        self.run(this, chain, false).0
    }

    /// The files `chain` names from `this`: its last step ends at a file field.
    pub(crate) fn files(&self, this: &str, chain: &Chain) -> BTreeSet<FileRef> {
        self.run(this, chain, true).1
    }

    /// The ids written in the last step's field of the entities before it that name no fact (ADR 0327 D6): what
    /// `chain` would pick from `this` were they held. Only a last `.field` step names any.
    // shortcut: a last step of one field only; a path that goes on past a reference (`a.b`) finds none of `b`'s.
    pub fn unheld(&self, this: &str, chain: &Chain) -> BTreeSet<String> {
        let Some((Step::Path(field), before)) = chain.steps.split_last() else {
            return BTreeSet::new();
        };
        let holders = Chain {
            head: chain.head.clone(),
            steps: before.to_vec(),
        };
        self.entities(this, &holders)
            .iter()
            .filter_map(|id| self.graph.node(id))
            .flat_map(|node| &node.unheld)
            .filter(|unheld| unheld.field.as_str() == field)
            .map(|unheld| unheld.id.clone())
            .collect()
    }

    fn run(&self, this: &str, chain: &Chain, files: bool) -> (BTreeSet<String>, BTreeSet<FileRef>) {
        let mut set: BTreeSet<String> = match &chain.head {
            Head::This => BTreeSet::from([this.to_string()]),
            Head::Kinds(kinds) => kinds
                .iter()
                .flat_map(|k| self.graph.nodes_by_kind(k))
                .filter(|n| self.graph.scopes().scope(n.id.raw.as_str()).is_none())
                .map(|n| n.id.raw.to_string())
                .collect(),
        };
        let mut named = BTreeSet::new();
        for (at, step) in chain.steps.iter().enumerate() {
            let last = at + 1 == chain.steps.len();
            match step {
                Step::Path(path) => {
                    let (next, found) = self.follow(&set, path, files && last);
                    set = next;
                    named = found;
                }
                Step::Referrers { kinds, path } => {
                    // The path may go on past its reference (ADR 0304): the prefix finds the referrers, the rest
                    // is a forward path from each.
                    let words: Vec<&str> = path.split('.').collect();
                    let end = kinds
                        .first()
                        .and_then(|k| self.reference_end(k, &words))
                        .unwrap_or(words.len());
                    let prefix = words[..end].join(".");
                    set = kinds
                        .iter()
                        .map(|k| self.index(k, &prefix))
                        .flat_map(|index| {
                            set.iter()
                                .filter_map(|target| index.get(target).cloned())
                                .flatten()
                                .collect::<Vec<_>>()
                        })
                        .collect();
                    if end < words.len() {
                        let (next, found) =
                            self.follow(&set, &words[end..].join("."), files && last);
                        set = next;
                        named = found;
                    }
                }
            }
        }
        (set, named)
    }

    /// One `.path` step from `from`: a reference after a reference is the next hop. With `files`, the hop that ends
    /// at a file field names its files and reaches no entity.
    fn follow(
        &self,
        from: &BTreeSet<String>,
        path: &str,
        files: bool,
    ) -> (BTreeSet<String>, BTreeSet<FileRef>) {
        let words: Vec<&str> = path.split('.').collect();
        let mut set = from.clone();
        let mut at = 0;
        while at < words.len() {
            let mut next = BTreeSet::new();
            let mut named = BTreeSet::new();
            let mut taken = 0;
            for id in &set {
                let Some(node) = self.graph.node(id) else {
                    continue;
                };
                let kind = node.kind.raw.as_str();
                let rest = words[at..].join(".");
                // A string or string list field holding paths (ADR 0304): from the root its path check gives, else
                // from the project root.
                if files
                    && let Some((_, form)) = self.fields.get_path(kind, &rest)
                    && matches!(form.field_type(), FieldType::String | FieldType::StringList)
                {
                    let root = self.checks.path_root(kind, &rest).unwrap_or(Root::Project);
                    named.extend(self.values_at(node, &rest).into_iter().map(|path| FileRef {
                        root: root.clone(),
                        path,
                    }));
                    taken = words.len() - at;
                    continue;
                }
                let Some(end) = self.reference_end(kind, &words[at..]) else {
                    continue;
                };
                taken = taken.max(end);
                next.extend(
                    self.values_at(node, &words[at..at + end].join("."))
                        .into_iter()
                        .filter(|target| self.graph.node(target).is_some()),
                );
            }
            if taken == 0 {
                return (BTreeSet::new(), BTreeSet::new());
            }
            at += taken;
            if files && at >= words.len() && !named.is_empty() {
                return (BTreeSet::new(), named);
            }
            set = next;
        }
        (set, BTreeSet::new())
    }

    /// How many of `words` end the first reference of `kind` (a block field takes the member words after it).
    fn reference_end(&self, kind: &str, words: &[&str]) -> Option<usize> {
        for end in 1..=words.len() {
            let (_, form) = self.fields.get_path(kind, &words[..end].join("."))?;
            if form.field_type().is_reference() {
                return Some(end);
            }
            if !form.field_type().is_block() {
                return None;
            }
        }
        None
    }

    /// The referrer index of `path` over the entities of `kind`, built on first use.
    fn index(&self, kind: &str, path: &str) -> Rc<BTreeMap<String, Vec<String>>> {
        let key = (kind.to_string(), path.to_string());
        if let Some(held) = self.referrers.borrow().get(&key) {
            return Rc::clone(held);
        }
        let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for node in self.graph.nodes_by_kind(kind) {
            for value in self.values_at(node, path) {
                index
                    .entry(value)
                    .or_default()
                    .push(node.id.raw.to_string());
            }
        }
        let index = Rc::new(index);
        self.referrers.borrow_mut().insert(key, Rc::clone(&index));
        index
    }

    /// The strings the field or member `path` of `node` holds, a reference as the full path of its target.
    fn values_at(&self, node: &Node, path: &str) -> Vec<String> {
        let words: Vec<&str> = path.split('.').collect();
        let Some(first) = wire::resolved(self.graph, self.forms, node, words[0]) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        pick(first.as_value(), &words[1..], &mut out);
        out
    }
}

fn pick(value: &Value, words: &[&str], out: &mut Vec<String>) {
    match (value, words.split_first()) {
        (Value::String(text), None) => out.push(text.clone()),
        (Value::Array(items), _) => items.iter().for_each(|i| pick(i, words, out)),
        (Value::Object(map), Some((word, rest))) => {
            if let Some(inner) = map.get(*word) {
                pick(inner, rest, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{gsd_judged_project, gsd_judged_runtime};
    use specforge_registry::judged::Selector;
    use std::sync::Arc;

    #[test]
    fn referrer_steps_share_one_index_per_check() {
        let project = gsd_judged_project("");
        let compiled = project.compile_in(Arc::new(gsd_judged_runtime()) as crate::SharedRuntime);
        let select = Selection::of(compiled.graph(), compiled.environment().registries());
        let Ok(Selector::Entities(chain)) = Selector::parse("self.phase<decision.phase") else {
            panic!("an entities selector")
        };
        let first = select.entities("v1.0/03/03-02", &chain);
        let second = select.entities("v1.0/03/03-03", &chain);
        assert_eq!(first, BTreeSet::from(["v1.0/03/D-01".to_string()]));
        assert_eq!(first, second);
        assert_eq!(
            select.referrers.borrow().len(),
            1,
            "one index serves every entity"
        );
    }
}
