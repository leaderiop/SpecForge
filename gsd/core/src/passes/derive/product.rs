//! The computed values of the product kinds (`spec/product.md`, ADR 0357): a term's `used_in`, the entities whose
//! prose holds its name or one of its aliases (`passes::vocabulary`), and a journey's `last_uat`, the latest session of
//! a UAT test that names it.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::plans::{link, of_kind, strings, text};
use crate::passes::vocabulary::{prose, uses, writers};

/// The product kinds' computed values.
pub(crate) fn values(input: &PassInput) -> Vec<PassValue> {
    let mut out = used_in(input);
    out.extend(last_uat(input));
    out
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

/// Each journey a UAT test names (`uat_test.journey`): its `last_uat`, the session of such a test that started last. A
/// session with no `started` is older than every dated one, and of two that started at once the greater id is the
/// later. Any result counts: the session's own `status` says how it went.
fn last_uat(input: &PassInput) -> Vec<PassValue> {
    let mut latest: BTreeMap<&str, (Option<i64>, &str)> = BTreeMap::new();
    for test in of_kind(input, "uat_test") {
        let (Some(journey), Some(session)) =
            (link(input, test, "journey"), link(input, test, "session"))
        else {
            continue;
        };
        let started = input
            .entities
            .iter()
            .find(|e| e.id == session)
            .and_then(|s| text(s, "started"))
            .and_then(crate::time::seconds);
        let candidate = (started, session);
        let best = latest.entry(journey).or_insert(candidate);
        *best = (*best).max(candidate);
    }
    latest
        .into_iter()
        .map(|(journey, (_, session))| value(journey, "last_uat", session))
        .collect()
}

#[cfg(test)]
mod tests;
