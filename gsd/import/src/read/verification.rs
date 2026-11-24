//! `NN-VERIFICATION.md` to a `verification` (13-RESEARCH Part B, the VERIFICATION table; the rows are
//! `table::sections::VERIFICATION`): the file stem is its id and its phase its only field. Everything else the file
//! records is the verifier's: the frontmatter's status, score and coverage are computed, and the Observable Truths,
//! Required Artifacts and Key Link tables are verdicts, never imported (ADR 0391 D5): the report lists them owed
//! instead ([`crate::owed`]). An `override` is a person's value, written in an editor.

use std::collections::BTreeMap;

use super::plan::{entries, front};
use super::{Context, is_footer, phase_file};
use crate::records::Records;
use crate::report::Report;
use crate::sections;

/// The fate table's name of a verification report.
pub const VF: &str = "NN-VERIFICATION.md";

const UNKNOWN: &str = "a section no row of the NN-VERIFICATION.md table names: not imported";

/// The frontmatter keys the table names, each reported with its row.
const KEYS: &[&str] = &[
    "phase",
    "status",
    "score",
    "verified",
    "covered_files",
    "covered_digest",
    "re_verification",
    "anti_patterns",
    "scan_partial",
    "behavior_unverified",
];

/// The header lines over the first section, and the row each reports with.
const HEADER: &[(&str, &str)] = &[
    ("**Phase Goal:**", "**Phase Goal:**"),
    ("**Verified:**", "verified"),
    ("**Status:**", "status"),
    ("**Score:**", "score"),
];

/// Reads the verification report `text`, served at `path` (`rel` under `.planning/`):
/// `phases/<NN-slug>/<NN>-VERIFICATION.md`.
pub fn read(
    path: &str,
    rel: &str,
    text: &str,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let Some(f) = phase_file(path, rel, VF, cx, report) else {
        return;
    };
    let (front, start) = front(&f.doc, text, report);
    for (key, v) in entries(&front) {
        let line = v.line;
        match key.as_str() {
            k if KEYS.contains(&k) => report.push(f.doc.row(k, line, k)),
            "override" | "overrides" => report.push(f.doc.row("override", line, key)),
            _ => report.push(f.doc.unmapped(
                line,
                key,
                format!("a key `{key}` no row of the NN-VERIFICATION.md table names: not imported"),
            )),
        }
    }
    let body = crate::frontmatter::body(text, start);
    let lines: Vec<&str> = body.split('\n').collect();
    let all = sections::sections(&body);
    let first = all
        .iter()
        .find(|s| s.level >= 2)
        .map_or(lines.len(), |s| s.line);
    let fenced = sections::fenced(&body);
    for (n, line) in lines.iter().enumerate() {
        if fenced[n] {
            continue;
        }
        let t = line.trim_start();
        if let Some((_, key)) = HEADER.iter().find(|(label, _)| t.starts_with(label)) {
            if n < first {
                report.push(f.row(key, n, key));
            }
        } else if is_footer(line) {
            report.push(f.row("footer", n, "footer"));
        }
    }
    let mut title = None;
    let mut at = 0;
    for s in &all {
        let gaps_none = || {
            lines[s.body.clone()]
                .iter()
                .any(|l| l.contains("No gaps found"))
        };
        match (s.level, s.title.as_str()) {
            (1, _) if title.is_none() => {
                title = Some(s.title.clone());
                at = s.line;
            }
            (2, "Goal Achievement") => report.push(f.row("## Goal Achievement", s.line, &s.title)),
            (
                3,
                "Observable Truths"
                | "Required Artifacts"
                | "Key Link Verification"
                | "Requirements Coverage",
            ) => report.push(f.row(&format!("### {}", s.title), s.line, &s.title)),
            (2, "Gaps Summary" | "Gaps") if gaps_none() => {
                report.push(f.row("No gaps found", s.line, &s.title))
            }
            (2, "Gaps Summary" | "Gaps") => report.push(f.row("## Gaps Summary", s.line, &s.title)),
            (2, "Human Verification Required") => {
                report.push(f.row("## Human Verification Required", s.line, &s.title))
            }
            _ => report.push(f.unmapped(s.line, &s.title, UNKNOWN.to_string())),
        }
    }
    records.add(
        f.within(
            "verification",
            ("phase", &f.phase),
            &f.stem,
            title.as_deref().unwrap_or(&f.stem),
            BTreeMap::new(),
            at,
        ),
        report,
    );
}
