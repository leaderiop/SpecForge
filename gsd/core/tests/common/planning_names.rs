//! The names ledger (ADR 0332 D8): own text that names a GSD file (`gsd_kit::files::PLANNING_FILES`) outside an
//! upstream quote is a row of `planning_names.txt`, `<source>\t<trimmed line>`. A source is an asset's lock key (its
//! own spans), `agent:<id>` (an agent's recording section), `prompt:<id>` (a core prompt's Rust-authored lines) or
//! `security-prompt:<id>`. An upstream quote is a kept span, or a link into the tracked release's blobs, exempt whole.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

use std::collections::BTreeMap;

use super::ledger;

/// The ledger's rows.
pub const PLANNING_NAMES: &str = include_str!("planning_names.txt");

/// What a link into the tracked release's blobs starts with: upstream's text, quoted by its address.
pub fn upstream_blob() -> String {
    format!(
        "https://github.com/open-gsd/gsd-core/blob/v{}/",
        gsd_kit::tracked_version!()
    )
}

/// The 0-based lines of `text` that name a GSD file outside a link into the tracked release's blobs, each with the
/// rows it names (`.planning` among them).
pub fn naming_lines(text: &str) -> BTreeMap<usize, Vec<&'static str>> {
    let blob = upstream_blob();
    let quoted: Vec<std::ops::Range<usize>> = gsd_kit::markdown::links(text)
        .into_iter()
        .filter(|(_, dest)| dest.starts_with(&blob))
        .map(|(range, _)| range)
        .collect();
    let mut out: BTreeMap<usize, Vec<&'static str>> = BTreeMap::new();
    for (range, row) in gsd_kit::files::planning_names(text) {
        if quoted
            .iter()
            .any(|q| q.start <= range.start && range.end <= q.end)
        {
            continue;
        }
        let line = text[..range.start].matches('\n').count();
        out.entry(line).or_default().push(row);
    }
    out
}

/// The hits of the lines of `text` that name a GSD file, as `(source, trimmed line)` rows. A `.planning` hit fails:
/// no own text names the folder, and no ledger row may hold it.
pub fn hits(source: &str, text: &str, keep: impl Fn(usize) -> bool) -> Vec<Vec<String>> {
    let lines: Vec<&str> = text.split('\n').collect();
    naming_lines(text)
        .into_iter()
        .filter(|(n, _)| keep(*n))
        .map(|(n, rows)| {
            assert!(
                !rows.contains(&".planning"),
                "{source}: own text names `.planning`: {}",
                lines[n].trim()
            );
            vec![source.to_string(), lines[n].trim().to_string()]
        })
        .collect()
}

/// The ledger's rows whose source `keep` takes; none may hold `.planning`.
pub fn rows(keep: impl Fn(&str) -> bool) -> Vec<Vec<String>> {
    let rows = ledger::rows(PLANNING_NAMES, 2);
    for row in &rows {
        assert!(
            !row[1].contains(".planning"),
            "a planning_names.txt row holds `.planning`: {}",
            row.join("\t")
        );
    }
    rows.into_iter().filter(|row| keep(&row[0])).collect()
}

/// Fails unless `hits` are the ledger's rows that `keep` takes.
pub fn assert_listed(hits: &[Vec<String>], keep: impl Fn(&str) -> bool) {
    ledger::assert_rows(
        "planning_names.txt",
        "name the entity, not the GSD file",
        hits,
        rows(keep),
    );
}
