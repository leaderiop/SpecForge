//! `REQUIREMENTS.md` to `requirement` records (13-RESEARCH Part B, the `REQUIREMENTS.md` table): the `## v1`, `## v2`
//! and `## Out of Scope` sections give the tier, `### Category` the category, each `**ID**: text` bullet one
//! requirement, and each row of the Out of Scope table one `OOS-<NN>` requirement, whose id the import mints.

use std::collections::BTreeMap;

use gsd_kit::markdown;
use serde_json::{Value, json};

use super::{is_date, mapped};
use crate::paths;
use crate::records::{Record, Records, Source};
use crate::report::{Entry, Outcome, Report};

const OUT_OF_SCOPE: &str = "Out of Scope";

/// Reads `text`, served at `path`.
pub fn read(path: &str, text: &str, records: &mut Records, report: &mut Report) {
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let fenced: Vec<bool> = {
        let mut fenced = vec![false; lines.len()];
        for fence in markdown::fences(text) {
            let end = (fence.code.end + 1).min(lines.len());
            fenced[fence.open.min(end)..end].fill(true);
        }
        fenced
    };
    let at = |line: usize| Source {
        file: path.to_string(),
        line: line + 1,
    };
    let defined = defined(&lines, &at, report);
    let mut section = String::new();
    let mut tier: Option<&'static str> = None;
    let mut category: Option<String> = None;
    let mut checkboxes = false;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let n = i;
        i += 1;
        if fenced[n] {
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            section = heading.trim().to_string();
            category = None;
            tier = tier_of(&section);
            let entry = match tier {
                _ if section == "Traceability" => Entry::at(
                    &at(n),
                    &section,
                    Outcome::Redundant,
                    "ROADMAP.md's **Requirements** lines give each phase's requirements"
                        .to_string(),
                ),
                Some(tier) => mapped(&at(n), &section, &format!("requirement.tier {tier}")),
                None => Entry::at(
                    &at(n),
                    &section,
                    Outcome::Unmapped,
                    "a section no row of the REQUIREMENTS.md table names: not imported".to_string(),
                ),
            };
            report.push(entry);
        } else if let Some(heading) = line.strip_prefix("### ") {
            category = Some(heading.trim().to_string());
            report.push(mapped(&at(n), heading.trim(), "requirement.category"));
        } else if let Some(title) = line.strip_prefix("# ") {
            report.push(Entry::at(
                &at(n),
                title.trim(),
                Outcome::Redundant,
                "PROJECT.md's title names the project".to_string(),
            ));
        } else if line.starts_with("**Core Value:**") {
            report.push(Entry::at(
                &at(n),
                "Core Value",
                Outcome::Redundant,
                "PROJECT.md's ## Core Value gives it".to_string(),
            ));
        } else if line.starts_with("**Coverage:**") {
            report.push(Entry::at(
                &at(n),
                &section,
                Outcome::Redundant,
                "the coverage counts follow from the roadmap's requirements".to_string(),
            ));
        } else if let Some(tier) = tier.filter(|t| *t != "out_of_scope") {
            let Some(item) = bullet(line) else {
                continue;
            };
            // A bullet's text runs on over its indented lines.
            let mut words = vec![item.text.to_string()];
            while i < lines.len()
                && !fenced[i]
                && lines[i].starts_with([' ', '\t'])
                && !lines[i].trim().is_empty()
                && bullet(lines[i]).is_none()
            {
                words.push(lines[i].trim().to_string());
                i += 1;
            }
            let text = words.join(" ").trim().to_string();
            if item.checkbox && !checkboxes {
                checkboxes = true;
                report.push(Entry::at(
                    &at(n),
                    &section,
                    Outcome::Computed,
                    "a requirement's checkbox is requirement.status, computed from plans, summaries and verifications"
                        .to_string(),
                ));
            }
            let Some(category) = &category else {
                report.push(Entry::at(
                    &at(n),
                    &section,
                    Outcome::Unmapped,
                    format!(
                        "requirement {} has no ### category above it: not imported",
                        item.id
                    ),
                ));
                continue;
            };
            if text.is_empty() {
                report.push(Entry::at(
                    &at(n),
                    category,
                    Outcome::Unmapped,
                    format!("requirement {} has no text: not imported", item.id),
                ));
                continue;
            }
            let mut fields = BTreeMap::from([
                ("text".to_string(), json!(text)),
                ("category".to_string(), json!(category)),
                ("tier".to_string(), json!(tier)),
            ]);
            add_defined(&mut fields, defined);
            records.add(
                Record {
                    kind: "requirement",
                    id: item.id.to_string(),
                    title: text,
                    file: paths::requirements(),
                    fields,
                    refs: Vec::new(),
                    links: Vec::new(),
                    source: at(n),
                },
                report,
            );
        }
    }
    out_of_scope(text, defined, &at, records, report);
}

