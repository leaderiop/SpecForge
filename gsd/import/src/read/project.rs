//! `PROJECT.md` to the `project` and its decisions (13-RESEARCH Part B, the `PROJECT.md` table; the rows are
//! `table::sections::PROJECT`): `# Name` gives the project's name and id, its prose sections their fields, `### Out of
//! Scope` and `## Constraints` their bullets as blocks, and each `## Key Decisions` row one `decision D-<NN>`.

use std::collections::BTreeMap;
use std::ops::Range;

use gsd_kit::markdown::{self, InlineBlock};
use serde_json::{Value, json};

use super::{blocks_in, is_bullet, mapped, prose, row, slug};
use crate::ids;
use crate::paths;
use crate::records::{Record, Records, Source};
use crate::report::{Entry, Outcome, Report};
use crate::sections::{self, Section};

const P: &str = "PROJECT.md";

/// A constraint's `kind` values (`@gsd/core`'s `project.constraints.kind`).
const CONSTRAINT_KINDS: &[&str] = &[
    "tech_stack",
    "timeline",
    "budget",
    "dependencies",
    "compatibility",
    "performance",
    "security",
    "other",
];

/// What a section no row names becomes.
const UNKNOWN: &str = "a section no row of the PROJECT.md table names: no field holds it";

/// Reads `text`, served at `path`.
pub fn read(path: &str, text: &str, records: &mut Records, report: &mut Report) {
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let fenced = sections::fenced(text);
    let blocks = markdown::inline_blocks(text);
    let at = |line: usize| Source {
        file: path.to_string(),
        line: line + 1,
    };
    let mut name: Option<(String, usize)> = None;
    let mut fields: BTreeMap<String, Value> = BTreeMap::new();
    let mut decisions: Vec<String> = Vec::new();
    // A section reported whole covers its sub-sections.
    let mut covered = 0..0;
    for s in sections::sections(text) {
        if covered.contains(&s.line) {
            continue;
        }
        let to = |what: &str, report: &mut Report| report.push(mapped(&at(s.line), &s.title, what));
        let prose_of = |field: &str, fields: &mut BTreeMap<String, Value>, report: &mut Report| {
            to(&format!("project.{field}"), report);
            if let Some(text) = prose(&blocks, &s.body) {
                fields.insert(field.to_string(), json!(text));
            }
        };
        match (s.level, s.title.as_str()) {
            (1, title) if name.is_none() => name = Some((title.to_string(), s.line)),
            (2, "What This Is") => prose_of("what", &mut fields, report),
            (2, "Core Value") => prose_of("core_value", &mut fields, report),
            (2, "Context") => prose_of("context", &mut fields, report),
            (2, "Requirements") => report.push(row(P, "## Requirements", &at(s.line), &s.title)),
            (3, "Validated" | "Active") => {
                let key = format!("### {}", s.title);
                report.push(row(P, &key, &at(s.line), &s.title));
                for b in blocks_in(&blocks, &s.body).filter(|b| b.text.starts_with("(None yet")) {
                    report.push(row(P, "(None yet", &at(b.lines[0]), &s.title));
                }
            }
            (3, "Out of Scope") => {
                to("project.out_of_scope", report);
                let items = out_of_scope(&s, &blocks, &lines, &at, report);
                if !items.is_empty() {
                    fields.insert("out_of_scope".to_string(), json!(items));
                }
            }
            (2, "Constraints") => {
                to("project.constraints", report);
                let items = constraints(&s, &blocks, &lines, &at, report);
                if !items.is_empty() {
                    fields.insert("constraints".to_string(), json!(items));
                }
            }
            (2, "Key Decisions") => {
                to("each row a decision", report);
                decisions = key_decisions(text, &s.body, &at, records, report);
            }
            (2, "Business Context" | "Current Milestone") => {
                covered = s.body.clone();
                let key = format!("## {}", s.title);
                report.push(row(P, &key, &at(s.line), &s.title));
            }
            _ => {
                covered = s.body.clone();
                report.push(Entry::at(
                    &at(s.line),
                    &s.title,
                    Outcome::Unmapped,
                    UNKNOWN.to_string(),
                ));
            }
        }
    }
    for (n, line) in lines.iter().enumerate() {
        if !fenced[n] && line.starts_with("*Last updated") {
            report.push(row(P, "*Last updated:", &at(n), "*Last updated:"));
        }
    }
    let Some((name, line)) = name else {
        report.push(Entry::at(
            &at(0),
            "# Name",
            Outcome::Unmapped,
            "no `# Name` heading names the project: it is not imported".to_string(),
        ));
        return;
    };
    let id = slug(&name);
    if !ids::name(&id) {
        report.push(Entry::at(
            &at(line),
            "# Name",
            Outcome::Unmapped,
            format!("the project's name `{name}` gives no id: it is not imported"),
        ));
        return;
    }
    fields.insert("name".to_string(), json!(name));
    if !decisions.is_empty() {
        fields.insert("key_decisions".to_string(), json!(decisions));
    }
    records.add(
        Record {
            kind: "project",
            id,
            title: name,
            file: paths::project(),
            fields,
            refs: decisions,
            source: at(line),
        },
        report,
    );
}

