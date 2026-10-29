//! The scan: what the verifier checks by reading the phase's files (K11), not by judging them.
//!
//! A compute pass answers `artifact.state` (the file is there, long enough, holds the text and the exports),
//! `key_link.state` (its `pattern` is in the `from` file or the `to` file), `verification.anti_patterns` (the lexical anti-patterns of
//! the phase's files), `validation_row.file_exists` and `wave0_item.done`. A check pass reports `gsd/E505`, a debt
//! marker in a file a plan modifies. Both read the same declared paths; a path the host did not serve (a budget, a
//! cap, a truncated read) is `unchecked` and never `missing`, and never an E505.
//!
//! D-r39-1: E505 is judged over `plan.files_modified`, which is authored, never over a commit's file list, and only
//! for a plan that has a summary: the plan being written is not refused for the files it names.
//!
//! A verification whose phase has a file the host did not serve has `scan_partial`; its `anti_patterns` list what was read.

mod facts;
mod patterns;

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::plans::{link, of_kind, phase_of, strings};
use facts::{Reads, Served, key, reads};
use patterns::{Severity, anti_patterns, is_literal};

const COMPUTES: &[&str] = &[
    "artifact.state",
    "key_link.state",
    "verification.anti_patterns",
    "verification.scan_partial",
    "validation_row.file_exists",
    "wave0_item.done",
];

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("scan", |p| {
        p.compute()
            .computes(COMPUTES)
            .reads(
                "modified",
                FileReadSpec::field("plan.files_modified").text(),
            )
            .reads("artifacts", FileReadSpec::field("artifact.path").text())
            .reads("links", FileReadSpec::field("key_link.from").text())
            .reads("link_targets", FileReadSpec::field("key_link.to").text())
            .reads("rows", FileReadSpec::field("validation_row.file"))
            .reads("wave0", FileReadSpec::field("wave0_item.path"))
            .reads_tests()
            .run(compute);
    });
    c.pass("scan_check", |p| {
        p.phase("check")
            .reads(
                "modified",
                FileReadSpec::field("plan.files_modified").text(),
            )
            .run(check);
    });
}

fn text_of<'a>(entity: &'a PassEntity, field: &str) -> Option<&'a str> {
    entity
        .fields
        .get(field)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

fn number(entity: &PassEntity, field: &str) -> Option<u64> {
    text_of(entity, field)?.parse().ok()
}

fn value(entity: &PassEntity, field: &str, value: serde_json::Value) -> PassValue {
    PassValue {
        entity: entity.id.clone(),
        field: field.into(),
        value,
    }
}

fn compute(input: &PassInput) -> PassOutput {
    let (artifacts, links) = (reads(input, "artifacts"), reads(input, "links"));
    let targets = reads(input, "link_targets");
    let mut values = Vec::new();
    for artifact in of_kind(input, "artifact") {
        let state = text_of(artifact, "path")
            .map_or("unchecked", |p| artifact_state(artifact, artifacts.get(p)));
        values.push(value(artifact, "state", state.into()));
    }
    for link in of_kind(input, "key_link") {
        if let (Some(from), Some(pattern)) = (text_of(link, "from"), text_of(link, "pattern")) {
            let to = text_of(link, "to").map(|t| targets.get(t));
            values.push(value(
                link,
                "state",
                link_state(pattern, links.get(from), to).into(),
            ));
        }
    }
    let (rows, wave0) = (reads(input, "rows"), reads(input, "wave0"));
    for row in of_kind(input, "validation_row") {
        let exists = text_of(row, "file").and_then(|f| match rows.get(f) {
            Served::File(_) => Some(true),
            Served::Absent => Some(false),
            Served::Unserved => None,
        });
        values.extend(exists.map(|e| value(row, "file_exists", e.into())));
    }
    for item in of_kind(input, "wave0_item") {
        let done = text_of(item, "path").and_then(|p| match wave0.get(p) {
            Served::File(_) => Some(ran(input, &item.id)),
            Served::Absent => Some(false),
            Served::Unserved => None,
        });
        values.extend(done.map(|d| value(item, "done", d.into())));
    }
    values.extend(verification_patterns(input));
    PassOutput {
        values,
        ..Default::default()
    }
}

/// A linked test ran: one recorded test of the entity passed or failed.
fn ran(input: &PassInput, id: &str) -> bool {
    input.test_results.as_ref().is_some_and(|report| {
        report
            .of(id)
            .iter()
            .any(|t| matches!(t.status, TestStatus::Passed | TestStatus::Failed))
    })
}

fn artifact_state(artifact: &PassEntity, served: Served) -> &'static str {
    let file = match served {
        Served::Unserved => return "unchecked",
        Served::Absent => return "missing",
        Served::File(file) => file,
    };
    // Upstream counts `split('\n')`: one more than the host's lines for a file that ends in a newline, and 1 for an
    // empty one. A file past the text cap is counted with that extra line.
    let lines = file
        .text
        .map(|t| t.split('\n').count() as u64)
        .or(file.lines.map(|l| l + 1));
    if let (Some(min), Some(lines)) = (number(artifact, "min_lines"), lines)
        && lines < min
    {
        return "too_short";
    }
    let wanted = text_of(artifact, "contains");
    let names = strings(artifact, "exports");
    if wanted.is_none() && names.is_empty() {
        return "present";
    }
    let Some(text) = file.text else {
        return "unchecked";
    };
    if wanted.is_some_and(|w| !text.contains(w)) {
        "lacks_contains"
    } else if names.iter().any(|n| !text.contains(n.as_str())) {
        "lacks_export"
    } else {
        "present"
    }
}

