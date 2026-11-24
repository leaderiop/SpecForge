//! `NN-UAT.md` to a `uat_session`, its `uat_test`s and their `uat_gap`s (13-RESEARCH Part B, the UAT table; the rows
//! are `table::sections::UAT`). The file stem is the session's id; each `### N. Title` under `## Tests` is a test `U<N>`
//! scoped by the session, its `key: value` lines read by the frontmatter reader (so `expected: |` is a block scalar);
//! each `- truth:` item of `## Gaps` a gap `G<n>` scoped by the test it names. A test's `result` is a harness value a
//! person recorded: written as a declared peer of `@gsd/core` (ADR 0391 D7). Statuses and totals are computed.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value as Json, json};

use super::plan::{entries, front, list};
use super::{Context, PhaseFile, is_timestamp, phase_file};
use crate::frontmatter::{self, Front, Located, Value};
use crate::ids;
use crate::records::Records;
use crate::report::{Entry, Outcome, Report};
use crate::sections::{self, Section};

/// The fate table's name of a UAT session.
pub const UA: &str = "NN-UAT.md";

/// `uat_test.result`'s values.
const RESULT: &[&str] = &["pending", "pass", "issue", "skipped", "blocked"];
const SEVERITY: &[&str] = &["blocker", "major", "minor", "cosmetic"];
const BLOCKED_BY: &[&str] = &[
    "server",
    "physical_device",
    "release_build",
    "third_party",
    "prior_phase",
];

const UNKNOWN: &str = "a section no row of the NN-UAT.md table names: not imported";
const TEST: &str = "### N. Title";

/// Reads the UAT session `text`, served at `path` (`rel` under `.planning/`): `phases/<NN-slug>/<NN>-UAT.md`.
pub fn read(
    path: &str,
    rel: &str,
    text: &str,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let Some(f) = phase_file(path, rel, UA, cx, report) else {
        return;
    };
    let session = format!("{}/{}", f.phase, f.stem);
    let mut fields = BTreeMap::new();
    let mut sources = Vec::new();
    let (front, start) = front(&f.doc, text, report);
    for (key, v) in entries(&front) {
        let line = v.line;
        let text = v.value.as_str().map(str::trim).filter(|t| !t.is_empty());
        match key.as_str() {
            "phase" | "status" | "updated" => report.push(f.doc.row(key, line, key)),
            "started" => match text {
                Some(t) if is_timestamp(t) => {
                    fields.insert("started".to_string(), json!(t));
                }
                _ => report.push(f.doc.unmapped(
                    line,
                    key,
                    "`started` is not an RFC 3339 timestamp: left out".to_string(),
                )),
            },
            "source" => {
                for (item, at) in items(v) {
                    match summary(&item, cx) {
                        Some(path) => sources.push(path),
                        None => report.push(f.doc.unmapped(
                            at,
                            key,
                            format!(
                                "`{item}` names no NN-MM-SUMMARY.md of a roadmap phase: left out"
                            ),
                        )),
                    }
                }
            }
            _ => report.push(f.doc.unmapped(
                line,
                key,
                format!("a key `{key}` no row of the NN-UAT.md table names: not imported"),
            )),
        }
    }
    if !sources.is_empty() {
        fields.insert("source".to_string(), json!(sources));
    }
    let body = frontmatter::body(text, start);
    let lines: Vec<&str> = body.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let all = sections::sections(&body);
    let mut tests = BTreeSet::new();
    let mut parent = String::new();
    let mut gaps = Vec::new();
    for s in &all {
        if s.level <= 2 {
            parent = s.title.clone();
        }
        match (s.level, parent.as_str(), s.title.as_str()) {
            (2, _, "Tests") => report.push(f.row("## Tests", s.line, &s.title)),
            (3, "Tests", _) => {
                if let Some(n) = test(&f, &session, s, &lines, records, report) {
                    tests.insert(n);
                }
            }
            (2, _, "Current Test" | "Summary" | "Deferred Follow-Ups") => {
                report.push(f.row(&format!("## {}", s.title), s.line, &s.title))
            }
            (2, _, "Gaps") => gaps.push(s),
            _ => report.push(f.unmapped(s.line, &s.title, UNKNOWN.to_string())),
        }
    }
    // The gaps last: each names a test of the file.
    let mut gap = 0;
    for s in gaps {
        report.push(Entry::at(
            &f.at(s.line),
            &s.title,
            Outcome::Mapped,
            "each item is a uat_gap".to_string(),
        ));
        for (at, item) in gap_items(&lines, s) {
            gap += 1;
            read_gap(&f, &session, gap, (at, &item), &tests, records, report);
        }
    }
    let mut record = f.within(
        "uat_session",
        ("phase", &f.phase),
        &f.stem,
        &f.stem,
        fields,
        0,
    );
    record.refs.extend(sources);
    records.add(record, report);
}

