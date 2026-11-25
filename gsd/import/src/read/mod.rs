//! The readers: each served `.planning/` file goes to the reader of its format, or to one report entry with the fate
//! the table gives it ([`crate::table`]).

pub mod context;
pub mod discussion;
pub mod plan;
pub mod project;
pub mod quick;
pub mod requirements;
pub mod research;
pub mod roadmap;
pub mod security;
pub mod state;
pub mod summary;
pub mod tasks;
pub mod threats;
pub mod uat;
pub mod verification;

use std::collections::BTreeMap;
use std::ops::Range;

use gsd_kit::markdown::InlineBlock;
use serde_json::{Value, json};

use crate::ids;
use crate::paths;
use crate::preflight::Manifest;
use crate::records::{Record, Records, Source};
use crate::report::{Entry, Outcome, Report};
use crate::sections::Section;
use crate::settings;
use crate::table::{self, Fate, Home, TIER_2};
use plan::{Doc, Scope};

/// One file the host served under `.planning/`: its project-relative path and what it read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Served {
    pub path: String,
    pub text: Option<String>,
    /// The host's reason it sent no text (`too_large`, `denied`, ..).
    pub error: Option<String>,
}

/// What a reader knows beyond its own file.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    pub manifest: &'a Manifest,
    /// The roadmap's phases and plan titles (empty while `ROADMAP.md` itself is read).
    pub roadmap: &'a roadmap::Roadmap,
    /// Every served path, relative to `.planning/`.
    pub served: &'a [&'a str],
}

/// The path of `served` relative to `.planning/`.
fn relative(served: &Served) -> &str {
    served
        .path
        .strip_prefix(".planning/")
        .unwrap_or(&served.path)
}

/// Whether `served` is read after every other file: a summary takes its plan's title, a research names its phase
/// context's decisions, a security audit sets its phase's plan threats.
fn is_late(served: &&Served) -> bool {
    let rel = relative(served);
    ["-SUMMARY.md", "-RESEARCH.md", "-SECURITY.md"]
        .iter()
        .any(|end| rel.ends_with(end))
}

/// Reads every served file: `REQUIREMENTS.md` first (a phase keeps only the requirements it defines), then
/// `ROADMAP.md` (later readers take its phases and plan titles), then the rest in the order served, the summaries,
/// researches and security audits last, then the Traceability check of `REQUIREMENTS.md` against the roadmap. The roadmap read is returned
/// for the readers after it.
pub fn all(
    planning: &[Served],
    manifest: &Manifest,
    records: &mut Records,
    report: &mut Report,
) -> roadmap::Roadmap {
    let find = |name: &str| planning.iter().find(|s| relative(s) == name);
    let (requirements, road) = (find("REQUIREMENTS.md"), find("ROADMAP.md"));
    let served: Vec<&str> = planning.iter().map(relative).collect();
    let empty = roadmap::Roadmap::default();
    let early = Context {
        manifest,
        roadmap: &empty,
        served: &served,
    };
    if let Some(served) = requirements {
        file(served, &early, records, report);
    }
    // The directories under `phases/`, from the files the host served in them (a `.gitkeep` keeps an empty one).
    let mut dirs: Vec<&str> = planning
        .iter()
        .filter_map(|s| relative(s).strip_prefix("phases/")?.split_once('/'))
        .map(|(dir, _)| dir)
        .collect();
    dirs.dedup();
    let roadmap = match road {
        Some(Served {
            path,
            text: Some(text),
            ..
        }) => roadmap::read(path, text, &dirs, records, report),
        Some(served) => {
            file(served, &early, records, report);
            roadmap::Roadmap::default()
        }
        None => roadmap::Roadmap::default(),
    };
    let cx = Context {
        roadmap: &roadmap,
        ..early
    };
    let rest = || {
        planning
            .iter()
            .filter(|s| !matches!(relative(s), "REQUIREMENTS.md" | "ROADMAP.md"))
    };
    for served in rest().filter(|s| !is_late(s)).chain(rest().filter(is_late)) {
        file(served, &cx, records, report);
    }
    if let (
        Some(Served {
            path,
            text: Some(text),
            ..
        }),
        Some(_),
    ) = (requirements, road)
    {
        roadmap::cross_check(path, text, &roadmap, report);
    }
    roadmap
}

