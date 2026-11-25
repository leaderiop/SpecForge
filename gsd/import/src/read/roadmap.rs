//! `ROADMAP.md` to milestones and phases (13-RESEARCH Part B, the `ROADMAP.md` table; the rows are
//! `table::sections::ROADMAP`), in both of GSD's forms:
//!
//! - **Flat** (GSD's initial roadmap): no `## Milestones` section, so one milestone `v1.0` holds every phase, its id
//!   reported `filled` and its title authored after the import (ADR 0391 D4).
//! - **Milestone-grouped** (after v1.0 ships): a `## Milestones` bullet per version (`- ✅ **v1.0 MVP** - ...`); a
//!   phase belongs to the milestone whose `<details><summary>` block or `### <emoji> vX.Y Name` heading holds it, and
//!   `**Milestone Goal:**` gives that milestone's goal.
//!
//! A phase heading (`### Phase 02.1: Name (INSERTED)`, `#### Phase 5: Name`) gives the phase, its id the number with the
//! integer part on two digits; its `**Goal**`, `**Depends on**`, `**Requirements**` and `**Success Criteria**` lines its
//! fields. A `Plans:` bullet gives a plan's title to the plan reader. Every other section is reported.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use super::{is_bullet, row, slug};
use crate::ids::{self, padded};
use crate::paths;
use crate::records::{Record, Records, Source};
use crate::report::{Entry, Outcome, Report};
use crate::sections::{self, Section};

const RM: &str = "ROADMAP.md";

/// The flat roadmap's one milestone.
const FLAT: &str = "v1.0";

/// What a section no row names becomes.
const UNKNOWN: &str = "a section no row of the ROADMAP.md table names: no field holds it";

/// What later readers take from the roadmap.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Roadmap {
    /// Each phase's number (`01`, `02.1`) and its graph path (`v1.0/01`).
    pub phases: BTreeMap<String, String>,
    /// Each `Plans:` bullet's plan id (`01-01`) and its title, backticks cut.
    pub plan_titles: BTreeMap<String, String>,
    /// Each requirement a phase's `**Requirements**` names, and that phase's number.
    pub requirements: BTreeMap<String, String>,
}

/// One phase as its heading and lines give it, before its references are resolved.
struct Draft {
    number: String,
    milestone: String,
    title: String,
    /// The heading as written: the section of the phase's report entries.
    heading: String,
    line: usize,
    fields: BTreeMap<String, Value>,
    depends: Option<(String, usize)>,
    requirements: Option<(String, usize)>,
}