/// The texts of a list (or one text) with each item's 1-based line.
fn items(v: &Located<Value>) -> Vec<(String, usize)> {
    match &v.value {
        Value::List(items) => items
            .iter()
            .filter_map(|i| Some((i.value.as_str()?.trim().to_string(), i.line)))
            .collect(),
        _ => list(&v.value)
            .unwrap_or_default()
            .into_iter()
            .map(|t| (t, v.line))
            .collect(),
    }
}

/// The graph path of the summary `item` names (`01-01-SUMMARY.md` is `v1.0/01/01-01/01-01-SUMMARY`), when its plan's
/// phase is the roadmap's.
fn summary(item: &str, cx: &Context<'_>) -> Option<String> {
    let name = item.rsplit('/').next().unwrap_or(item);
    let plan = name.strip_suffix("-SUMMARY.md")?;
    let number = plan.rsplit_once('-')?.0;
    let phase = cx.roadmap.phases.get(number).filter(|_| ids::plan(plan))?;
    Some(format!("{phase}/{plan}/{plan}-SUMMARY"))
}

/// The map the frontmatter reader reads from `lines`, the first at the 0-based `first`; its refusal reported.
fn block(
    f: &PhaseFile<'_>,
    lines: Vec<&str>,
    first: usize,
    report: &mut Report,
) -> Option<Located<Value>> {
    match frontmatter::read(&Front {
        lines,
        line: first + 1,
    }) {
        Ok(map) => Some(map),
        Err(refused) => {
            report.push(f.doc.unmapped(
                refused.line,
                "## Tests",
                format!("not read: {}", refused.reason),
            ));
            None
        }
    }
}

/// `value` when it is one of `values`, else reported at its line and none.
fn word(
    f: &PhaseFile<'_>,
    key: &str,
    v: &Located<Value>,
    values: &[&str],
    report: &mut Report,
) -> Option<Json> {
    let text = v.value.as_str().map(str::trim).unwrap_or_default();
    if values.contains(&text) {
        return Some(json!(text));
    }
    report.push(f.doc.unmapped(
        v.line,
        key,
        format!("`{key}: {text}` is none of {}: left out", values.join(", ")),
    ));
    None
}

/// The test `### N. Title` of `s`: `U<N>` with its key lines; its number, when its heading gives one.
fn test(
    f: &PhaseFile<'_>,
    session: &str,
    s: &Section,
    lines: &[&str],
    records: &mut Records,
    report: &mut Report,
) -> Option<u64> {
    let parsed = s.title.split_once(". ").and_then(|(n, title)| {
        let n = n.trim().parse::<u64>().ok().filter(|n| *n > 0)?;
        Some((n, title.trim()))
    });
    let Some((n, title)) = parsed else {
        report.push(f.unmapped(
            s.line,
            &s.title,
            "a test heading is `### N. Title`: not imported".to_string(),
        ));
        return None;
    };
    let mut fields = BTreeMap::new();
    let body: Vec<&str> = lines[s.body.clone()].to_vec();
    if let Some(map) = block(f, body, s.body.start, report) {
        let Value::Map(keys) = &map.value else {
            return None;
        };
        for (key, v) in keys {
            let text = v.value.as_str().map(str::trim).filter(|t| !t.is_empty());
            let value = match key.as_str() {
                "expected" | "reported" | "reason" => text.map(|t| json!(t)),
                "result" => word(f, key, v, RESULT, report),
                "severity" => word(f, key, v, SEVERITY, report),
                "blocked_by" => word(f, key, v, BLOCKED_BY, report),
                _ => {
                    report.push(f.doc.unmapped(
                        v.line,
                        key,
                        format!("a key `{key}` no row of the {TEST} table names: not imported"),
                    ));
                    None
                }
            };
            if let Some(value) = value {
                fields.insert(key.clone(), value);
            }
        }
    }
    let record = f.within(
        "uat_test",
        ("session", session),
        &format!("U{n}"),
        title,
        fields,
        s.line,
    );
    records.add(record, report);
    Some(n)
}