/// Whether the pattern is in one file of the link: `None` when nothing can be said of it.
fn holds(pattern: &str, served: Served) -> Option<bool> {
    match served {
        Served::Unserved => None,
        Served::Absent => Some(false),
        Served::File(file) => file
            .text
            .filter(|_| is_literal(pattern))
            .map(|t| t.contains(pattern)),
    }
}

/// A key link is wired when its `pattern` is in the `from` file or the `to` file (upstream), not wired when every
/// file it names was read and lacks it, else unchecked.
fn link_state(pattern: &str, from: Served, to: Option<Served>) -> &'static str {
    let (from, to) = (
        holds(pattern, from),
        to.map_or(Some(false), |t| holds(pattern, t)),
    );
    if from == Some(true) || to == Some(true) {
        "wired"
    } else if from.is_some() && to.is_some() {
        "not_wired"
    } else {
        "unchecked"
    }
}

/// One hit of a file a plan of the phase modifies.
struct Found {
    file: String,
    line: usize,
    text: String,
    severity: Severity,
    marker: &'static str,
    /// The first plan (by id) that names the file.
    plan: String,
}

/// What the scan found in the files `plans` modify.
struct Scan {
    /// The anti-patterns of every distinct file, in (file, line) order.
    hits: Vec<Found>,
    /// A file was not served (a budget, a cap, a truncated read): the scan is partial.
    partial: bool,
}

fn scanned(plans: &[&PassEntity], modified: &Reads) -> Scan {
    let mut seen = BTreeSet::new();
    let mut partial = false;
    let mut hits = Vec::new();
    for plan in plans {
        for path in strings(plan, "files_modified") {
            let path = key(&path).to_string();
            if !seen.insert(path.clone()) {
                continue;
            }
            match modified.get(&path) {
                Served::File(file) => match file.text {
                    Some(text) => hits.extend(anti_patterns(text).into_iter().map(|hit| Found {
                        file: path.clone(),
                        line: hit.line,
                        text: hit.text,
                        severity: hit.severity,
                        marker: hit.marker,
                        plan: plan.id.clone(),
                    })),
                    None => partial = true,
                },
                Served::Unserved => partial = true,
                Served::Absent => {}
            }
        }
    }
    hits.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Scan { hits, partial }
}

fn plans_sorted(input: &PassInput) -> Vec<&PassEntity> {
    let mut plans: Vec<&PassEntity> = of_kind(input, "plan").collect();
    plans.sort_by(|a, b| a.id.cmp(&b.id));
    plans
}

fn verification_patterns(input: &PassInput) -> Vec<PassValue> {
    let modified = reads(input, "modified");
    let plans = plans_sorted(input);
    of_kind(input, "verification")
        .flat_map(|verification| {
            let Some(phase) = link(input, verification, "phase") else {
                return Vec::new();
            };
            let mine: Vec<&PassEntity> = plans
                .iter()
                .copied()
                .filter(|p| phase_of(input, &p.id) == Some(phase))
                .collect();
            let scan = scanned(&mine, &modified);
            let rows: Vec<serde_json::Value> = scan
                .hits
                .iter()
                .map(|h| {
                    serde_json::json!({
                        "file": h.file,
                        "line": h.line,
                        "text": h.text,
                        "severity": h.severity.as_str(),
                    })
                })
                .collect();
            let mut out = Vec::new();
            // An empty list is not a block list to the host (E061): a phase with none has the field unset.
            if !rows.is_empty() {
                out.push(value(verification, "anti_patterns", rows.into()));
            }
            if scan.partial {
                out.push(value(verification, "scan_partial", true.into()));
            }
            out
        })
        .collect()
}

/// The plans a summary says were executed. E505 judges those only (upstream judges debt markers when it verifies an
/// executed phase): a plan being written is never refused for a file it names (C2).
fn executed<'a>(input: &'a PassInput, plans: Vec<&'a PassEntity>) -> Vec<&'a PassEntity> {
    plans
        .into_iter()
        .filter(|plan| {
            of_kind(input, "summary").any(|s| link(input, s, "plan") == Some(plan.id.as_str()))
        })
        .collect()
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let modified = reads(input, "modified");
    let plans = executed(input, plans_sorted(input));
    scanned(&plans, &modified)
        .hits
        .into_iter()
        .filter(|h| h.severity == Severity::Blocker)
        .map(|h| {
            PassDiagnostic::new(
                "E505",
                PassSeverity::Error,
                format!(
                    "{}:{} has a {} debt marker and the line names no issue: {}",
                    h.file, h.line, h.marker, h.text
                ),
            )
            .with_suggestion(
                "name the follow-up on the same line (`#123`, `PR #123`, `DEF-12`) or resolve the marker (GSD: a debt marker is a blocker)",
            )
            .with_entity(&h.plan)
        })
        .collect()
}

#[cfg(test)]
mod tests;
