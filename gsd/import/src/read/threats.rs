//! A plan's `<threat_model>` to `@gsd/security`'s kinds (ADR 0391 D11; 13-RESEARCH Part B, the `SECURITY.md` and
//! `<threat_model>` table; the rows are `<threat_model> ## Trust Boundaries` and `## STRIDE Threat Register` of
//! `table::sections::PLAN`): each Trust Boundaries row a `trust_boundary TB<n>`, each register row a `threat` named by
//! its Threat ID, both scoped by the plan. Without `@gsd/security` loaded, each element is one needs-extension entry
//! and no record. A threat's `status` is the auditor's (`SECURITY.md`, 13-08): never written here.

use std::collections::BTreeMap;

use serde_json::{Value as Json, json};

use super::plan::{Doc, elements};
use super::tasks::scoped;
use crate::ids;
use crate::records::Records;
use crate::report::{Entry, Outcome, Report};

/// `@gsd/security`'s `threat.category` values, in STRIDE order: a letter is the first of its word.
const CATEGORY: &[&str] = &[
    "spoofing",
    "tampering",
    "repudiation",
    "information_disclosure",
    "denial_of_service",
    "elevation_of_privilege",
];
const SEVERITY: &[&str] = &["critical", "high", "medium", "low"];
const DISPOSITION: &[&str] = &["mitigate", "accept", "transfer"];

const BOUNDARIES: &str = "Trust Boundaries";
const REGISTER: &str = "STRIDE Threat Register";

/// Reads the `<threat_model>` elements of the plan body `body` into trust boundaries and threats of the plan `plan`,
/// written to `file`, when `security`; else reports each element.
pub fn read(
    doc: &Doc<'_>,
    body: &str,
    (plan, file): (&str, &str),
    security: bool,
    records: &mut Records,
    report: &mut Report,
) {
    let mut boundary = 0;
    for element in elements(doc, body, 1, "threat_model", report) {
        if !security {
            report.push(Entry::at(
                &doc.at(element.line),
                "<threat_model>",
                Outcome::NeedsExtension,
                "trust_boundary and threat are kinds of @gsd/security, which the project does not load".to_string(),
            ));
            continue;
        }
        for table in gsd_kit::markdown::tables(&element.body) {
            // The body starts on the opening tag's line; a table's line is 0-based, its rows two lines below it.
            let at = element.line + table.line;
            let section = format!("<threat_model> ## {}", table.heading);
            let kind = match table.heading.as_str() {
                BOUNDARIES => "trust_boundary",
                REGISTER => "threat",
                _ => {
                    report.push(doc.unmapped(
                        at,
                        &section,
                        "a table no row of the plan's table names: not imported".to_string(),
                    ));
                    continue;
                }
            };
            let columns = columns(doc, &table.header, kind, at, &section, report);
            for (i, cells) in table.rows.iter().enumerate() {
                let line = at + 2 + i;
                let cell = |name: &str| {
                    columns
                        .get(name)
                        .and_then(|&c| cells.get(c))
                        .map(|c| c.trim())
                        .filter(|c| !c.is_empty())
                };
                let built = if kind == "trust_boundary" {
                    boundary += 1;
                    Ok((format!("TB{boundary}"), trust_boundary(&cell)))
                } else {
                    threat(&cell)
                };
                let (id, (title, fields)) = match built {
                    Ok(built) => built,
                    Err(why) => {
                        report.push(doc.unmapped(line, &section, format!("{why}: not imported")));
                        continue;
                    }
                };
                let title = title.unwrap_or_else(|| id.clone());
                let record = scoped(doc, kind, id, title, (plan, file), fields, line);
                records.add(record, report);
            }
        }
    }
}

/// The columns of a table's `header` this reader takes, by name; each other column is reported once.
fn columns(
    doc: &Doc<'_>,
    header: &[String],
    kind: &str,
    at: usize,
    section: &str,
    report: &mut Report,
) -> BTreeMap<String, usize> {
    let known: &[&str] = match kind {
        "trust_boundary" => &["Boundary", "Description"],
        _ => &[
            "Threat ID",
            "Category",
            "Component",
            "Severity",
            "Disposition",
            "Mitigation Plan",
            "Mitigation",
        ],
    };
    let mut out = BTreeMap::new();
    for (i, name) in header.iter().map(|h| h.trim()).enumerate() {
        if known.contains(&name) {
            out.insert(name.to_string(), i);
        } else {
            report.push(doc.unmapped(
                at,
                section,
                format!("a column `{name}` no row of the table names: not imported"),
            ));
        }
    }
    out
}

type Fields = (Option<String>, BTreeMap<String, Json>);

/// A Trust Boundaries row: its title (the boundary) and fields. A missing cell is a missing field, which the emit
/// reports.
fn trust_boundary<'c>(cell: &impl Fn(&str) -> Option<&'c str>) -> Fields {
    let mut fields = BTreeMap::new();
    for (column, field) in [("Boundary", "boundary"), ("Description", "description")] {
        if let Some(v) = cell(column) {
            fields.insert(field.to_string(), json!(v));
        }
    }
    (cell("Boundary").map(str::to_string), fields)
}

/// A register row: its Threat ID, title (the component) and fields; or why it is refused (an id the grammar refuses,
/// a value no enum of `threat` holds).
fn threat<'c>(cell: &impl Fn(&str) -> Option<&'c str>) -> Result<(String, Fields), String> {
    let id = cell("Threat ID").unwrap_or_default();
    if !ids::name(id) {
        return Err(format!("the Threat ID `{id}` is no id a threat takes"));
    }
    let mut fields = BTreeMap::new();
    if let Some(v) = cell("Category") {
        let word = v.to_ascii_lowercase().replace([' ', '-'], "_");
        let found = CATEGORY
            .iter()
            .find(|c| **c == word || (word.len() == 1 && c.starts_with(&word)))
            .ok_or_else(|| {
                format!(
                    "the Category `{v}` is no STRIDE letter or word ({})",
                    CATEGORY.join(", ")
                )
            })?;
        fields.insert("category".to_string(), json!(found));
    }
    for (column, field, values) in [
        ("Severity", "severity", SEVERITY),
        ("Disposition", "disposition", DISPOSITION),
    ] {
        if let Some(v) = cell(column) {
            let word = v.to_ascii_lowercase();
            if !values.contains(&word.as_str()) {
                return Err(format!(
                    "the {column} `{v}` is none of {}",
                    values.join(", ")
                ));
            }
            fields.insert(field.to_string(), json!(word));
        }
    }
    if let Some(v) = cell("Component") {
        fields.insert("component".to_string(), json!(v));
    }
    if let Some(v) = cell("Mitigation Plan").or_else(|| cell("Mitigation")) {
        fields.insert("mitigation".to_string(), json!(v));
    }
    let title = cell("Component").map(str::to_string);
    Ok((id.to_string(), (title, fields)))
}
