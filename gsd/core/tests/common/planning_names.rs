//! Own text names no GSD file (ADR 0332 D8): no own line names one of `gsd_kit::files::PLANNING_FILES` outside an
//! upstream quote. A hit is `<source>\t<trimmed line>`; a source is an asset's lock key (its own spans), `agent:<id>`
//! (an agent's recording section), `prompt:<id>` (a core prompt's Rust-authored lines) or `security-prompt:<id>`. An
//! upstream quote is a kept span, or a link into the tracked release's blobs, exempt whole.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

use std::collections::BTreeMap;

/// The 0-based lines of `text` that name a GSD file outside a link into the tracked release's blobs, each with the
/// rows it names (`.planning` among them).
pub fn naming_lines(text: &str) -> BTreeMap<usize, Vec<&'static str>> {
    let quoted: Vec<std::ops::Range<usize>> = gsd_kit::markdown::links(text)
        .into_iter()
        .filter(|(_, dest)| dest.starts_with(gsd_kit::files::UPSTREAM_BLOB))
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
/// no own text names the folder.
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

/// Fails when own text names a GSD file: each hit is a line to rewrite.
pub fn assert_none(hits: &[Vec<String>]) {
    assert!(
        hits.is_empty(),
        "own text names a GSD file (name the entity or the command, not the file):\n{}",
        hits.iter()
            .map(|row| row.join("\t"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
