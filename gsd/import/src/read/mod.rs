//! The readers: each served `.planning/` file goes to the reader of its format, or to one report entry with the fate
//! the table gives it ([`crate::table`]).

pub mod requirements;

use crate::preflight::Manifest;
use crate::records::Records;
use crate::report::{Entry, Outcome, Report};
use crate::settings;
use crate::table::{self, Fate, TIER_2};

/// One file the host served under `.planning/`: its project-relative path and what it read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Served {
    pub path: String,
    pub text: Option<String>,
    /// The host's reason it sent no text (`too_large`, `denied`, ..).
    pub error: Option<String>,
}

/// Reads `served` into `records`, or reports it whole with its fate.
pub fn file(served: &Served, manifest: &Manifest, records: &mut Records, report: &mut Report) {
    let rel = served
        .path
        .strip_prefix(".planning/")
        .unwrap_or(&served.path);
    let Some((name, fate)) = table::fate_of(rel) else {
        report.push(whole(served, Outcome::Unmapped, "unknown file".to_string()));
        return;
    };
    match (name, fate, &served.text) {
        ("REQUIREMENTS.md", _, Some(text)) => {
            requirements::read(&served.path, text, records, report)
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
