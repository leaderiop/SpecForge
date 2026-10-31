//! Selectors over a graph (ADR 0290): the entities and the files a judged rule's input picks for one entity.
//! Resolution is type-directed like the registry's check (`reference_end`): a block field's next word is a member, a
//! reference followed by more words begins the next hop. Every reference is read as the full path it resolves to.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use serde_json::Value;
use specforge_graph::{FieldForms, Graph, Node, wire};
use specforge_protocol_types::{FieldType, FileBase};
use specforge_registry::FieldRegistry;
use specforge_registry::judged::{Chain, Head, Step};

/// A file a chain names, and the base its field reads it from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FileRef {
    pub path: String,
    /// Read from the project root (else the spec root).
    pub project: bool,
}

/// For a `(kind, field path)`: each target the field of a `kind` entity names, with the entities that name it.
pub(super) type Referrers = RefCell<BTreeMap<(String, String), Rc<BTreeMap<String, Vec<String>>>>>;

pub(super) struct Select<'a> {
    pub graph: &'a Graph,
    pub fields: &'a FieldRegistry,
    pub forms: &'a FieldForms,
    /// Built once per check and shared by every selector evaluation (not per judged entity).
    pub referrers: &'a Referrers,
}

impl Select<'_> {
    /// The entities `chain` picks from `this`, by path.
    pub fn entities(&self, this: &str, chain: &Chain) -> BTreeSet<String> {
        self.run(this, chain, false).0
    }

    /// The files `chain` names from `this`: its last step ends at a file field.
    pub fn files(&self, this: &str, chain: &Chain) -> BTreeSet<FileRef> {
        self.run(this, chain, true).1
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
                // A file field, or (ADR 0304) a string or string list field holding a path from the project root.
                if files
                    && let Some((declared, form)) = self.fields.get_path(kind, &rest)
                    && (declared.file_reference
                        || matches!(form.field_type(), FieldType::String | FieldType::StringList))
                {
                    named.extend(self.values_at(node, &rest).into_iter().map(|path| FileRef {
                        path,
                        project: !declared.file_reference || form.base() == FileBase::Project,
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
        let registries = compiled.environment().registries();
        let referrers = Referrers::default();
        let select = Select {
            graph: compiled.graph(),
            fields: registries.fields(),
            forms: registries.field_forms(),
            referrers: &referrers,
        };
        let Ok(Selector::Entities(chain)) = Selector::parse("self.phase<decision.phase") else {
            panic!("an entities selector")
        };
        let first = select.entities("v1.0/03/03-02", &chain);
        let second = select.entities("v1.0/03/03-03", &chain);
        assert_eq!(first, BTreeSet::from(["v1.0/03/D-01".to_string()]));
        assert_eq!(first, second);
        assert_eq!(referrers.borrow().len(), 1, "one index serves every entity");
    }
}
