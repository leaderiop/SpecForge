//! Selectors over a graph (ADR 0290): the entities and the files a judged rule's input picks for one entity.
//! Resolution is type-directed like the registry's check (`reference_end`): a block field's next word is a member, a
//! reference followed by more words begins the next hop. Every reference is read as the full path it resolves to.

use std::collections::BTreeSet;

use serde_json::Value;
use specforge_graph::{FieldForms, Graph, Node, wire};
use specforge_protocol_types::FileBase;
use specforge_registry::FieldRegistry;
use specforge_registry::judged::{Chain, Head, Step};

/// A file a chain names, and the base its field reads it from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FileRef {
    pub path: String,
    /// Read from the project root (else the spec root).
    pub project: bool,
}

pub(super) struct Select<'a> {
    pub graph: &'a Graph,
    pub fields: &'a FieldRegistry,
    pub forms: &'a FieldForms,
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
                    set = kinds
                        .iter()
                        .flat_map(|k| self.graph.nodes_by_kind(k))
                        .filter(|n| self.values_at(n, path).iter().any(|v| set.contains(v)))
                        .map(|n| n.id.raw.to_string())
                        .collect();
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
                if files
                    && let Some((declared, form)) = self.fields.get_path(kind, &rest)
                    && declared.file_reference
                {
                    named.extend(self.values_at(node, &rest).into_iter().map(|path| FileRef {
                        path,
                        project: form.base() == FileBase::Project,
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
