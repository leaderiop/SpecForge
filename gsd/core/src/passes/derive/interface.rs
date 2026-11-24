//! A milestone's `api_breaks` (ADR 0390 D4): a selection of the `@code/core` `api_item` facts, which the API packs
//! classified against the committed accepted report. Each fact whose `change` is `changed`, `narrowed` or `removed`
//! is one `<change> <id>` line, sorted by id, at most [`LINES`] then one `more <N>` line; `added`, `unchanged` and any
//! later value never count. Written on each milestone neither shipped nor abandoned, and only when some fact carries
//! a `change` at all: absent means never observed, empty means clean. It reflects the facts of the last collect;
//! `gsd/E533` reads it at the close.

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::one_line;
use crate::passes::plans::{of_kind, text};

/// `@code/core`'s kind, named once as `thread.rs` names `test`.
pub const API_ITEM: &str = "api_item";
const LINES: usize = 50;

/// How the packs judged an item against the accepted report; `None` when no accepted report covered it.
fn change(item: &PassEntity) -> Option<&str> {
    text(item, "change")
}

pub fn values(input: &PassInput) -> Vec<PassValue> {
    let items: Vec<&PassEntity> = of_kind(input, API_ITEM).collect();
    if !items.iter().any(|i| change(i).is_some()) {
        return Vec::new();
    }
    let mut broken: Vec<(&str, &str)> = items
        .iter()
        .filter_map(|i| {
            change(i)
                .filter(|c| matches!(*c, "changed" | "narrowed" | "removed"))
                .map(|c| (i.id.as_str(), c))
        })
        .collect();
    broken.sort();
    let more = broken.len().saturating_sub(LINES);
    let mut lines: Vec<String> = broken
        .iter()
        .take(LINES)
        .map(|(id, c)| format!("{c} {}", one_line(id)))
        .collect();
    if more > 0 {
        lines.push(format!("more {more}"));
    }
    of_kind(input, "milestone")
        .filter(|m| !m.fields.contains_key("abandoned") && !m.fields.contains_key("completion"))
        .map(|m| value(&m.id, "api_breaks", lines.clone()))
        .collect()
}

#[cfg(test)]
mod tests;
