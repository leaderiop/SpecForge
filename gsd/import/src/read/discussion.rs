//! `NN-DISCUSSION-LOG.md` to a `discussion` (13-RESEARCH Part B, the DISCUSSION-LOG rows; the rows are
//! `table::sections::DISCUSSION_LOG`): the file stem is its id, and each `## <Area>` one of its `entries`, the area's
//! Option column its `options`, the row marked ✓ (else `**User's choice:**`) its `chosen`, `**Notes:**` its `notes`.
//! The header, the note under the title and the sections CONTEXT.md gives are reported.

use std::collections::BTreeMap;

use gsd_kit::markdown::{self, Table};
use serde_json::{Value, json};

use super::context::{Text, discretion};
use super::{Context, PhaseFile, is_footer, labelled, phase_file};
use crate::records::Records;
use crate::report::Report;
use crate::sections::{self, Section};

/// The fate table's name of a discussion log.
pub const DL: &str = "NN-DISCUSSION-LOG.md";

const HEADER: &str = "**Date:**, **Phase:**, **Areas discussed:**";
const UNKNOWN: &str = "a section no row of the NN-DISCUSSION-LOG.md table names: no field holds it";

/// Reads the discussion log `text`, served at `path` (`rel` under `.planning/`): `phases/<NN-slug>/<NN>-DISCUSSION-LOG.md`.
pub fn read(
    path: &str,
    rel: &str,
    text: &str,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let Some(f) = phase_file(path, rel, DL, cx, report) else {
        return;
    };
    let t = Text::new(text);
    for (n, line) in t.raw() {
        if ["**Date:**", "**Phase:**", "**Areas discussed:**"]
            .iter()
            .any(|h| line.starts_with(h))
        {
            let label = line.trim_start_matches('*').split(":**").next();
            report.push(f.row(HEADER, n, label.unwrap_or(line)));
        } else if line.starts_with('>') {
            report.push(f.row("> **Audit trail only.**", n, "note"));
        } else if is_footer(line) {
            report.push(f.row("footer", n, "footer"));
        }
    }
    let tables = markdown::tables(text);
    let mut title = None;
    let mut entries = Vec::new();
    for s in sections::sections(text) {
        match (s.level, s.title.as_str()) {
            (1, _) if title.is_none() => title = Some(s.title.clone()),
            (2, area) if discretion(area) => {
                report.push(f.row("## Claude's Discretion", s.line, area))
            }
            (2, "Deferred Ideas") => report.push(f.row("## Deferred Ideas", s.line, &s.title)),
            (2, _) => {
                report.push(match entries.is_empty() {
                    true => f.row("## Area", s.line, &s.title),
                    false => f.mapped(s.line, &s.title, "one of discussion.entries"),
                });
                entries.push(entry(&f, &t, &s, &tables, report));
            }
            _ => report.push(f.unmapped(s.line, &s.title, UNKNOWN.to_string())),
        }
    }
    let mut fields = BTreeMap::new();
    if !entries.is_empty() {
        fields.insert("entries".to_string(), json!(entries));
    }
    records.add(
        f.within(
            "discussion",
            ("phase", &f.phase),
            &f.stem,
            title.as_deref().unwrap_or(&f.stem),
            fields,
            0,
        ),
        report,
    );
}

/// The entry of the area `s`: its first table's options and ✓ row, its user's choice and its notes, mode `interactive`.
fn entry(
    f: &PhaseFile<'_>,
    t: &Text<'_>,
    s: &Section,
    tables: &[Table],
    report: &mut Report,
) -> Value {
    let mut entry = json!({ "area": s.title, "mode": "interactive" });
    let mut marked = Vec::new();
    if let Some(table) = tables.iter().find(|t| s.body.contains(&t.line)) {
        let column = |name: &str| {
            table
                .header
                .iter()
                .position(|h| h.eq_ignore_ascii_case(name))
        };
        let option = column("Option").unwrap_or(0);
        if let Some(c) = column("Description") {
            report.push(f.row("| Description |", table.line, &table.header[c]));
        }
        let cell = |row: &[String], c: usize| row.get(c).map_or("", |c| c.trim()).to_string();
        let options: Vec<String> = table
            .rows
            .iter()
            .map(|r| cell(r, option))
            .filter(|o| !o.is_empty())
            .collect();
        if let Some(selected) = column("Selected") {
            marked = table
                .rows
                .iter()
                .filter(|r| cell(r, selected).contains(['✓', '✔']))
                .map(|r| cell(r, option))
                .collect();
        }
        if !options.is_empty() {
            entry["options"] = json!(options);
        }
        if marked.len() > 1 {
            report.push(f.unmapped(
                table.line,
                &s.title,
                format!(
                    "{} options are marked ✓: the user's choice is taken instead",
                    marked.len()
                ),
            ));
        }
    }
    let choice = labelled(
        &t.lines,
        &t.fenced,
        &s.body,
        &["**User's choice:**", "**User’s choice:**"],
    );
    match (marked.as_slice(), choice) {
        ([one], _) => entry["chosen"] = json!(one),
        (_, Some((_, choice))) if !choice.is_empty() => entry["chosen"] = json!(choice),
        _ => {}
    }
    if let Some((_, notes)) =
        labelled(&t.lines, &t.fenced, &s.body, &["**Notes:**"]).filter(|(_, n)| !n.is_empty())
    {
        entry["notes"] = json!(notes);
    }
    entry
}
