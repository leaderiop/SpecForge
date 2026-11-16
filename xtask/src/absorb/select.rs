//! A sample a selector computes from the old graph (ADR 0360): `--only sample:<name>` converts the entities its rule
//! selects, so the sample follows the spec as it is, never a list kept beside it.
//!
//! A [`Sample`] names seed files (every entity in them), `through` files (their entities are referral targets, never
//! sampled), the kinds that join when one of their outgoing edges ends at a seed or a `through` entity (one step), and
//! whole files (every entity, drawing no referrer). Then a `term` joins when its title or one of its aliases occurs in
//! the prose of a sampled entity of another kind: the values of the fields the old registry declares `string` or
//! `string_list`, each matched alone, as a whole word or phrase, in any case, Unicode-aware. A term never draws a
//! term.

use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;
use serde_json::Value;
use specforge_graph::field_form;
use specforge_registry::FieldType;

use super::read::Old;
use super::scope::Scope;

/// The kind whose entities the prose step selects.
const TERM: &str = "term";

/// A sample's rule, in project-relative paths as the graph's spans hold them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub name: &'static str,
    /// Files every entity of which is sampled, and draws its referrers.
    pub seeds: &'static [&'static str],
    /// Path patterns (`*` within a segment) whose entities draw their referrers but are not sampled.
    pub through: &'static [&'static str],
    /// The kinds an entity of which joins when an outgoing edge ends at a seed or a `through` entity.
    pub referrers: &'static [&'static str],
    /// Files every entity of which is sampled, drawing no referrer.
    pub whole: &'static [&'static str],
}

/// The samples `--only sample:<name>` names.
pub const SAMPLES: &[Sample] = &[Sample {
    name: "wasm-host",
    seeds: &["spec/features/wasm.spec", "spec/invariants/wasm.spec"],
    through: &["spec/behaviors/wasm-*.spec"],
    referrers: &["failure_mode", "constraint", "decision", "journey"],
    whole: &["spec/product/personas.spec"],
}];

/// The sample `name` names; refused naming every sample.
pub fn sample(name: &str) -> Result<&'static Sample, String> {
    SAMPLES.iter().find(|s| s.name == name).ok_or_else(|| {
        let names: Vec<&str> = SAMPLES.iter().map(|s| s.name).collect();
        format!("no sample '{name}'; the samples are: {}", names.join(", "))
    })
}

/// What a sample selected: old kind → ids, both sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection(pub BTreeMap<String, BTreeSet<String>>);

impl Selection {
    /// The scope that converts exactly the selected ids.
    pub fn scope(&self) -> Scope {
        Scope::Only(self.0.values().flatten().cloned().collect())
    }

    /// Its pretty JSON, `{kind: [id, ...]}` sorted by kind then id, with a final newline.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(&self.0).expect("a selection serializes");
        text.push('\n');
        text
    }
}

/// The entities of `old` that `sample`'s rule selects.
pub fn select(old: &Old, sample: &Sample) -> Selection {
    let graph = old.project.graph();
    let nodes = graph.nodes();
    let in_any = |file: &str, files: &[&str]| files.contains(&file);
    let through = |file: &str| {
        sample
            .through
            .iter()
            .any(|p| gate::Pattern(p).matches(file))
    };
    let mut selected: BTreeMap<&str, &str> = BTreeMap::new();
    let mut targets: BTreeSet<&str> = BTreeSet::new();
    for node in &nodes {
        let file = node.source_span.file.as_str();
        if in_any(file, sample.seeds) {
            selected.insert(node.id.raw.as_str(), node.kind.raw.as_str());
            targets.insert(node.id.raw.as_str());
        } else if through(file) {
            targets.insert(node.id.raw.as_str());
        } else if in_any(file, sample.whole) {
            selected.insert(node.id.raw.as_str(), node.kind.raw.as_str());
        }
    }
    for node in &nodes {
        let kind = node.kind.raw.as_str();
        if sample.referrers.contains(&kind)
            && graph
                .edges_from(node.id.raw.as_str())
                .iter()
                .any(|edge| targets.contains(edge.target.as_str()))
        {
            selected.insert(node.id.raw.as_str(), kind);
        }
    }
    let prose: Vec<String> = nodes
        .iter()
        .filter(|node| {
            node.kind.raw.as_str() != TERM && selected.contains_key(node.id.raw.as_str())
        })
        .flat_map(|node| prose_of(old, node))
        .collect();
    // Each text alone: a phrase split between two fields, or two entities, is named by no text.
    for node in nodes.iter().filter(|node| node.kind.raw.as_str() == TERM) {
        if names_of(node)
            .iter()
            .filter_map(|name| pattern(name))
            .any(|re| prose.iter().any(|text| re.is_match(text)))
        {
            selected.insert(node.id.raw.as_str(), TERM);
        }
    }
    let mut selection = Selection::default();
    for (id, kind) in selected {
        selection
            .0
            .entry(kind.to_string())
            .or_default()
            .insert(id.to_string());
    }
    selection
}

/// The texts of `node`'s fields the old registry declares `string` or `string_list`.
fn prose_of(old: &Old, node: &specforge_graph::Node) -> Vec<String> {
    let fields = old.project.environment().registries().fields();
    node.fields
        .entries()
        .iter()
        .filter(|entry| {
            fields
                .get(node.kind.raw.as_str(), entry.key.as_str())
                .is_some_and(|f| {
                    matches!(f.field_type(), FieldType::String | FieldType::StringList)
                })
        })
        .flat_map(|entry| strings(field_form::write(&entry.value).as_value()))
        .collect()
}

/// Every string in `value`.
fn strings(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) => vec![text.clone()],
        Value::Array(items) => items.iter().flat_map(strings).collect(),
        _ => Vec::new(),
    }
}

/// A term's names: its title and its aliases. Its id is not one (a word in prose is no id).
fn names_of(node: &specforge_graph::Node) -> Vec<String> {
    let aliases = node
        .fields
        .entries()
        .iter()
        .filter(|entry| entry.key.as_str() == "aliases")
        .flat_map(|entry| strings(field_form::write(&entry.value).as_value()))
        .collect::<Vec<_>>();
    node.title.iter().cloned().chain(aliases).collect()
}

/// The pattern of `name` as a whole word or phrase, in any case, its words apart by any whitespace (a phrase may wrap
/// a line of one text); None for a name of no word.
fn pattern(name: &str) -> Option<Regex> {
    let words: Vec<String> = name.split_whitespace().map(regex::escape).collect();
    if words.is_empty() {
        return None;
    }
    Regex::new(&format!(r"(?i)(?:^|\W){}(?:\W|$)", words.join(r"\s+"))).ok()
}

#[cfg(test)]
#[path = "select/tests.rs"]
mod tests;
