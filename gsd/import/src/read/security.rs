//! `NN-SECURITY.md` to `@gsd/security`'s kinds, when the project loads it (ADR 0391 D7, D11; 13-RESEARCH Part B, the
//! `SECURITY.md` table; the rows are `table::sections::SECURITY`): the file stem is a `security_audit` of its phase, with
//! the harness `asvs_level` and `created` a person recorded. Its other tables record the auditor's results on the
//! phase's plans, read before it ([`super::all`] reads it last):
//!
//! - a Threat Register row sets the harness `status` of the one plan threat with the same Threat ID **and** the same
//!   component; no match, or two, sets nothing and is reported (a duplicate id across plans is by design);
//! - a Trust Boundaries row sets `data_crossing` on the one plan boundary whose `Boundary` text is equal;
//! - an Accepted Risks Log row is an `accepted_risk AR<n>` naming its unique threat;
//! - a `## Security Audit <date>` block is an `audit_run AU<n>`, its `at` filled `<date>T00:00:00Z` and reported.
//!
//! Nothing SECURITY.md does not record is written: a plan threat with no row keeps no status.

use std::collections::BTreeMap;

use gsd_kit::markdown::{self, Table};
use serde_json::{Value as Json, json};

use super::plan::{entries, front};
use super::{Context, PhaseFile, is_date, is_footer, is_timestamp, phase_file};
use crate::frontmatter;
use crate::ids;
use crate::records::Records;
use crate::report::{Entry, Outcome, Report};
use crate::sections::{self, Section};

/// The fate table's name of a security audit.
pub const SC: &str = "SECURITY.md";

const THREAT_STATUS: &[&str] = &["open", "closed"];
const UNKNOWN: &str = "a section no row of the SECURITY.md table names: not imported";
const RUN: &str = "Security Audit ";

/// Reads the security audit `text`, served at `path` (`rel` under `.planning/`): `phases/<NN-slug>/<NN>-SECURITY.md`.
pub fn read(
    path: &str,
    rel: &str,
    text: &str,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let Some(f) = phase_file(path, rel, SC, cx, report) else {
        return;
    };
    let audit = format!("{}/{}", f.phase, f.stem);
    let mut fields = BTreeMap::new();
    let (front, start) = front(&f.doc, text, report);
    for (key, v) in entries(&front) {
        let line = v.line;
        let text = v.value.as_str().map(str::trim).unwrap_or_default();
        match key.as_str() {
            "phase" => {
                let named = ids::padded(text.split_once('-').map_or(text, |(n, _)| n));
                if named.as_deref() == Some(f.number.as_str()) {
                    report.push(f.doc.row("phase, slug", line, key));
                } else {
                    report.push(f.doc.unmapped(
                        line,
                        key,
                        format!("`phase: {text}` names another phase than the file name's {}: the file name wins", f.number),
                    ));
                }
            }
            "slug" => report.push(f.doc.row("phase, slug", line, key)),
            "status" | "threats_open" => report.push(f.doc.row(key, line, key)),
            "asvs_level" => match text.parse::<u8>().ok().filter(|l| (1..=3).contains(l)) {
                Some(level) => {
                    fields.insert("asvs_level".to_string(), json!(level));
                }
                None => report.push(f.doc.unmapped(
                    line,
                    key,
                    format!("`asvs_level: {text}` is no level 1 to 3: left out"),
                )),
            },
            "created" if is_timestamp(text) => {
                fields.insert("created".to_string(), json!(text));
            }
            "created" if is_date(text) => {
                fields.insert("created".to_string(), json!(format!("{text}T00:00:00Z")));
                report.push(f.doc.row("created", line, key));
            }
            "created" => report.push(f.doc.unmapped(
                line,
                key,
                format!("`created: {text}` is no date or RFC 3339 timestamp: left out"),
            )),
            _ => report.push(f.doc.unmapped(
                line,
                key,
                format!("a key `{key}` no row of the SECURITY.md table names: not imported"),
            )),
        }
    }
    let body = frontmatter::body(text, start);
    let lines: Vec<&str> = body.split('\n').collect();
    let fenced = sections::fenced(&body);
    for (n, line) in lines.iter().enumerate() {
        if fenced[n] {
            continue;
        }
        if is_footer(line) {
            report.push(f.row("status legend", n, "legend"));
        } else if line.starts_with('>') {
            report.push(f.row("> Per-phase security contract", n, "note"));
        } else if line.trim_start().starts_with("**Approval:**") {
            report.push(f.row("**Approval:**", n, "Sign-Off"));
        }
    }
    let tables = markdown::tables(&body);
    let mut title = None;
    let mut at = 0;
    let mut run = 0;
    for s in sections::sections(&body) {
        let table = tables.iter().find(|t| s.body.contains(&t.line));
        let mapped = |what: &str, report: &mut Report| {
            report.push(Entry::at(
                &f.at(s.line),
                &s.title,
                Outcome::Mapped,
                what.to_string(),
            ));
        };
        match (s.level, s.title.as_str()) {
            (1, _) if title.is_none() => {
                title = Some(s.title.clone());
                at = s.line;
            }
            (2, "Trust Boundaries") => {
                mapped(
                    "each row sets data_crossing on the plan boundary it names",
                    report,
                );
                if let Some(t) = table {
                    boundaries(&f, &s, t, records, report);
                }
            }
            (2, "Threat Register") => {
                mapped(
                    "each row sets status on the plan threat with its Threat ID and component",
                    report,
                );
                if let Some(t) = table {
                    register(&f, &s, t, records, report);
                }
            }
            (2, "Accepted Risks Log") => {
                mapped("each row is an accepted_risk", report);
                if let Some(t) = table {
                    for (i, cells) in t.rows.iter().enumerate() {
                        let row = Row::new(t, cells, t.line + 2 + i);
                        accepted(&f, &s, &audit, &row, records, report);
                    }
                }
            }
            (2, "Security Audit Trail") => {
                report.push(f.row("## Security Audit Trail", s.line, &s.title))
            }
            (2, title) if title.starts_with(RUN) => {
                run += 1;
                audit_run(&f, &s, &audit, run, table, records, report);
            }
            (2, "Sign-Off") => report.push(f.row("## Sign-Off", s.line, &s.title)),
            _ => report.push(f.unmapped(s.line, &s.title, UNKNOWN.to_string())),
        }
    }
    records.add(
        f.within(
            "security_audit",
            ("phase", &f.phase),
            &f.stem,
            title.as_deref().unwrap_or(&f.stem),
            fields,
            at,
        ),
        report,
    );
}

