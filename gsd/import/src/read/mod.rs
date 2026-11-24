//! The readers: each served `.planning/` file goes to the reader of its format, or to one report entry with the fate
//! the table gives it ([`crate::table`]).

pub mod project;
pub mod requirements;
pub mod roadmap;
pub mod state;

use std::ops::Range;

use gsd_kit::markdown::InlineBlock;

use crate::preflight::Manifest;
use crate::records::{Records, Source};
use crate::report::{Entry, Outcome, Report};
use crate::settings;
use crate::table::{self, Fate, Home, TIER_2};

/// One file the host served under `.planning/`: its project-relative path and what it read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Served {
    pub path: String,
    pub text: Option<String>,
    /// The host's reason it sent no text (`too_large`, `denied`, ..).
    pub error: Option<String>,
}

/// The path of `served` relative to `.planning/`.
fn relative(served: &Served) -> &str {
    served
        .path
        .strip_prefix(".planning/")
        .unwrap_or(&served.path)
}

/// Reads every served file: `REQUIREMENTS.md` first (a phase keeps only the requirements it defines), then
/// `ROADMAP.md` (later readers take its phases and plan titles), then the rest in the order served, then the
/// Traceability check of `REQUIREMENTS.md` against the roadmap. The roadmap read is returned for the readers after it.
pub fn all(
    planning: &[Served],
    manifest: &Manifest,
    records: &mut Records,
    report: &mut Report,
) -> roadmap::Roadmap {
    let find = |name: &str| planning.iter().find(|s| relative(s) == name);
    let (requirements, road) = (find("REQUIREMENTS.md"), find("ROADMAP.md"));
    if let Some(served) = requirements {
        file(served, manifest, records, report);
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
            file(served, manifest, records, report);
            roadmap::Roadmap::default()
        }
        None => roadmap::Roadmap::default(),
    };
    for served in planning {
        if !matches!(relative(served), "REQUIREMENTS.md" | "ROADMAP.md") {
            file(served, manifest, records, report);
        }
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
pub fn file(served: &Served, manifest: &Manifest, records: &mut Records, report: &mut Report) {
    let Some((name, fate)) = table::fate_of(relative(served)) else {
        report.push(whole(served, Outcome::Unmapped, "unknown file".to_string()));
        return;
    };
    match (name, fate, &served.text) {
        ("REQUIREMENTS.md", _, Some(text)) => {
            requirements::read(&served.path, text, records, report)
        }
        ("PROJECT.md", _, Some(text)) => project::read(&served.path, text, records, report),
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