/// The bullets of `body`: each leaf block that a raw list marker starts, with whether its text opens with a bold label.
fn bullets<'b>(
    blocks: &'b [InlineBlock],
    lines: &[&str],
    body: &Range<usize>,
) -> Vec<(&'b InlineBlock, bool)> {
    blocks_in(blocks, body)
        .filter_map(|b| {
            let raw = lines.get(b.lines[0])?;
            is_bullet(raw).then(|| (b, raw.trim_start()[2..].trim_start().starts_with("**")))
        })
        .collect()
}

/// `### Out of Scope` bullets `- item — why` as `{item, why}` blocks; a bullet with no ` — ` is reported.
fn out_of_scope(
    s: &Section,
    blocks: &[InlineBlock],
    lines: &[&str],
    at: &dyn Fn(usize) -> Source,
    report: &mut Report,
) -> Vec<Value> {
    let mut items = Vec::new();
    for (b, _) in bullets(blocks, lines, &s.body) {
        match b.text.split_once(" — ") {
            Some((item, why)) if !item.trim().is_empty() && !why.trim().is_empty() => {
                items.push(json!({"item": item.trim(), "why": why.trim()}));
            }
            _ => report.push(Entry::at(
                &at(b.lines[0]),
                &s.title,
                Outcome::Unmapped,
                "an Out of Scope item needs a reason (`- item — why`): not imported".to_string(),
            )),
        }
    }
    items
}

/// `## Constraints` bullets `- **Label**: what — why` as `{kind, what, why}` blocks: the label lower-cased with `_`
/// for spaces is the kind; a label that names no kind, or none, is `other` with the label kept in `what`, reported.
fn constraints(
    s: &Section,
    blocks: &[InlineBlock],
    lines: &[&str],
    at: &dyn Fn(usize) -> Source,
    report: &mut Report,
) -> Vec<Value> {
    let mut items = Vec::new();
    for (b, bold) in bullets(blocks, lines, &s.body) {
        let text = b.text.trim();
        let labelled = bold.then(|| text.split_once(':')).flatten();
        let (kind, rest) = match labelled {
            Some((label, rest)) => (label.trim().to_lowercase().replace(' ', "_"), rest.trim()),
            None => (String::new(), text),
        };
        let (what, why) = match rest.split_once(" — ") {
            Some((what, why)) => (what.trim(), Some(why.trim())),
            None => (rest, None),
        };
        let mut block = if CONSTRAINT_KINDS.contains(&kind.as_str()) {
            json!({"kind": kind, "what": what})
        } else {
            report.push(row(P, "## Constraints", &at(b.lines[0]), &s.title));
            let what = match labelled {
                Some((label, _)) => format!("{}: {what}", label.trim()),
                None => what.to_string(),
            };
            json!({"kind": "other", "what": what})
        };
        if let Some(why) = why.filter(|w| !w.is_empty()) {
            block["why"] = json!(why);
        }
        items.push(block);
    }
    items
}

/// Each `## Key Decisions` row as `decision D-<NN>` (row order, from 01), origin `project`; their ids.
fn key_decisions(
    text: &str,
    body: &Range<usize>,
    at: &dyn Fn(usize) -> Source,
    records: &mut Records,
    report: &mut Report,
) -> Vec<String> {
    let mut ids = Vec::new();
    for table in markdown::tables(text)
        .into_iter()
        .filter(|t| body.contains(&t.line))
    {
        let column = |name: &str, or: usize| {
            table
                .header
                .iter()
                .position(|h| h.eq_ignore_ascii_case(name))
                .unwrap_or(or)
        };
        let (decision, rationale, outcome) = (
            column("Decision", 0),
            column("Rationale", 1),
            column("Outcome", 2),
        );
        report.push(row(P, "## Key Decisions", &at(table.line), "Key Decisions"));
        for (i, cells) in table.rows.iter().enumerate() {
            // The header and the delimiter row come first.
            let source = at(table.line + 2 + i);
            let cell = |i: usize| cells.get(i).map(|c| c.trim()).unwrap_or_default();
            if cell(decision).is_empty() {
                report.push(Entry::at(
                    &source,
                    "Key Decisions",
                    Outcome::Unmapped,
                    "a Key Decisions row with no decision: not imported".to_string(),
                ));
                continue;
            }
            let id = format!("D-{:02}", ids.len() + 1);
            let mut fields = BTreeMap::from([
                ("text".to_string(), json!(cell(decision))),
                ("origin".to_string(), json!("project")),
            ]);
            if !cell(rationale).is_empty() {
                fields.insert("rationale".to_string(), json!(cell(rationale)));
            }
            match outcome_of(cell(outcome)) {
                Some(word) => {
                    fields.insert("outcome".to_string(), json!(word));
                }
                None if cell(outcome).is_empty() => {}
                None => report.push(Entry::at(
                    &source,
                    "Key Decisions",
                    Outcome::Unmapped,
                    format!(
                        "decision {id}: the outcome `{}` is none of Good, Revisit, Pending: imported without one",
                        cell(outcome)
                    ),
                )),
            }
            records.add(
                Record {
                    kind: "decision",
                    id: id.clone(),
                    title: cell(decision).to_string(),
                    file: paths::project(),
                    fields,
                    refs: Vec::new(),
                    source,
                },
                report,
            );
            ids.push(id);
        }
    }
    ids
}

/// `— Pending`, `✓ Good`, `⚠️ Revisit` as `pending`, `good`, `revisit`.
fn outcome_of(cell: &str) -> Option<&'static str> {
    let word = cell
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .trim()
        .to_lowercase();
    ["pending", "good", "revisit"]
        .into_iter()
        .find(|w| *w == word)
}