/// The classes reported once per file, not at each line.
#[derive(Default)]
struct Once(BTreeSet<&'static str>);

impl Once {
    fn push(&mut self, key: &'static str, source: &Source, section: &str, report: &mut Report) {
        if self.0.insert(key) {
            report.push(row(RM, key, source, section));
        }
    }
}

/// Reads `text`, served at `path`; `dirs` are the names of the directories under `.planning/phases/`, which give the
/// phases' slugs. The requirement records must be read first: a phase keeps only the requirements they hold.
pub fn read(
    path: &str,
    text: &str,
    dirs: &[&str],
    records: &mut Records,
    report: &mut Report,
) -> Roadmap {
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let fenced = sections::fenced(text);
    let all = sections::sections(text);
    let at = |line: usize| Source {
        file: path.to_string(),
        line: line + 1,
    };
    let mut once = Once::default();
    let mut roadmap = Roadmap::default();
    let grouped = all.iter().find(|s| s.level == 2 && s.title == "Milestones");
    let mut current: Option<String> = None;
    let mut milestone_goal: BTreeMap<String, (String, usize)> = BTreeMap::new();
    let mut found = match grouped {
        Some(s) => milestones(s, &lines, &fenced, &at, &mut once, report),
        None => {
            let line = all.iter().find(|s| s.level == 1).map_or(0, |s| s.line);
            report.push(row(RM, "(no milestone)", &at(line), "(no milestone)"));
            report.authored_after_import(format!("milestone {FLAT}: its title"));
            current = Some(FLAT.to_string());
            vec![milestone(FLAT, FLAT, at(line))]
        }
    };
    let headings: BTreeMap<usize, &Section> = all.iter().map(|s| (s.line, s)).collect();
    let mut drafts: Vec<Draft> = Vec::new();
    let mut covered = 0..0;
    for (n, raw) in lines.iter().enumerate() {
        // A phase heading is read wherever it stands: a section reported whole never swallows the phases.
        let phase_heading = headings
            .get(&n)
            .is_some_and(|s| s.level >= 3 && s.title.starts_with("Phase"));
        if fenced[n] || (covered.contains(&n) && !phase_heading) {
            continue;
        }
        if phase_heading {
            covered = 0..0;
        }
        if let Some(s) = headings.get(&n) {
            let title = s.title.as_str();
            match (s.level, title) {
                // Any title (`# Roadmap: X`, `# Project Roadmap`): its spelling is not load-bearing.
                (1, _) => report.push(row(RM, "# Roadmap:", &at(n), title)),
                (2, "Overview") => report.push(row(RM, "## Overview", &at(n), title)),
                (2, "Phases") => report.push(row(RM, "## Phases", &at(n), title)),
                (2, "Phase Details") => report.push(row(RM, "## Phase Details", &at(n), title)),
                (2, "Progress") => {
                    covered = s.body.clone();
                    report.push(row(RM, "## Progress", &at(n), title));
                }
                (2, "Milestones") if grouped.is_some() => {}
                (3.., _) if title.starts_with("Phase") => {
                    let draft = phase(s, &lines, &fenced, &at, &mut once, &mut roadmap, report);
                    match (draft, &current) {
                        (Some(draft), Some(milestone)) => drafts.push(Draft {
                            milestone: milestone.clone(),
                            ..draft
                        }),
                        (Some(_), None) => report.push(Entry::at(
                            &at(n),
                            title,
                            Outcome::Unmapped,
                            "a phase outside any milestone's heading or <details> block: not imported".to_string(),
                        )),
                        (None, _) => {}
                    }
                }
                (3.., _) if grouped.is_some() && version(title).is_some() => {
                    current = opens(title, &at(n), title, report);
                    if current.is_some() {
                        once.push("milestone status", &at(n), title, report);
                    }
                }
                _ => {
                    covered = s.body.clone();
                    report.push(Entry::at(
                        &at(n),
                        title,
                        Outcome::Unmapped,
                        UNKNOWN.to_string(),
                    ));
                }
            }
            continue;
        }
        let line = raw.trim();
        if grouped.is_some() && line.contains("<summary>") {
            if version(line).is_some() {
                current = opens(line, &at(n), "<details>", report);
                if current.is_some() {
                    once.push("milestone status", &at(n), "<details>", report);
                }
            }
        } else if let Some(("Milestone Goal", goal)) = key_line(line) {
            match &current {
                Some(m) if !goal.is_empty() => {
                    milestone_goal.insert(m.clone(), (goal.to_string(), n));
                }
                _ => report.push(Entry::at(
                    &at(n),
                    "**Milestone Goal:**",
                    Outcome::Unmapped,
                    "a milestone goal outside a milestone, or empty: not imported".to_string(),
                )),
            }
        } else if let Some(("Phase Numbering", _)) = key_line(line) {
            report.push(row(RM, "Phase Numbering", &at(n), "Phase Numbering"));
        }
    }
    for (id, (goal, n)) in milestone_goal {
        match found.iter_mut().find(|m| m.id == id) {
            Some(m) => {
                m.fields.insert("goal".to_string(), json!(goal));
            }
            None => report.push(Entry::at(
                &at(n),
                "**Milestone Goal:**",
                Outcome::Unmapped,
                format!("milestone {id} is in no ## Milestones bullet: its goal is not imported"),
            )),
        }
    }
    for m in found {
        records.add(m, report);
    }
    // Every phase's path is known: resolve the references.
    for d in &drafts {
        roadmap
            .phases
            .entry(d.number.clone())
            .or_insert(format!("{}/{}", d.milestone, d.number));
    }
    for mut d in drafts {
        // The phase directory `NN-slug` gives the slug, else the title does (13-RESEARCH Pitfall 9).
        let dir = dirs.iter().find_map(|dir| {
            let (number, slug) = dir.split_once('-')?;
            (padded(number).as_deref() == Some(d.number.as_str())).then_some(slug)
        });
        let slug = dir.map_or_else(|| slug(&d.title), str::to_string);
        if !slug.is_empty() {
            d.fields.insert("slug".to_string(), json!(slug));
        }
        let mut links = Vec::new();
        if let Some((value, n)) = &d.depends {
            let deps = depends(value, *n, &roadmap, &at, &d.heading, report);
            links.extend(deps.iter().map(|id| ("depends_on", id.clone())));
            if !deps.is_empty() {
                d.fields.insert("depends_on".to_string(), json!(deps));
            }
        }
        if let Some((value, n)) = &d.requirements {
            let mut kept = Vec::new();
            for id in requirement_ids(value) {
                if id == "TBD" {
                    report.push(row(RM, "TBD", &at(*n), &d.heading));
                } else if records.contains("requirement", id) {
                    roadmap
                        .requirements
                        .insert(id.to_string(), d.number.clone());
                    kept.push(id.to_string());
                } else {
                    report.push(Entry::at(
                        &at(*n),
                        &d.heading,
                        Outcome::Unmapped,
                        format!("**Requirements** names {id}, which REQUIREMENTS.md does not define: left out of the phase"),
                    ));
                }
            }
            links.extend(kept.iter().map(|id| ("requirements", id.clone())));
            if !kept.is_empty() {
                d.fields.insert("requirements".to_string(), json!(kept));
            }
        }
        d.fields.insert("milestone".to_string(), json!(d.milestone));
        records.add(
            Record {
                kind: "phase",
                id: format!("{}/{}", d.milestone, d.number),
                title: d.title,
                file: paths::milestone(&d.milestone),
                fields: d.fields,
                refs: vec![d.milestone.clone()],
                links,
                source: at(d.line),
            },
            report,
        );
    }
    roadmap
}

/// A milestone record, its goal added later.
fn milestone(id: &str, title: &str, source: Source) -> Record {
    Record {
        kind: "milestone",
        id: id.to_string(),
        title: title.to_string(),
        file: paths::milestone(id),
        fields: BTreeMap::new(),
        refs: Vec::new(),
        links: Vec::new(),
        source,
    }
}

/// The `## Milestones` bullets: one milestone per version, its title the bold text.
fn milestones(
    s: &Section,
    lines: &[&str],
    fenced: &[bool],
    at: &dyn Fn(usize) -> Source,
    once: &mut Once,
    report: &mut Report,
) -> Vec<Record> {
    let mut out = Vec::new();
    for n in s.body.clone() {
        let Some(line) = lines.get(n).filter(|l| !fenced[n] && is_bullet(l)) else {
            continue;
        };
        let Some(v) = version(line).filter(|v| ids::milestone(v)) else {
            report.push(Entry::at(
                &at(n),
                "Milestones",
                Outcome::Unmapped,
                "a milestone bullet names no version vX.Y: not imported".to_string(),
            ));
            continue;
        };
        let title = line
            .split_once("**")
            .and_then(|(_, rest)| rest.split_once("**"))
            .map_or(v, |(bold, _)| bold.trim());
        once.push("milestone status", &at(n), "Milestones", report);
        out.push(milestone(v, title, at(n)));
    }
    out
}

/// The milestone a version heading or `<summary>` line `text` opens: its version, when a milestone takes it as an id
/// (as a `## Milestones` bullet's is checked); else reported, and the phases under it stand in no milestone.
fn opens(text: &str, source: &Source, section: &str, report: &mut Report) -> Option<String> {
    let v = version(text)?;
    if ids::milestone(v) {
        return Some(v.to_string());
    }
    report.push(Entry::at(
        source,
        section,
        Outcome::Unmapped,
        format!("`{v}` names no version vX.Y: the phases under it are not imported"),
    ));
    None
}

/// The first word of `text` that is a version `v<digit>...` (`v1.0`), without trailing punctuation.
fn version(text: &str) -> Option<&str> {
    text.split(|c: char| c.is_whitespace() || matches!(c, '*' | '(' | ')' | '>' | '<'))
        .map(|w| w.trim_end_matches([':', ',', '.', ';']))
        .find(|w| {
            w.strip_prefix('v')
                .is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()))
        })
}