/// Reads `served` into `records`, or reports it whole with its fate.
pub fn file(served: &Served, cx: &Context<'_>, records: &mut Records, report: &mut Report) {
    let manifest = cx.manifest;
    let Some((name, fate)) = table::fate_of(relative(served)) else {
        report.push(whole(served, Outcome::Unmapped, "unknown file".to_string()));
        return;
    };
    match (name, fate, &served.text) {
        ("REQUIREMENTS.md", _, Some(text)) => {
            requirements::read(&served.path, text, records, report)
        }
        ("PROJECT.md", _, Some(text)) => project::read(&served.path, text, records, report),
        ("NN-MM-PLAN.md", _, Some(text)) => {
            plan::read_phase(&served.path, relative(served), text, cx, records, report)
        }
        ("NN-MM-SUMMARY.md", _, Some(text)) => {
            summary::read_phase(&served.path, relative(served), text, cx, records, report)
        }
        ("quick/*", _, Some(text)) => {
            quick::read(&served.path, relative(served), text, cx, records, report)
        }
        ("NN-CONTEXT.md", _, Some(text)) => {
            context::read(&served.path, relative(served), text, cx, records, report)
        }
        ("NN-DISCUSSION-LOG.md", _, Some(text)) => {
            discussion::read(&served.path, relative(served), text, cx, records, report)
        }
        ("NN-RESEARCH.md", _, Some(text)) => {
            research::read(&served.path, relative(served), text, cx, records, report)
        }
        ("NN-VERIFICATION.md", _, Some(text)) => {
            verification::read(&served.path, relative(served), text, cx, records, report)
        }
        ("NN-UAT.md", _, Some(text)) => {
            uat::read(&served.path, relative(served), text, cx, records, report)
        }
        ("STATE.md", _, Some(text)) => {
            report.push(whole(
                served,
                table::outcome(fate),
                table::reason(name, fate),
            ));
            state::read(&served.path, text, report);
        }
        ("config.json", Fate::Settings, Some(text)) => {
            let (lines, entries) = settings::settings_lines(text, manifest);
            let reason = format!("{} settings lines, printed below", lines.len());
            report.push(whole(served, Outcome::Setting, reason));
            for line in lines {
                report.setting(line);
            }
            for entry in entries {
                report.push(entry);
            }
        }
        ("SECURITY.md", Fate::Maps, _) if !manifest.security => {
            let fate = Fate::NeedsExtension("@gsd/security");
            report.push(whole(
                served,
                table::outcome(fate),
                table::reason(name, fate),
            ));
        }
        ("SECURITY.md", Fate::Maps, Some(text)) => {
            security::read(&served.path, relative(served), text, cx, records, report)
        }
        // The extension is loaded, but this importer does not read its formats.
        (_, Fate::NeedsExtension(extension), _)
            if manifest.loaded.iter().any(|l| l == extension) =>
        {
            let fate = Fate::Unmapped(TIER_2);
            report.push(whole(
                served,
                table::outcome(fate),
                table::reason(name, fate),
            ));
        }
        _ => report.push(whole(
            served,
            table::outcome(fate),
            table::reason(name, fate),
        )),
    }
}

/// One entry about the whole of `served`, naming the host's read error, if any.
fn whole(served: &Served, outcome: Outcome, mut reason: String) -> Entry {
    if let Some(error) = &served.error {
        reason.push_str(&format!(" (not read: {error})"));
    }
    Entry::file(&served.path, outcome, reason)
}

