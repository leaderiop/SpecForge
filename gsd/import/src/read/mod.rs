//! The readers: each served `.planning/` file goes to the reader of its format, or to one report entry.

pub mod requirements;

use gsd_kit::files::{PLANNING_FILES, planning_names};

use crate::records::Records;
use crate::report::{Entry, Outcome, Report};

/// One file the host served under `.planning/`: its project-relative path and what it read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Served {
    pub path: String,
    pub text: Option<String>,
    /// The host's reason it sent no text (`too_large`, `denied`, ..).
    pub error: Option<String>,
}

/// Reads `served` into `records`, or reports it.
pub fn file(served: &Served, records: &mut Records, report: &mut Report) {
    let rel = served
        .path
        .strip_prefix(".planning/")
        .unwrap_or(&served.path);
    match (rel, &served.text) {
        ("REQUIREMENTS.md", Some(text)) => requirements::read(&served.path, text, records, report),
        _ => {
            let mut reason = kinds_of(rel).unwrap_or("unknown file").to_string();
            if let Some(error) = &served.error {
                reason.push_str(&format!(" (not read: {error})"));
            }
            report.push(Entry::file(&served.path, Outcome::Unmapped, reason));
        }
    }
}

/// The kit's kinds column for the row of `PLANNING_FILES` that names the whole of `rel` (a path under `.planning/`).
pub fn kinds_of(rel: &str) -> Option<&'static str> {
    let (_, row) = planning_names(rel)
        .into_iter()
        .find(|(range, _)| *range == (0..rel.len()))?;
    PLANNING_FILES
        .iter()
        .find(|(name, _)| *name == row)
        .map(|(_, kinds)| *kinds)
}