/// `**Key**: value`, `**Key:** value` or `**Key** (note): value`, as the key and the value, trimmed.
fn key_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim_start().strip_prefix("**")?;
    let (key, after) = rest.split_once("**")?;
    match key.strip_suffix(':') {
        Some(key) => Some((key.trim(), after.trim())),
        None => after.split_once(':').map(|(_, v)| (key.trim(), v.trim())),
    }
}

/// The phase of the heading `s`, read from its lines, with no milestone yet; none, reported, when the heading is in
/// neither form.
fn phase(
    s: &Section,
    lines: &[&str],
    fenced: &[bool],
    at: &dyn Fn(usize) -> Source,
    once: &mut Once,
    roadmap: &mut Roadmap,
    report: &mut Report,
) -> Option<Draft> {
    let refuse = |why: String, report: &mut Report| {
        report.push(Entry::at(
            &at(s.line),
            &s.title,
            Outcome::Unmapped,
            format!("{why}: not imported"),
        ));
    };
    let Some((number, name)) = s
        .title
        .strip_prefix("Phase")
        .and_then(|r| r.split_once(':'))
    else {
        refuse(
            "a phase heading in neither form `Phase N: Name`".to_string(),
            report,
        );
        return None;
    };
    let number = number.trim();
    if number.is_empty() {
        refuse("a phase heading with no number".to_string(), report);
        return None;
    }
    let Some(id) = padded(number) else {
        refuse(format!("`{number}` is not a phase number"), report);
        return None;
    };
    let mut title = name.trim();
    if let Some(cut) = title.strip_suffix("(INSERTED)") {
        title = cut.trim_end();
        once.push("(INSERTED)", &at(s.line), &s.title, report);
    }
    let mut d = Draft {
        number: id,
        milestone: String::new(),
        title: title.to_string(),
        heading: s.title.clone(),
        line: s.line,
        fields: BTreeMap::new(),
        depends: None,
        requirements: None,
    };
    let mut n = s.body.start;
    while n < s.body.end {
        let line = lines[n];
        let here = n;
        n += 1;
        if fenced[here] {
            continue;
        }
        if line.trim() == "Plans:" {
            while n < s.body.end && (is_bullet(lines[n]) || lines[n].trim().is_empty()) {
                plan_title(lines[n], n, at, &s.title, roadmap, report);
                n += 1;
            }
            continue;
        }
        let Some((key, value)) = key_line(line) else {
            continue;
        };
        match key {
            "Goal" if !value.is_empty() => {
                d.fields.insert("goal".to_string(), json!(value));
            }
            "Depends on" => d.depends = Some((value.to_string(), here)),
            "Requirements" => d.requirements = Some((value.to_string(), here)),
            "Success Criteria" => {
                let mut criteria = Vec::new();
                while let Some(item) = lines.get(n).and_then(|l| numbered(l)) {
                    criteria.push(item.to_string());
                    n += 1;
                }
                if !criteria.is_empty() {
                    d.fields
                        .insert("success_criteria".to_string(), json!(criteria));
                }
            }
            "Plans" => once.push("**Plans**: N/M plans complete", &at(here), &s.title, report),
            "Mode" => report.push(row(RM, "**Mode**", &at(here), &s.title)),
            _ => report.push(Entry::at(
                &at(here),
                &s.title,
                Outcome::Unmapped,
                format!("a phase line **{key}** no row names: no field holds it"),
            )),
        }
    }
    Some(d)
}