/// One body row of a table: its cells by column name, and its 0-based line.
struct Row<'t> {
    cells: BTreeMap<&'t str, &'t str>,
    line: usize,
}

impl<'t> Row<'t> {
    fn new(table: &'t Table, cells: &'t [String], line: usize) -> Self {
        let cells = table
            .header
            .iter()
            .zip(cells)
            .map(|(h, c)| (h.trim(), c.trim()))
            .filter(|(_, c)| !c.is_empty())
            .collect();
        Row { cells, line }
    }

    fn get(&self, column: &str) -> Option<&'t str> {
        self.cells.get(column).copied()
    }
}

/// The ids of the `kind` records of the phase of `f` whose fields `matches`.
fn of_phase(
    f: &PhaseFile<'_>,
    records: &Records,
    kind: &str,
    matches: impl Fn(&str, &Json) -> bool,
) -> Vec<String> {
    let prefix = format!("{}/", f.phase);
    records
        .iter()
        .filter(|r| r.kind == kind && r.id.starts_with(&prefix))
        .filter(|r| matches(r.id.rsplit('/').next().unwrap_or(&r.id), &json!(r.fields)))
        .map(|r| r.id.clone())
        .collect()
}

/// `found`'s one id, or the reason none is taken (none, or more than one).
fn unique(found: Vec<String>, what: &str) -> Result<String, String> {
    match found.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!("no plan of the phase holds {what}")),
        many => Err(format!(
            "{} plans' entities match {what} ({})",
            many.len(),
            many.join(", ")
        )),
    }
}

/// Each Trust Boundaries row: `data_crossing` on the plan boundary whose `Boundary` text is equal.
fn boundaries(
    f: &PhaseFile<'_>,
    s: &Section,
    table: &Table,
    records: &mut Records,
    report: &mut Report,
) {
    for (i, cells) in table.rows.iter().enumerate() {
        let row = Row::new(table, cells, table.line + 2 + i);
        let boundary = row.get("Boundary").unwrap_or_default();
        let Some(crossing) = row.get("Data Crossing") else {
            report.push(f.unmapped(
                row.line,
                &s.title,
                "no Data Crossing: nothing to set".to_string(),
            ));
            continue;
        };
        let found = of_phase(f, records, "trust_boundary", |_, fields| {
            fields["boundary"] == boundary
        });
        match unique(found, &format!("the boundary `{boundary}`")) {
            Ok(id) => {
                if let Some(r) = records.get_mut("trust_boundary", &id) {
                    r.fields
                        .insert("data_crossing".to_string(), json!(crossing));
                }
                report.push(Entry::at(
                    &f.at(row.line),
                    &s.title,
                    Outcome::Mapped,
                    format!("trust_boundary {id}: data_crossing"),
                ));
            }
            Err(why) => report.push(f.unmapped(row.line, &s.title, format!("{why}: nothing set"))),
        }
    }
}

