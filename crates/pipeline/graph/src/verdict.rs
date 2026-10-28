//! A verdict as every output reads it (ADR 0290): the rule, the outcome and the block's members written, no
//! position. An entity's `verdict` statements are its own member (`Node::verdicts`), never a field; the graph delta
//! compares them through this form, so a verdict that only moved is not modified.

use std::collections::BTreeMap;

use serde::Serialize;
use specforge_parser::{VerdictMember, Written};

use crate::field_form::{FieldJson, write_fields};

/// One `verdict <rule> <outcome> { … }` statement without positions.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VerdictJson {
    pub rule: String,
    pub outcome: String,
    /// The block's members by key, each written as a field's value is (a key written twice is its first).
    pub members: BTreeMap<String, FieldJson>,
}

impl VerdictJson {
    pub fn of(verdict: &VerdictMember) -> Self {
        Self {
            rule: verdict.rule.clone(),
            outcome: verdict.outcome.clone(),
            members: write_fields(&verdict.body),
        }
    }

    /// A string member (`judge`, `model`, `at`), when it is one.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.members.get(key)?.as_value().as_str()
    }
}

/// The bare words of a verdict's `cited` list: the entities it names, for the finder (a rename respells them) and
/// never an edge. A quoted item is a path.
pub(crate) fn cited_entities(
    verdict: &VerdictMember,
) -> impl Iterator<Item = &specforge_parser::ListItem> {
    let items = match verdict.body.get("cited") {
        Some(specforge_parser::FieldValue::List(items)) => items.as_slice(),
        _ => &[],
    };
    items
        .iter()
        .filter(|item| matches!(item.written, Written::Bare))
}
