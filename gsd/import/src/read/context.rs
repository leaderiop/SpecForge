//! `NN-CONTEXT.md` to a `context`, its phase's decisions and its deferred ideas (13-RESEARCH Part B, the CONTEXT table;
//! the rows are `table::sections::CONTEXT`): the file stem is the context's id, its sections its fields, each
//! `- **D-NN:** text` under a `### <Area>` of `## Implementation Decisions` a `decision D-NN` and each `## Deferred Ideas`
//! bullet a `deferred_idea DI<n>`, all scoped by the phase. Values are kept as written (trimmed).

use std::collections::BTreeMap;
use std::ops::Range;

use gsd_kit::markdown::{self, InlineBlock};
use serde_json::{Value, json};

use super::{
    Context, PhaseFile, Rest, blocks_in, bullets_in, is_bold_only, is_date, is_footer, is_tag,
    mapped, own, phase_file, prose,
};
use crate::ids;
use crate::records::Records;
use crate::report::Report;
use crate::sections::{self, Section};

/// The fate table's name of a phase context.
pub const CX: &str = "NN-CONTEXT.md";

/// What a section no row names becomes.
const UNKNOWN: &str = "a section no row of the NN-CONTEXT.md table names: no field holds it";

const DECISIONS: &str = "Implementation Decisions";
const INSIGHTS: &str = "Existing Code Insights";

/// Whether `title` is `Claude's Discretion`, with either apostrophe.
pub fn discretion(title: &str) -> bool {
    matches!(title, "Claude's Discretion" | "Claude’s Discretion")
}

/// One file's raw lines, their fence marks and its leaf blocks less the template's own lines.
pub struct Text<'t> {
    pub lines: Vec<&'t str>,
    pub fenced: Vec<bool>,
    pub blocks: Vec<InlineBlock>,
}

impl<'t> Text<'t> {
    pub fn new(text: &'t str) -> Self {
        let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
        // A heading is a section, read by its title, never a block of the section over it.
        let headings: Vec<usize> = markdown::headings(text).iter().map(|h| h.line).collect();
        let blocks = markdown::inline_blocks(text)
            .into_iter()
            .filter(|b| !headings.contains(&b.lines[0]))
            .filter_map(|mut b| {
                // A closing tag on the line under a paragraph is inline HTML of that paragraph (`B. </domain>`).
                let mut kept = b.text.trim_end();
                while let Some((head, last)) = kept.rsplit_once(char::is_whitespace) {
                    if !is_tag(last) {
                        break;
                    }
                    kept = head.trim_end();
                }
                if kept.split('\n').all(|l| l.trim().is_empty() || is_tag(l)) {
                    return None;
                }
                let len = kept.len();
                b.text.truncate(len);
                b.lines.truncate(len);
                lines
                    .get(b.lines[0])
                    .is_none_or(|l| !is_footer(l) && !is_bold_only(l))
                    .then_some(b)
            })
            .collect();
        Text {
            fenced: sections::fenced(text),
            lines,
            blocks,
        }
    }

    /// The trimmed text of each bullet of `body`, with its 0-based line.
    pub fn bullets(&self, body: &Range<usize>) -> Vec<(usize, &str)> {
        bullets_in(&self.blocks, &self.lines, body)
            .into_iter()
            .map(|b| (b.lines[0], b.text.trim()))
            .collect()
    }

    /// The raw lines outside a fence, with their 0-based numbers.
    pub fn raw(&self) -> impl Iterator<Item = (usize, &&'t str)> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(n, _)| !self.fenced[*n])
    }
}