/// The entry of the table row `key` of the format `file`, at `source` in `section`: its first home that writes nothing,
/// as an outcome and its reason. A reader reports with it, so the reason is the table's, not the reader's.
pub fn row(file: &str, key: &str, source: &Source, section: &str) -> Entry {
    let home = table::section(file, key).and_then(|row| {
        row.homes
            .iter()
            .find(|h| !matches!(h, Home::To { .. }))
            .copied()
    });
    let (outcome, reason) = match home {
        Some(Home::Computed { kind, field }) => (
            Outcome::Computed,
            format!("{kind}.{field} is computed: never written"),
        ),
        Some(Home::Redundant(why)) => (Outcome::Redundant, why.to_string()),
        Some(Home::Reported(why)) => (Outcome::Unmapped, why.to_string()),
        Some(Home::Ignored(why)) => (Outcome::Ignored, why.to_string()),
        Some(Home::Runtime(why)) => (Outcome::Runtime, why.to_string()),
        Some(Home::Filled { why, .. }) => (Outcome::Filled, why.to_string()),
        Some(Home::To { .. }) | None => (
            Outcome::Unmapped,
            format!("no row of the {file} table reports {key}"),
        ),
    };
    Entry::at(source, section, outcome, reason)
}

/// The entry of a heading whose section a reader maps (into `to`: `project.core_value`, `each row a decision`), so no
/// heading goes unreported (the census of `tests/census.rs`).
pub fn mapped(source: &Source, section: &str, to: &str) -> Entry {
    Entry::at(source, section, Outcome::Mapped, format!("mapped: {to}"))
}

/// `text` as an id: ASCII letters and digits lower-cased, every other run of characters one `-`, none at either end
/// (`Hotfix: titles survive a reload` is `hotfix-titles-survive-a-reload`).
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// The lines the section `s` holds itself: its body up to its first sub-section (`all` are the file's sections, in line
/// order). A sub-section is read under its own heading, never into its parent's field.
pub fn own(s: &Section, all: &[Section]) -> Range<usize> {
    let end = all
        .iter()
        .find(|h| h.line > s.line)
        .map_or(s.body.end, |h| h.line.min(s.body.end));
    s.body.start..end
}

/// The leaf blocks whose first line lies in `lines`.
pub fn blocks_in<'b>(
    blocks: &'b [InlineBlock],
    lines: &Range<usize>,
) -> impl Iterator<Item = &'b InlineBlock> {
    blocks
        .iter()
        .filter(move |b| b.lines.first().is_some_and(|l| lines.contains(l)))
}

/// The prose of `lines`: each leaf block's text, the blocks joined by a blank line; none when empty.
pub fn prose(blocks: &[InlineBlock], lines: &Range<usize>) -> Option<String> {
    let text: Vec<&str> = blocks_in(blocks, lines).map(|b| b.text.trim()).collect();
    (!text.is_empty()).then(|| text.join("\n\n"))
}

/// Whether the raw line `line` starts a list item (`- `, `* `, `+ `).
pub fn is_bullet(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ")
}

/// Whether the raw line `line` is a template footer line, wholly in emphasis (`*Phase: 02-persistence*`).
pub fn is_footer(line: &str) -> bool {
    let t = line.trim();
    t.len() > 2
        && t.starts_with('*')
        && t.ends_with('*')
        && !t.starts_with("**")
        && !t.starts_with("* ")
}

/// Whether `line` is one XML section tag alone (`<decisions>`, `</domain>`), which only wraps the Markdown inside it.
pub fn is_tag(line: &str) -> bool {
    let t = line.trim();
    let name = t
        .strip_prefix("</")
        .or_else(|| t.strip_prefix('<'))
        .and_then(|t| t.strip_suffix('>'))
        .map(|t| t.split(' ').next().unwrap_or(t));
    name.is_some_and(|n| {
        n.starts_with(|c: char| c.is_ascii_alphabetic())
            && n.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

/// Whether the raw line `line` is wholly bold: a template's instruction to agents
/// (`**Downstream agents MUST read these before planning or implementing.**`).
pub fn is_bold_only(line: &str) -> bool {
    let t = line.trim();
    t.len() > 4 && t.starts_with("**") && t.ends_with("**") && !t[2..t.len() - 2].contains("**")
}

/// The leaf blocks of `blocks` in `body` that a raw list marker starts (`lines` are the file's raw lines).
pub fn bullets_in<'b>(
    blocks: &'b [InlineBlock],
    lines: &[&str],
    body: &Range<usize>,
) -> Vec<&'b InlineBlock> {
    blocks_in(blocks, body)
        .filter(|b| lines.get(b.lines[0]).is_some_and(|l| is_bullet(l)))
        .collect()
}

/// The text after the bold label `label` (`**What:**`) on the first raw line of `body` that starts with it, outside a
/// fence, trimmed, with its 0-based line.
pub fn labelled<'l>(
    lines: &[&'l str],
    fenced: &[bool],
    body: &Range<usize>,
    labels: &[&str],
) -> Option<(usize, &'l str)> {
    body.clone().filter(|n| !fenced[*n]).find_map(|n| {
        let line = lines.get(n)?.trim_start();
        labels
            .iter()
            .find_map(|l| line.strip_prefix(l))
            .map(|v| (n, v.trim()))
    })
}

/// Whether `value` is a `YYYY-MM-DD` date of the calendar (`2026-13-45` is not): the host refuses a plan holding one
/// that is not.
pub fn is_date(value: &str) -> bool {
    let b = value.as_bytes();
    let shaped = b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        });
    let n = |r: Range<usize>| value[r].parse::<u32>().unwrap_or(0);
    shaped && {
        let (year, month, day) = (n(0..4), n(5..7), n(8..10));
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            1..=12 => 31,
            _ => 0,
        };
        (1..=days).contains(&day)
    }
}