/// Each Threat Register row: the harness `status` of the one plan threat with its Threat ID and component.
fn register(
    f: &PhaseFile<'_>,
    s: &Section,
    table: &Table,
    records: &mut Records,
    report: &mut Report,
) {
    for (i, cells) in table.rows.iter().enumerate() {
        let row = Row::new(table, cells, table.line + 2 + i);
        let (id, component) = (
            row.get("Threat ID").unwrap_or_default(),
            row.get("Component").unwrap_or_default(),
        );
        let found = of_phase(f, records, "threat", |local, fields| {
            local == id && fields["component"] == component
        });
        let threat = match unique(
            found,
            &format!("the threat `{id}` of the component `{component}`"),
        ) {
            Ok(threat) => threat,
            Err(why) => {
                report.push(f.unmapped(row.line, &s.title, format!("{why}: no status set")));
                continue;
            }
        };
        let status = row.get("Status").map(str::to_ascii_lowercase);
        let Some(status) = status.filter(|st| THREAT_STATUS.contains(&st.as_str())) else {
            report.push(f.unmapped(
                row.line,
                &s.title,
                format!(
                    "the Status of {threat} is none of {}: no status set",
                    THREAT_STATUS.join(", ")
                ),
            ));
            continue;
        };
        let Some(record) = records.get_mut("threat", &threat) else {
            continue;
        };
        record.fields.insert("status".to_string(), json!(status));
        let planned = record.fields.get("mitigation").and_then(Json::as_str);
        if let Some(mitigation) = row.get("Mitigation").filter(|m| Some(*m) != planned) {
            report.push(f.unmapped(
                row.line,
                &s.title,
                format!(
                    "the Mitigation `{mitigation}` differs from the plan's `{}`: the plan's is kept",
                    planned.unwrap_or_default()
                ),
            ));
        }
        report.push(Entry::at(
            &f.at(row.line),
            &s.title,
            Outcome::Mapped,
            format!("threat {threat}: status {status}"),
        ));
    }
}

/// The Accepted Risks Log row `row` as `accepted_risk AR<n>` of the audit `audit`; its `at` the harness date a person
/// recorded.
fn accepted(
    f: &PhaseFile<'_>,
    s: &Section,
    audit: &str,
    row: &Row<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let said = row.get("Risk ID").unwrap_or_default();
    let id = said.replace('-', "");
    if !ids::minted("AR", &id) {
        report.push(f.unmapped(
            row.line,
            &s.title,
            format!("the Risk ID `{said}` is no `AR-<n>`: not imported"),
        ));
        return;
    }
    let reference = row.get("Threat Ref").unwrap_or_default();
    let threat = match unique(
        of_phase(f, records, "threat", |local, _| local == reference),
        &format!("the threat `{reference}`"),
    ) {
        Ok(threat) => threat,
        Err(why) => {
            report.push(f.unmapped(row.line, &s.title, format!("{why}: not imported")));
            return;
        }
    };
    let mut fields = BTreeMap::new();
    fields.insert("threat".to_string(), json!(threat));
    for (column, field) in [("Rationale", "rationale"), ("Accepted By", "accepted_by")] {
        if let Some(v) = row.get(column) {
            fields.insert(field.to_string(), json!(v));
        }
    }
    match row.get("Date") {
        Some(date) if is_date(date) => {
            fields.insert("at".to_string(), json!(date));
        }
        Some(date) => report.push(f.unmapped(
            row.line,
            &s.title,
            format!("the Date `{date}` is no YYYY-MM-DD date: `at` left out"),
        )),
        None => {}
    }
    let mut record = f.within(
        "accepted_risk",
        ("audit", audit),
        &id,
        said,
        fields,
        row.line,
    );
    record.refs.push(threat);
    records.add(record, report);
}

/// The `## Security Audit <date>` block `s` as `audit_run AU<n>`: `at` filled from the heading's date, the `Metric |
/// Count` table its counts.
fn audit_run(
    f: &PhaseFile<'_>,
    s: &Section,
    audit: &str,
    n: usize,
    table: Option<&Table>,
    records: &mut Records,
    report: &mut Report,
) {
    let mut fields = BTreeMap::new();
    let date = s.title.trim_start_matches(RUN).trim();
    if is_date(date) {
        fields.insert("at".to_string(), json!(format!("{date}T00:00:00Z")));
        report.push(f.row("## Security Audit <date>", s.line, &s.title));
    } else {
        report.push(f.unmapped(
            s.line,
            &s.title,
            format!("the heading's `{date}` is no YYYY-MM-DD date: `at` left out"),
        ));
    }
    let rows = table.into_iter().flat_map(|t| {
        t.rows
            .iter()
            .enumerate()
            .map(move |(i, cells)| Row::new(t, cells, t.line + 2 + i))
    });
    for row in rows {
        let metric = row.get("Metric").unwrap_or_default();
        let field = match metric.to_ascii_lowercase().as_str() {
            "threats found" | "threats total" | "total" => "total",
            "closed" => "closed",
            "open" => "open",
            _ => {
                report.push(f.unmapped(
                    row.line,
                    &s.title,
                    format!("the metric `{metric}` is no field of an audit run: not imported"),
                ));
                continue;
            }
        };
        match row.get("Count").and_then(|c| c.parse::<u64>().ok()) {
            Some(count) => {
                fields.insert(field.to_string(), json!(count));
            }
            None => report.push(f.unmapped(
                row.line,
                &s.title,
                format!("the Count of `{metric}` is no whole number: left out"),
            )),
        }
    }
    let record = f.within(
        "audit_run",
        ("audit", audit),
        &format!("AU{n}"),
        &s.title,
        fields,
        s.line,
    );
    records.add(record, report);
}