/// Reads the context `text`, served at `path` (`rel` under `.planning/`): `phases/<NN-slug>/<NN>-CONTEXT.md`.
pub fn read(
    path: &str,
    rel: &str,
    text: &str,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let Some(f) = phase_file(path, rel, CX, cx, report) else {
        return;
    };
    let t = Text::new(text);
    let mut fields: BTreeMap<String, Value> = BTreeMap::new();
    let mut rest = Rest::default();
    for (n, line) in t.raw() {
        if let Some(v) = line.strip_prefix("**Gathered:**").map(str::trim) {
            rest.take(n);
            if is_date(v) {
                fields.insert("gathered".to_string(), json!(v));
            } else {
                report.push(f.unmapped(
                    n,
                    "Gathered",
                    "**Gathered:** is not a YYYY-MM-DD date: left out".to_string(),
                ));
            }
        } else if line.starts_with("**Status:**") {
            report.push(f.row("**Status:**", n, "Status"));
        } else if is_footer(line) {
            report.push(f.row("footer", n, "footer"));
        } else if is_bold_only(line) {
            report.push(f.row("**Downstream agents MUST read these", n, "<canonical_refs>"));
        }
    }
    let mut title = None;
    let mut parent = String::new();
    let mut covered = 0..0;
    let mut decided = false;
    let all = sections::sections(text);
    for s in &all {
        if covered.contains(&s.line) {
            continue;
        }
        if s.level <= 2 {
            parent = s.title.clone();
        }
        let to =
            |what: &str, report: &mut Report| report.push(mapped(&f.at(s.line), &s.title, what));
        // What the section holds itself: a sub-section is read under its own heading.
        let mine = own(s, &all);
        let list = |field: &str, fields: &mut BTreeMap<String, Value>, report: &mut Report| {
            to(&format!("context.{field}"), report);
            let items: Vec<&str> = t.bullets(&mine).into_iter().map(|(_, b)| b).collect();
            if !items.is_empty() {
                fields.insert(field.to_string(), json!(items));
            }
        };
        let prose_of = |field: &str, fields: &mut BTreeMap<String, Value>, report: &mut Report| {
            to(&format!("context.{field}"), report);
            if let Some(text) = prose(&t.blocks, &mine) {
                fields.insert(field.to_string(), json!(text));
            }
        };
        // A section read in part: its bullets are taken, and what else it holds is reported.
        let in_part = match (s.level, parent.as_str(), s.title.as_str()) {
            (1, ..) => title.is_none(),
            (2, _, DECISIONS | INSIGHTS) => true,
            (3, DECISIONS, _) => true,
            (3, INSIGHTS, name) => {
                matches!(
                    name,
                    "Reusable Assets" | "Established Patterns" | "Integration Points"
                )
            }
            _ => false,
        };
        if in_part {
            rest.section(mine.clone(), &s.title);
            for b in bullets_in(&t.blocks, &t.lines, &mine) {
                rest.take_block(b);
            }
        }
        match (s.level, parent.as_str(), s.title.as_str()) {
            (1, ..) if title.is_none() => title = Some(s.title.clone()),
            (2, _, "Phase Boundary") => prose_of("boundary", &mut fields, report),
            (2, _, "Specific Ideas") => prose_of("specifics", &mut fields, report),
            (2, _, DECISIONS | INSIGHTS) => to("each area under it", report),
            (2, _, "Canonical References") => {
                to("context.canonical_refs", report);
                covered = s.body.clone();
                let refs = canonical_refs(&t, s, &all);
                if !refs.is_empty() {
                    fields.insert("canonical_refs".to_string(), json!(refs));
                }
            }
            (2, _, "Deferred Ideas") => {
                to("each bullet a deferred_idea", report);
                covered = s.body.clone();
                deferred(&f, &t, s, records, report);
            }
            (3, DECISIONS, title) if discretion(title) => list("discretion", &mut fields, report),
            (3, DECISIONS, _) => {
                to("each D-NN bullet a decision", report);
                let area = Section {
                    body: mine.clone(),
                    ..s.clone()
                };
                decisions(&f, &t, &area, &mut decided, records, report)
            }
            (3, INSIGHTS, "Reusable Assets") => list("reusable_assets", &mut fields, report),
            (3, INSIGHTS, "Established Patterns") => list("patterns", &mut fields, report),
            (3, INSIGHTS, "Integration Points") => list("integration_points", &mut fields, report),
            _ => {
                if s.level > 1 {
                    covered = s.body.clone();
                }
                report.push(f.unmapped(s.line, &s.title, UNKNOWN.to_string()));
            }
        }
    }
    records.add(
        f.within(
            "context",
            ("phase", &f.phase),
            &f.stem,
            title.as_deref().unwrap_or(&f.stem),
            fields,
            0,
        ),
        report,
    );
    rest.report(path, text, CX, report);
}

