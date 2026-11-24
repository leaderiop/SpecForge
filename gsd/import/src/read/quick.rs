//! A `quick/<id>-<slug>/` directory (13-RESEARCH Part B, the `quick/` table; the rows are
//! `table::sections::{QUICK, QUICK_PLAN, QUICK_SUMMARY}`): `<id>-PLAN.md` is a plan with no phase, read by
//! [`super::plan`] with [`Scope::Quick`] (its id the file name's, which its `quick_id` repeats); `<id>-SUMMARY.md` its
//! summary, read by [`super::summary`]; a quick `CONTEXT.md` or `RESEARCH.md` is Tier 2 and reported whole.

use super::plan::{self, Doc, Scope};
use super::{Context, row, summary};
use crate::ids;
use crate::records::{Records, Source};
use crate::report::{Entry, Outcome, Report};

/// The fate table's name of a quick directory's files.
pub const QK: &str = "quick/*";

/// Reads the file `text` of a quick directory, served at `path` (`rel` under `.planning/`).
pub fn read(
    path: &str,
    rel: &str,
    text: &str,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let (dir, name) = rel.rsplit_once('/').unwrap_or(("", rel));
    let doc = |prefix| Doc {
        path,
        format: QK,
        prefix,
    };
    let stem = |suffix: &str| name.strip_suffix(suffix).filter(|id| ids::quick(id));
    if let Some(id) = stem("-PLAN.md") {
        plan::read(&doc("PLAN "), text, id, &Scope::Quick, cx, records, report);
    } else if let Some(id) = stem("-SUMMARY.md") {
        let doc = doc("SUMMARY ");
        if summary::has_plan(cx, &format!("{dir}/{id}-PLAN.md"), &doc, report) {
            summary::read(&doc, text, id, &Scope::Quick, cx, records, report);
        }
    } else if let Some(key) = ["CONTEXT", "RESEARCH"]
        .into_iter()
        .find(|k| name.ends_with(&format!("-{k}.md")))
    {
        let mut entry = row(
            QK,
            key,
            &Source {
                file: path.to_string(),
                line: 1,
            },
            "",
        );
        entry.line = None;
        report.push(entry);
    } else {
        report.push(Entry::file(
            path,
            Outcome::Unmapped,
            "a file of a quick directory that is no `<YYMMDD-id>-PLAN.md`, SUMMARY, CONTEXT or RESEARCH: no row names it"
                .to_string(),
        ));
    }
}