/// A numbered list item's text (`  1. A user can add a task`), verbatim.
fn numbered(line: &str) -> Option<&str> {
    let t = line.trim_start();
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    let rest = t[digits..].strip_prefix(". ")?;
    (digits > 0 && !rest.trim().is_empty()).then(|| rest.trim())
}

/// A `Plans:` bullet `- [x] 01-01: Title`: the plan's title for the plan reader; `TBD (run ...)` is ignored.
fn plan_title(
    line: &str,
    n: usize,
    at: &dyn Fn(usize) -> Source,
    section: &str,
    roadmap: &mut Roadmap,
    report: &mut Report,
) {
    if !is_bullet(line) {
        return;
    }
    let item = line.trim_start()[2..].trim_start();
    let item = ["[x] ", "[X] ", "[ ] "]
        .iter()
        .find_map(|b| item.strip_prefix(b))
        .unwrap_or(item);
    if item.starts_with("TBD") {
        report.push(row(RM, "TBD (run /gsd-plan-phase", &at(n), section));
        return;
    }
    match item.split_once(": ") {
        Some((id, title)) if ids::plan(id.trim()) => {
            roadmap.plan_titles.insert(
                id.trim().to_string(),
                title.replace('`', "").trim().to_string(),
            );
        }
        _ => report.push(Entry::at(
            &at(n),
            section,
            Outcome::Unmapped,
            "a Plans: bullet names no plan `NN-MM: Title`: no title taken".to_string(),
        )),
    }
}

