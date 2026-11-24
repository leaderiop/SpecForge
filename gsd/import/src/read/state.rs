//! `STATE.md` maps nothing (13-RESEARCH Part B, the `STATE.md` fate; the rows are `table::sections::STATE`): the graph
//! computes its frontmatter and position, its session continuity is a session's, and each blocker is a Tier 2 entity.
//! Every section is reported with its outcome, so nothing of it is dropped silently.

use gsd_kit::markdown;

use super::{blocks_in, row};
use crate::frontmatter;
use crate::records::Source;
use crate::report::{Entry, Outcome, Report};
use crate::sections;

const ST: &str = "STATE.md";

/// What a section no row names becomes.
const DERIVED: &str =
    "no field holds it: STATE.md is derived from the project's other files and the graph";

/// Reports `text`, served at `path`, section by section.
pub fn read(path: &str, text: &str, report: &mut Report) {
    let at = |line: usize| Source {
        file: path.to_string(),
        line,
    };
    let (front, start) = frontmatter::split(text);
    match front.and_then(|f| f.map(|f| frontmatter::read(&f)).transpose()) {
        Ok(Some(_)) => report.push(row(ST, "---", &at(1), "---")),
        Ok(None) => {}
        Err(refused) => report.push(Entry::at(
            &at(refused.line),
            "---",
            Outcome::Unmapped,
            format!("frontmatter not read: {}", refused.reason),
        )),
    }
    let body = frontmatter::body(text, start);
    let blocks = markdown::inline_blocks(&body);
    let all = sections::sections(&body);
    for (i, s) in all.iter().enumerate() {
        let here = at(s.line + 1);
        match s.title.as_str() {
            "Current Position" | "Session Continuity" if s.level == 2 => {
                report.push(row(ST, &format!("## {}", s.title), &here, &s.title));
            }
            "Blockers/Concerns" if s.level == 3 => {
                // The section's own lines, up to the next heading: a sub-section reports itself.
                let own = s.body.start..all.get(i + 1).map_or(s.body.end, |n| n.line);
                let mut any = false;
                for b in blocks_in(&blocks, &own) {
                    any = true;
                    let key = if b.text.trim() == "None yet." {
                        "None yet."
                    } else {
                        "### Blockers/Concerns"
                    };
                    report.push(row(ST, key, &at(b.lines[0] + 1), &s.title));
                }
                if !any {
                    report.push(row(ST, "None yet.", &here, &s.title));
                }
            }
            _ => report.push(Entry::at(
                &here,
                &s.title,
                Outcome::Unmapped,
                DERIVED.to_string(),
            )),
        }
    }
}
