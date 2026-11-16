//! An invariant's obligations as proofs (ADR 0360 D6): the tests whose spec-link attributes state them. An attribute
//! naming the invariant with an obligation's `verify` text is joined to the recorded `@rust/source` test item that
//! names the invariant at the same file and line (the line of the id's string); its `test_id` is the proof.

use specforge_protocol_types::Fact;
use std::collections::{BTreeMap, BTreeSet};
use xtask::links::Link;

/// A recorded test item: its test id, and the places (file, line) each entity name its attributes give is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub test_id: String,
    pub names: BTreeMap<String, Vec<(String, usize)>>,
}

/// What proofs are read from: the source's spec-link attributes and its recorded test items.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Proofs {
    pub links: Vec<Link>,
    pub items: Vec<Item>,
}

/// What one obligation is proved by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proved {
    /// The test ids of the items whose attributes state it, sorted, each once.
    Tests(Vec<String>),
    /// No test item states it: the attributes that do (`file:line`), none when no attribute does.
    Unproven(Vec<String>),
}

/// The test items among recorded `item` facts: those with a `test_id`.
pub fn items(facts: &[Fact]) -> Vec<Item> {
    facts
        .iter()
        .filter(|fact| fact.kind == "item")
        .filter_map(|fact| {
            let test_id = fact.fields.get("test_id")?.as_str()?.to_string();
            let names = fact
                .named_at
                .get("names")
                .into_iter()
                .flatten()
                .map(|(id, places)| {
                    let places = places
                        .iter()
                        .map(|at| (at.file.clone(), at.start_line))
                        .collect();
                    (id.clone(), places)
                })
                .collect();
            Some(Item { test_id, names })
        })
        .collect()
}

impl Proofs {
    /// What proves the obligation `text` of `invariant`.
    pub fn of(&self, invariant: &str, text: &str) -> Proved {
        let stating: Vec<&Link> = self
            .links
            .iter()
            .filter(|link| link.id == invariant && link.verify.as_deref() == Some(text))
            .collect();
        let tests: BTreeSet<&str> = self
            .items
            .iter()
            .filter(|item| {
                item.names.get(invariant).is_some_and(|places| {
                    stating.iter().any(|link| {
                        places
                            .iter()
                            .any(|(file, line)| *file == link.file && *line == link.id_line)
                    })
                })
            })
            .map(|item| item.test_id.as_str())
            .collect();
        if tests.is_empty() {
            Proved::Unproven(
                stating
                    .iter()
                    .map(|link| format!("{}:{}", link.file, link.line))
                    .collect(),
            )
        } else {
            Proved::Tests(tests.into_iter().map(String::from).collect())
        }
    }
}