/// The graph paths of the phases `**Depends on**` names (`Phase 2`, `Phase 1, Phase 2`); `Nothing (first phase)` is
/// ignored, and a phase the roadmap does not hold is reported.
fn depends(
    value: &str,
    n: usize,
    roadmap: &Roadmap,
    at: &dyn Fn(usize) -> Source,
    section: &str,
    report: &mut Report,
) -> Vec<String> {
    if value.starts_with("Nothing") || value.is_empty() {
        report.push(row(RM, "Nothing (first phase)", &at(n), section));
        return Vec::new();
    }
    let mut out = Vec::new();
    for part in value.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let word = part.strip_prefix("Phase").unwrap_or(part).trim();
        let number: String = word
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        match padded(number.trim_end_matches('.')).and_then(|p| roadmap.phases.get(&p)) {
            Some(path) => out.push(path.clone()),
            None => report.push(Entry::at(
                &at(n),
                section,
                Outcome::Unmapped,
                format!(
                    "**Depends on** names `{part}`, which is no phase of the roadmap: left out"
                ),
            )),
        }
    }
    out
}

/// The ids of a `**Requirements**` value: `[A-01, B-02]` or `A-01, B-02`.
fn requirement_ids(value: &str) -> impl Iterator<Item = &str> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
}

/// Reports each row of REQUIREMENTS.md's Traceability table (served at `path`) that disagrees with the roadmap's
/// `**Requirements**` lines: the roadmap wins (13-RESEARCH Pitfall 8).
pub fn cross_check(path: &str, text: &str, roadmap: &Roadmap, report: &mut Report) {
    for table in gsd_kit::markdown::tables(text)
        .into_iter()
        .filter(|t| t.heading == "Traceability")
    {
        let column = |name: &str, or: usize| {
            table
                .header
                .iter()
                .position(|h| h.eq_ignore_ascii_case(name))
                .unwrap_or(or)
        };
        let (requirement, phase) = (column("Requirement", 0), column("Phase", 1));
        for (i, cells) in table.rows.iter().enumerate() {
            let cell = |i: usize| cells.get(i).map(|c| c.trim()).unwrap_or_default();
            let named = cell(phase)
                .strip_prefix("Phase")
                .unwrap_or(cell(phase))
                .trim();
            let roadmap_phase = roadmap.requirements.get(cell(requirement));
            let why = match (padded(named), roadmap_phase) {
                (Some(p), Some(r)) if p == *r => continue,
                (_, Some(r)) => format!(
                    "Traceability puts {} in `{}`, the roadmap's **Requirements** in phase {r}: the roadmap wins",
                    cell(requirement),
                    cell(phase)
                ),
                (_, None) => format!(
                    "Traceability puts {} in `{}`, and no phase's **Requirements** names it: the roadmap wins",
                    cell(requirement),
                    cell(phase)
                ),
            };
            let source = Source {
                file: path.to_string(),
                line: table.line + 3 + i,
            };
            report.push(Entry::at(&source, "Traceability", Outcome::Redundant, why));
        }
    }
}