/// Whether `value` is a timestamp the gsd family's rule E107 accepts on an authored `ts` field: exactly
/// `gsd_kit::dsl::RFC3339_UTC` (`YYYY-MM-DDTHH:MM:SS`, an optional fraction, `Z`; an uppercase `T`, no offset), which
/// the unit test holds this equal to. The import never writes a value `check` turns into an Error.
pub fn is_timestamp(value: &str) -> bool {
    let b = value.as_bytes();
    let digits = |r: Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let two = |at: usize| (b[at] - b'0') * 10 + (b[at + 1] - b'0');
    let Some(rest) = value.strip_suffix('Z') else {
        return false;
    };
    let fraction = match rest.get(19..) {
        Some("") => true,
        Some(f) => f.len() > 1 && f.starts_with('.') && f[1..].bytes().all(|c| c.is_ascii_digit()),
        None => false,
    };
    fraction
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && b[10] == b'T'
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16)
        && b[16] == b':'
        && digits(17..19)
        && (1..=12).contains(&two(5))
        && (1..=31).contains(&two(8))
        && two(11) <= 23
        && two(14) <= 59
        && two(17) <= 60
}

/// A file of one phase, `phases/<dir>/<NN><suffix>` (`02-CONTEXT.md`): where its records go.
#[derive(Debug, Clone)]
pub struct PhaseFile<'a> {
    pub doc: Doc<'a>,
    /// The phase number (`02`).
    pub number: String,
    /// The phase's graph path (`v1.0/02`).
    pub phase: String,
    /// The file stem, the record's own id (`02-CONTEXT`).
    pub stem: String,
}

impl PhaseFile<'_> {
    /// The 0-based `line` of the file.
    pub fn at(&self, line: usize) -> Source {
        self.doc.at(line + 1)
    }

    /// The table's entry for its key `key`, at the 0-based `line`, in `section`.
    pub fn row(&self, key: &str, line: usize, section: &str) -> Entry {
        self.doc.row(key, line + 1, section)
    }

    /// An `unmapped` entry at the 0-based `line`.
    pub fn unmapped(&self, line: usize, section: &str, why: String) -> Entry {
        self.doc.unmapped(line + 1, section, why)
    }

    /// The [`mapped`] entry of the heading at the 0-based `line`.
    pub fn mapped(&self, line: usize, section: &str, to: &str) -> Entry {
        mapped(&self.at(line), section, to)
    }

    /// The `spec/gsd/` file of the phase.
    pub fn file(&self) -> String {
        paths::phase(&self.number)
    }

    /// A record of the phase file, scoped by the entity `scope` through its field `field`: its id is `<scope>/<id>`.
    pub fn within(
        &self,
        kind: &'static str,
        (field, scope): (&str, &str),
        id: &str,
        title: &str,
        mut fields: BTreeMap<String, Value>,
        line: usize,
    ) -> Record {
        fields.insert(field.to_string(), json!(scope));
        Record {
            kind,
            id: format!("{scope}/{id}"),
            title: title.to_string(),
            file: self.file(),
            fields,
            refs: vec![scope.to_string()],
            links: Vec::new(),
            source: self.at(line),
        }
    }
}

