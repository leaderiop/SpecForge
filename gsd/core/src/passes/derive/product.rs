//! The computed values of the product kinds (`spec/product.md`, ADR 0357): a term's `used_in`, the entities whose
//! prose holds its name or one of its aliases (`passes::vocabulary`).

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::plans::{of_kind, strings, text};
use crate::passes::vocabulary::{prose, uses, writers};

/// The product kinds' computed values.
pub(crate) fn values(input: &PassInput) -> Vec<PassValue> {
    used_in(input)
}

/// Each term's `used_in`: the ids of the entities whose prose holds its `name` or an alias as a whole word or phrase,
/// sorted, and `[]` when none, so "unused" is never "unknown". An `avoid` word is no use.
fn used_in(input: &PassInput) -> Vec<PassValue> {
    let written: Vec<(&str, Vec<(String, String)>)> =
        writers(input).map(|e| (e.id.as_str(), prose(e))).collect();
    of_kind(input, "term")
        .map(|term| {
            let words: Vec<String> = text(term, "name")
                .map(str::to_string)
                .into_iter()
                .chain(strings(term, "aliases"))
                .collect();
            let mut ids: Vec<&str> = written
                .iter()
                .filter(|(_, places)| words.iter().any(|w| uses(places, w).is_some()))
                .map(|(id, _)| *id)
                .collect();
            ids.sort_unstable();
            ids.dedup();
            value(&term.id, "used_in", ids)
        })
        .collect()
}

#[cfg(test)]
mod tests;