/// Each `- ` item of the gaps section `s`: its 0-based line and its lines, the marker and the item's indent removed.
fn gap_items<'l>(lines: &[&'l str], s: &Section) -> Vec<(usize, Vec<&'l str>)> {
    let mut out: Vec<(usize, Vec<&str>)> = Vec::new();
    for n in s.body.clone() {
        let line = lines[n];
        if let Some(first) = line.strip_prefix("- ") {
            out.push((n, vec![first]));
        } else if let Some((_, item)) = out.last_mut() {
            item.push(line.strip_prefix("  ").unwrap_or(line));
        }
    }
    out
}

/// The gap `G<n>` read from `item`, at the 0-based `at`: scoped by the test its `test:` names, which must be a test of
/// the file (`tests`); else reported and dropped.
fn read_gap(
    f: &PhaseFile<'_>,
    session: &str,
    n: usize,
    (at, item): (usize, &[&str]),
    tests: &BTreeSet<u64>,
    records: &mut Records,
    report: &mut Report,
) {
    let Some(map) = block(f, item.to_vec(), at, report) else {
        return;
    };
    let Value::Map(keys) = &map.value else {
        return;
    };
    let mut fields = BTreeMap::new();
    let mut test = None;
    for (key, v) in keys {
        let text = v.value.as_str().map(str::trim).filter(|t| !t.is_empty());
        match key.as_str() {
            "truth" | "reason" | "root_cause" => {
                if let Some(t) = text {
                    fields.insert(key.clone(), json!(t));
                }
            }
            "severity" => {
                if let Some(w) = text.and_then(|_| word(f, key, v, SEVERITY, report)) {
                    fields.insert(key.clone(), w);
                }
            }
            "missing" => {
                let items: Vec<String> = list(&v.value).unwrap_or_default();
                if !items.is_empty() {
                    fields.insert(key.clone(), json!(items));
                }
            }
            "artifacts" => {
                let items: Vec<Json> = list(&v.value)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|path| json!({ "path": path }))
                    .collect();
                if !items.is_empty() {
                    fields.insert(key.clone(), json!(items));
                }
            }
            "status" => report.push(f.doc.row("## Gaps status", v.line, key)),
            "debug_session" if text.is_some() => {
                report.push(f.doc.row("## Gaps debug_session", v.line, key))
            }
            "debug_session" => {}
            "test" => test = Some((text.unwrap_or_default().to_string(), v.line)),
            _ => report.push(f.doc.unmapped(
                v.line,
                key,
                format!("a key `{key}` no row of the ## Gaps table names: not imported"),
            )),
        }
    }
    let named = test
        .as_ref()
        .and_then(|(t, _)| t.parse::<u64>().ok())
        .filter(|t| tests.contains(t));
    let Some(t) = named else {
        let (said, line) = test.unwrap_or_default();
        report.push(f.doc.unmapped(
            if line == 0 { at + 1 } else { line },
            "## Gaps",
            format!("the gap names no test of this file (`test: {said}`): not imported"),
        ));
        return;
    };
    let title = fields
        .get("truth")
        .and_then(Json::as_str)
        .map_or_else(|| format!("G{n}"), str::to_string);
    let test_path = format!("{session}/U{t}");
    let record = f.within(
        "uat_gap",
        ("test", &test_path),
        &format!("G{n}"),
        &title,
        fields,
        at,
    );
    records.add(record, report);
}