/// The phase file of the format `format` (`NN-CONTEXT.md`) served at `path` (`rel` under `.planning/`); none, reported,
/// when its name gives no phase number or the roadmap holds no such phase.
pub fn phase_file<'a>(
    path: &'a str,
    rel: &str,
    format: &'static str,
    cx: &Context<'_>,
    report: &mut Report,
) -> Option<PhaseFile<'a>> {
    let doc = Doc {
        path,
        format,
        prefix: "",
    };
    // `NN-CONTEXT.md` names its number; `SECURITY.md` is the kit's name of `NN-SECURITY.md`.
    let suffix = format
        .strip_prefix("NN")
        .map_or_else(|| format!("-{format}"), str::to_string);
    let Some((dir, number)) = plan::in_phase(rel, &suffix).filter(|(_, n)| ids::phase(n)) else {
        report.push(doc.unmapped(
            1,
            "file name",
            "the file name gives no phase number `NN`: not imported".to_string(),
        ));
        return None;
    };
    match plan::phase_scope(&doc, dir, number, cx, report)? {
        Scope::Phase { number, path } => Some(PhaseFile {
            doc,
            stem: format!("{number}{}", suffix.trim_end_matches(".md")),
            number,
            phase: path,
        }),
        Scope::Quick => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_timestamps_are_the_calendars() {
        for ok in ["2026-10-30", "2024-02-29", "2026-12-31"] {
            assert!(is_date(ok), "{ok}");
        }
        for bad in [
            "2026-13-45",
            "2026-02-29",
            "2026-04-31",
            "2026-00-10",
            "26-10-30",
            "2026/10/30",
        ] {
            assert!(!is_date(bad), "{bad}");
        }
        for ok in [
            "2026-03-03T10:30:00Z",
            "2026-03-03T23:59:60.123Z",
            "2026-02-31T00:00:00Z",
        ] {
            assert!(is_timestamp(ok), "{ok}");
        }
        for bad in [
            "2026-03-03",
            "yesterday",
            "2026-03-03T24:00:00Z",
            "2026-03-03T10:30Z",
            "2026-03-03T10:30:00",
            "2026-03-03T10:30:00.5+01:00",
            "2026-03-03t23:59:60z",
            "2026-03-03T10:30:00z",
            "2026-03-03T10:30:00.Z",
        ] {
            assert!(!is_timestamp(bad), "{bad}");
        }
    }

    /// `is_timestamp` is E107's `RFC3339_UTC`, value for value: the import writes no `ts` that `check` refuses.
    #[test]
    fn a_timestamp_is_what_e107_accepts() {
        let e107 = regex::Regex::new(gsd_kit::dsl::RFC3339_UTC).expect("a pattern");
        let mut values = vec![
            String::new(),
            "Z".to_string(),
            "2026-10-28 09:30:00Z".to_string(),
            "\u{662}\u{660}\u{662}\u{666}-10-28T09:30:00Z".to_string(),
            "2026-10-28T09:30:00Zx".to_string(),
            "2026-10-28T09:30:00.1234567Z".to_string(),
            "2026-10-28T09:30:00+00:00".to_string(),
            "2026-10-28T09:30:0éZ".to_string(),
        ];
        for (month, day, hour, minute, second) in [
            ("00", "10", "09", "30", "00"),
            ("13", "10", "09", "30", "00"),
            ("12", "00", "09", "30", "00"),
            ("12", "31", "09", "30", "00"),
            ("12", "32", "09", "30", "00"),
            ("02", "30", "20", "59", "60"),
            ("02", "30", "24", "00", "00"),
            ("02", "30", "23", "60", "00"),
            ("02", "30", "23", "59", "61"),
            ("1a", "30", "23", "59", "59"),
        ] {
            let at = format!("2026-{month}-{day}T{hour}:{minute}:{second}");
            values.extend([format!("{at}Z"), format!("{at}.5Z"), format!("{at}z")]);
        }
        for value in values {
            assert_eq!(is_timestamp(&value), e107.is_match(&value), "{value:?}");
        }
    }
}