/// The `**Defined:**` date, if the file gives one; a value that is no date is reported.
fn defined<'a>(
    lines: &[&'a str],
    at: &dyn Fn(usize) -> Source,
    report: &mut Report,
) -> Option<&'a str> {
    let (n, value) = lines
        .iter()
        .enumerate()
        .find_map(|(n, l)| Some((n, l.strip_prefix("**Defined:**")?.trim())))?;
    if is_date(value) {
        return Some(value);
    }
    report.push(Entry::at(
        &at(n),
        "Defined",
        Outcome::Unmapped,
        "**Defined:** is not a YYYY-MM-DD date: no requirement gets one".to_string(),
    ));
    None
}

fn add_defined(fields: &mut BTreeMap<String, Value>, defined: Option<&str>) {
    if let Some(date) = defined {
        fields.insert("defined".to_string(), json!(date));
    }
}

/// The tier a `## ` heading names.
fn tier_of(heading: &str) -> Option<&'static str> {
    if heading.starts_with("v1") {
        Some("v1")
    } else if heading.starts_with("v2") {
        Some("v2")
    } else if heading == OUT_OF_SCOPE {
        Some("out_of_scope")
    } else {
        None
    }
}

/// A requirement bullet: `- [x] **ID**: text`, `- **ID**: text` or `- **ID:** text`.
struct Bullet<'a> {
    checkbox: bool,
    id: &'a str,
    text: &'a str,
}

fn bullet(line: &str) -> Option<Bullet<'_>> {
    let rest = line.trim_start();
    let rest = rest
        .strip_prefix("- ")
        .or_else(|| rest.strip_prefix("* "))?;
    let (checkbox, rest) = match ["[x] ", "[X] ", "[ ] "]
        .iter()
        .find_map(|b| rest.strip_prefix(b))
    {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let (id, after) = rest.strip_prefix("**")?.split_once("**")?;
    let (id, text) = match id.strip_suffix(':') {
        Some(id) => (id, after),
        None => (id, after.strip_prefix(':')?),
    };
    Some(Bullet {
        checkbox,
        id: id.trim(),
        text: text.trim(),
    })
}

/// Each row of a table under `## Out of Scope`: a requirement `OOS-<NN>` in row order, its id `filled`.
fn out_of_scope(
    text: &str,
    defined: Option<&str>,
    at: &dyn Fn(usize) -> Source,
    records: &mut Records,
    report: &mut Report,
) {
    let mut n = 0;
    for table in markdown::tables(text)
        .into_iter()
        .filter(|t| t.heading == OUT_OF_SCOPE)
    {
        let column = |name: &str, or: usize| {
            table
                .header
                .iter()
                .position(|h| h.eq_ignore_ascii_case(name))
                .unwrap_or(or)
        };
        let (feature, reason) = (column("Feature", 0), column("Reason", 1));
        for (row_index, row) in table.rows.iter().enumerate() {
            // The header and the delimiter row come first.
            let source = at(table.line + 2 + row_index);
            let cell = |i: usize| row.get(i).map(|c| c.trim()).unwrap_or_default();
            if cell(feature).is_empty() {
                report.push(Entry::at(
                    &source,
                    OUT_OF_SCOPE,
                    Outcome::Unmapped,
                    "an Out of Scope row with no feature: not imported".to_string(),
                ));
                continue;
            }
            n += 1;
            let id = format!("OOS-{n:02}");
            let mut fields = BTreeMap::from([
                ("text".to_string(), json!(cell(feature))),
                ("category".to_string(), json!(OUT_OF_SCOPE)),
                ("tier".to_string(), json!("out_of_scope")),
            ]);
            if !cell(reason).is_empty() {
                fields.insert("reason".to_string(), json!(cell(reason)));
            }
            add_defined(&mut fields, defined);
            report.push(Entry::at(
                &source,
                OUT_OF_SCOPE,
                Outcome::Filled,
                format!("requirement {id}: GSD gives an Out of Scope row no id, so the import numbers the rows"),
            ));
            records.add(
                Record {
                    kind: "requirement",
                    id,
                    title: cell(feature).to_string(),
                    file: paths::requirements(),
                    fields,
                    refs: Vec::new(),
                    links: Vec::new(),
                    source,
                },
                report,
            );
        }
    }
}
