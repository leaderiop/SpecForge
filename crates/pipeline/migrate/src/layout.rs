//! What a migration writes is laid out by the formatter (ADR 0235): each block its steps added or changed, with the
//! file's format configuration and the project's field forms, as a typed edit lays out the block it changes. Every
//! other byte is the steps'.

use specforge_formatter::{FormatConfig, format_range};
use specforge_graph::FieldForms;
use specforge_parser::{Entity, parse};
use std::collections::HashMap;

type Key = (String, String, usize);

/// `after`, what the migration steps made of `before`, with each changed block laid out by `format_range` under
/// `config` and `forms`. A block of `after` is changed when `before` has no block of its kind and id at the same rank
/// among the blocks of that kind and id (a block a step added: the `defaults` block), or has one whose lines differ
/// (a block the hoist shortened, a block the titles step titled). `after` itself when it equals `before` or does not
/// parse (the steps never write one that does not). Laying a block out changes only its own lines, so the blocks are
/// laid out from the last up and the rows of the ones above stay valid.
pub(crate) fn lay_out(
    before: &str,
    after: &str,
    config: &FormatConfig,
    forms: &FieldForms,
) -> String {
    if before == after {
        return after.to_string();
    }
    let file = parse(after, "layout.spec");
    if !file.errors.is_empty() {
        return after.to_string();
    }
    // `after` parses only when `before` did (both steps return a broken text unchanged), so `blocks` needs no error arm.
    let written = blocks(before);
    let lines: Vec<&str> = after.split_inclusive('\n').collect();
    let mut seen = HashMap::new();
    let changed: Vec<(usize, usize)> = file
        .entities
        .iter()
        .filter(|e| written.get(&key(e, &mut seen)) != Some(&text_of(&lines, e)))
        .map(|e| (e.span.start_line - 1, e.span.end_line - 1))
        .collect();
    changed
        .iter()
        .rev()
        .fold(after.to_string(), |text, &(first, last)| {
            format_range(&text, first, last, config, forms).formatted
        })
}

/// Each block of `text` by its key, holding its lines.
fn blocks(text: &str) -> HashMap<Key, String> {
    let file = parse(text, "layout.spec");
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut seen = HashMap::new();
    file.entities
        .iter()
        .map(|e| (key(e, &mut seen), text_of(&lines, e)))
        .collect()
}

/// A block's kind, id and rank among the blocks before it with that kind and id.
fn key(entity: &Entity, seen: &mut HashMap<(String, String), usize>) -> Key {
    let (kind, id) = (entity.kind.raw.to_string(), entity.id.raw.to_string());
    let rank = seen.entry((kind.clone(), id.clone())).or_insert(0);
    *rank += 1;
    (kind, id, *rank - 1)
}

/// The lines `entity`'s span covers, joined as written.
fn text_of(lines: &[&str], entity: &Entity) -> String {
    lines[entity.span.start_line - 1..entity.span.end_line].concat()
}