/// Each `- **D-NN:** text` bullet of the area `s` as `decision D-NN` of the phase, origin `context`; any other bullet,
/// or an id that is no `D-NN`, is reported. The first decision of the file reports the filled origin.
fn decisions(
    f: &PhaseFile<'_>,
    t: &Text<'_>,
    s: &Section,
    decided: &mut bool,
    records: &mut Records,
    report: &mut Report,
) {
    for (line, text) in t.bullets(&s.body) {
        let Some((id, rest)) = text
            .split_once(':')
            .filter(|(id, _)| id.trim().starts_with("D-"))
        else {
            report.push(f.unmapped(
                line,
                &s.title,
                "a decision bullet with no `**D-NN:**` id: not imported".to_string(),
            ));
            continue;
        };
        let (id, rest) = (id.trim(), rest.trim());
        if !ids::decision(id) || rest.is_empty() {
            report.push(f.unmapped(
                line,
                &s.title,
                format!("`{id}` is no decision `D-NN` with a text: not imported"),
            ));
            continue;
        }
        if !std::mem::replace(decided, true) {
            report.push(f.row("- **D-NN:**", line, &s.title));
        }
        let fields = BTreeMap::from([
            ("area".to_string(), json!(s.title)),
            ("text".to_string(), json!(rest)),
            ("origin".to_string(), json!("context")),
        ]);
        records.add(
            f.within("decision", ("phase", &f.phase), id, rest, fields, line),
            report,
        );
    }
}

/// Each `## Deferred Ideas` bullet `- text — why` as `deferred_idea DI<n>` of the phase, origin `discussion`; a bullet
/// with no ` — ` is imported without a `why`, reported.
fn deferred(
    f: &PhaseFile<'_>,
    t: &Text<'_>,
    s: &Section,
    records: &mut Records,
    report: &mut Report,
) {
    for (n, (line, text)) in t.bullets(&s.body).into_iter().enumerate() {
        if n == 0 {
            report.push(f.row("<deferred>", line, &s.title));
        }
        let mut fields = BTreeMap::from([("origin".to_string(), json!("discussion"))]);
        let idea = match text.split_once(" — ") {
            Some((idea, why)) if !why.trim().is_empty() => {
                fields.insert("why".to_string(), json!(why.trim()));
                idea.trim()
            }
            _ => {
                report.push(f.unmapped(
                    line,
                    &s.title,
                    "a deferred idea with no ` — why`: imported without a why".to_string(),
                ));
                text
            }
        };
        fields.insert("text".to_string(), json!(idea));
        records.add(
            f.within(
                "deferred_idea",
                ("phase", &f.phase),
                &format!("DI{}", n + 1),
                idea,
                fields,
                line,
            ),
            report,
        );
    }
}

/// The `<canonical_refs>` blocks of `s`: one `{topic, paths}` per bullet or paragraph, its code spans without a space
/// the `paths` and the rest its topic, after the `### <Topic>` heading over it (`all` are the file's sections).
fn canonical_refs(t: &Text<'_>, s: &Section, all: &[Section]) -> Vec<Value> {
    let heading = |line: usize| {
        all.iter()
            .rfind(|h| h.level > s.level && s.body.contains(&h.line) && h.line < line)
            .map(|h| h.title.as_str())
    };
    blocks_in(&t.blocks, &s.body)
        .map(|b| {
            let (rest, paths) = code_paths(b.text.trim());
            let topic = match (heading(b.lines[0]), rest.as_str()) {
                (Some(h), "") => h.to_string(),
                (Some(h), rest) => format!("{h}: {rest}"),
                (None, rest) => rest.to_string(),
            };
            let mut block = json!({ "paths": paths });
            if !topic.is_empty() {
                block["topic"] = json!(topic);
            }
            block
        })
        .collect()
}

/// `text` without its code spans that hold no space, and those spans: an odd count of backticks keeps all as text.
fn code_paths(text: &str) -> (String, Vec<String>) {
    if text.matches('`').count() % 2 == 1 {
        return (text.to_string(), Vec::new());
    }
    let (mut rest, mut paths) = (String::new(), Vec::new());
    for (i, part) in text.split('`').enumerate() {
        if i % 2 == 0 {
            rest.push_str(part);
        } else if !part.is_empty() && !part.contains(char::is_whitespace) {
            paths.push(part.to_string());
        } else {
            rest.push_str(&format!("`{part}`"));
        }
    }
    let rest = rest
        .trim_matches(|c: char| c.is_whitespace() || matches!(c, '—' | '-' | ':' | ',' | ';'))
        .to_string();
    (rest, paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ref_keeps_its_paths_apart_from_its_topic() {
        assert_eq!(
            code_paths("`docs/a.md` — Feed rules"),
            ("Feed rules".to_string(), vec!["docs/a.md".to_string()])
        );
        assert_eq!(
            code_paths("No external specs — fully captured"),
            ("No external specs — fully captured".to_string(), vec![])
        );
        assert_eq!(
            code_paths("`a b` and `x"),
            ("`a b` and `x".to_string(), vec![])
        );
        assert_eq!(
            code_paths("`a b` uses `c.md`"),
            ("`a b` uses".to_string(), vec!["c.md".to_string()])
        );
    }
}
