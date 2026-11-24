//! The fate table (ADR 0391 D3, ADR 0372's pattern applied to files): one [`Fate`] for every name of
//! `gsd_kit::files::PLANNING_FILES` ([`FATES`]) and for the three names it lacks ([`EXTRAS`]), and one [`Row`] for every
//! known section, key and table row of the Tier 1 formats ([`SECTIONS`]). A reader implements rows of this table; it
//! does not invent them. The tests count it: a name or a section with no row, or with two, fails.

mod sections;

use gsd_kit::files::{PLANNING_FILES, planning_names};

use crate::report::Outcome;

/// What becomes of a whole file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fate {
    /// Tier 1: a reader maps its sections, row by row ([`SECTIONS`]).
    Maps,
    /// `config.json`: the report prints the `specforge settings set` lines (ADR 0391 D6).
    Settings,
    /// Session or runtime state, which the graph computes; the reason says whose.
    Runtime(&'static str),
    /// Nothing to import; the reason says why.
    Ignored(&'static str),
    /// No reader. [`KIT`] is the kit's own "none: ..." text, read from the kit so the two cannot disagree; any other
    /// reason follows the kit's kinds ([`reason`]).
    Unmapped(&'static str),
    /// Its kinds are this extension's.
    NeedsExtension(&'static str),
    /// Another file gives the same entities; the reason names it.
    Redundant(&'static str),
}

/// The reason of a name whose kit row reads "none: ...": the kit's text, verbatim.
pub const KIT: &str = "";

/// The reason of a Tier 2 name: its kinds exist, the importer does not read the format yet.
pub const TIER_2: &str = "this importer reads the Tier 1 formats of ADR 0391 D3";

/// gsd-tools' session state, over once the project is imported.
const SESSION: &str = "a session's state, over once the project is imported";

/// One fate per `PLANNING_FILES` name, in the kit's order.
pub const FATES: &[(&str, Fate)] = &[
    ("PROJECT.md", Fate::Maps),
    ("SKELETON.md", Fate::Unmapped(TIER_2)),
    ("REQUIREMENTS.md", Fate::Maps),
    ("ROADMAP.md", Fate::Maps),
    ("MILESTONES.md", Fate::Unmapped(TIER_2)),
    (
        "milestones/*",
        Fate::Redundant(
            "an archive copy of a closed milestone's files, which have no counterpart: the live files give its entities",
        ),
    ),
    (
        "STATE.md",
        Fate::Runtime(
            "derived: the graph computes its progress and position, and its session continuity is a session's",
        ),
    ),
    ("RETROSPECTIVE.md", Fate::Unmapped(TIER_2)),
    ("LEARNINGS.md", Fate::Unmapped(TIER_2)),
    ("METHODOLOGY.md", Fate::Unmapped(TIER_2)),
    ("NN-CONTEXT.md", Fate::Maps),
    ("NN-DISCUSSION-LOG.md", Fate::Maps),
    ("NN-DISCUSS-CHECKPOINT.json", Fate::Unmapped(TIER_2)),
    ("NN-SPEC.md", Fate::Unmapped(TIER_2)),
    ("NN-RESEARCH.md", Fate::Maps),
    ("research/*.md", Fate::Unmapped(TIER_2)),
    ("NN-MM-PLAN.md", Fate::Maps),
    ("NN-REVIEWS.md", Fate::Unmapped(TIER_2)),
    ("NN-MM-SUMMARY.md", Fate::Maps),
    (".continue-here.md", Fate::Unmapped(TIER_2)),
    ("NN-VERIFICATION.md", Fate::Maps),
    ("NN-UAT.md", Fate::Maps),
    ("NN-VALIDATION.md", Fate::Unmapped(TIER_2)),
    ("v<version>-MILESTONE-AUDIT.md", Fate::Unmapped(TIER_2)),
    ("WINDOWS.md", Fate::Unmapped(TIER_2)),
    ("NN-REFACTOR.md", Fate::Unmapped(TIER_2)),
    ("quick/*", Fate::Maps),
    ("workstreams/*", Fate::Unmapped(TIER_2)),
    // Mapped only when the project loads `@gsd/security`, decided at read time (ADR 0391 D11).
    ("SECURITY.md", Fate::Maps),
    ("AI-SPEC.md", Fate::NeedsExtension("@gsd/ai")),
    ("NN-EVAL-REVIEW.md", Fate::NeedsExtension("@gsd/ai")),
    ("NN-UI-SPEC.md", Fate::NeedsExtension("@gsd/ui")),
    ("NN-UI-REVIEW.md", Fate::NeedsExtension("@gsd/ui")),
    ("codebase/*", Fate::NeedsExtension("@gsd/codebase")),
    ("intel/*", Fate::NeedsExtension("@gsd/codebase")),
    ("NN-PATTERNS.md", Fate::NeedsExtension("@gsd/codebase")),
    ("debug/*", Fate::NeedsExtension("@gsd/ops")),
    ("NN-REVIEW.md", Fate::NeedsExtension("@gsd/ops")),
    ("todos/*", Fate::NeedsExtension("@gsd/ops")),
    ("seeds/*", Fate::NeedsExtension("@gsd/ops")),
    ("threads/*", Fate::NeedsExtension("@gsd/ops")),
    ("BACKLOG.md", Fate::NeedsExtension("@gsd/ops")),
    ("USER-PROFILE.md", Fate::NeedsExtension("@gsd/ops")),
    ("reports/*.md", Fate::Unmapped(KIT)),
    ("onboarding/SUMMARY.md", Fate::Unmapped(KIT)),
    ("INGEST-CONFLICTS.md", Fate::Unmapped(KIT)),
    ("estimation-calibration.json", Fate::Unmapped(KIT)),
    ("WORKSPACE.md", Fate::Unmapped(KIT)),
    ("MILESTONE_SUMMARY-v*.md", Fate::Unmapped(KIT)),
    ("SESSION_REPORT*.md", Fate::Unmapped(KIT)),
    ("*-session-report.md", Fate::Unmapped(KIT)),
    ("*continue-here*.md", Fate::Unmapped(TIER_2)),
    ("HANDOFF.json", Fate::Unmapped(TIER_2)),
    ("*HANDOFF*.md", Fate::Unmapped(TIER_2)),
    ("agent-history.json", Fate::Runtime(SESSION)),
    ("current-agent-id.txt", Fate::Runtime(SESSION)),
    ("USER-SETUP.md", Fate::Unmapped(TIER_2)),
    ("deferred-items.md", Fate::Unmapped(TIER_2)),
    (
        "DECISIONS-INDEX.md",
        Fate::Redundant(
            "a query of the decisions, not data: the decisions are read where they are made",
        ),
    ),
    ("active-workstream", Fate::Runtime(SESSION)),
    (".gsd-ws-arg", Fate::Runtime(SESSION)),
    (".gsd-outgoing-milestone", Fate::Runtime(SESSION)),
    (".gsd-allow-shrink", Fate::Unmapped(KIT)),
    ("SEED-*.md", Fate::NeedsExtension("@gsd/ops")),
    ("quick-batches/*", Fate::Unmapped(TIER_2)),
    ("ui-reviews/*", Fate::NeedsExtension("@gsd/ui")),
    ("MEMORY.md", Fate::Unmapped(KIT)),
    ("memory/*", Fate::Unmapped(KIT)),
    ("async-jobs/*", Fate::Unmapped(KIT)),
    ("deliberations/*", Fate::Unmapped(KIT)),
    ("graphs/*", Fate::Unmapped(KIT)),
    ("logs/*", Fate::Unmapped(KIT)),
    ("sketches/*", Fate::Unmapped(KIT)),
    ("spikes/*", Fate::Unmapped(KIT)),
    ("tmp/*", Fate::Unmapped(KIT)),
];

/// The names `PLANNING_FILES` lacks that a `.planning/` folder holds, matched by a path's last segment.
pub const EXTRAS: &[(&str, Fate)] = &[
    ("config.json", Fate::Settings),
    (
        "state.json",
        Fate::Runtime("gsd-tools' derived state (`state json`): STATE.md and the graph compute it"),
    ),
    (
        ".gitkeep",
        Fate::Ignored("an empty directory's placeholder"),
    ),
];

/// The name of the `FATES` or `EXTRAS` row the path `rel` (relative to `.planning/`) matches, and its fate: a kit hit
/// spanning the whole path, else an extra by the last segment; none for an unknown file.
pub fn fate_of(rel: &str) -> Option<(&'static str, Fate)> {
    let row = planning_names(rel)
        .into_iter()
        .find(|(range, _)| *range == (0..rel.len()))
        .map(|(_, row)| row);
    let last = rel.rsplit('/').next().unwrap_or(rel);
    match row {
        Some(row) => FATES.iter().find(|(name, _)| *name == row),
        None => EXTRAS.iter().find(|(name, _)| *name == last),
    }
    .copied()
}

/// Whether the file `rel` (under `.planning/`) maps to entities: a read error on one refuses the import (D9).
pub fn maps(rel: &str) -> bool {
    matches!(fate_of(rel), Some((_, Fate::Maps)))
}

/// The kit's kinds column for the `PLANNING_FILES` row `name` (empty for an extra).
pub fn kit_kinds(name: &str) -> &'static str {
    PLANNING_FILES
        .iter()
        .find(|(row, _)| *row == name)
        .map_or("", |(_, kinds)| *kinds)
}

/// The report's outcome for a whole file of fate `fate` that no reader read.
pub fn outcome(fate: Fate) -> Outcome {
    match fate {
        // A Tier 1 file with no reader yet: what its kinds are, unread.
        Fate::Maps | Fate::Unmapped(_) => Outcome::Unmapped,
        Fate::Settings => Outcome::Setting,
        Fate::Runtime(_) => Outcome::Runtime,
        Fate::Ignored(_) => Outcome::Ignored,
        Fate::NeedsExtension(_) => Outcome::NeedsExtension,
        Fate::Redundant(_) => Outcome::Redundant,
    }
}

/// The report's reason for a whole file of the row `name` and fate `fate`.
pub fn reason(name: &str, fate: Fate) -> String {
    let kinds = kit_kinds(name);
    match fate {
        Fate::Maps => kinds.to_string(),
        Fate::Settings => "settings: the report prints the line that sets each".to_string(),
        Fate::Unmapped(KIT) => kinds.to_string(),
        Fate::Unmapped(why) => format!("kinds {kinds} exist; {why}"),
        Fate::NeedsExtension(extension) => {
            format!("kinds {kinds} are {extension}'s, which the project does not load")
        }
        Fate::Runtime(why) | Fate::Ignored(why) | Fate::Redundant(why) => why.to_string(),
    }
}

/// Where one source value goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Home {
    /// Written to the field `field` of a `kind` entity through `how` (a `block.member` field names a member; [`ID`]
    /// and [`TITLE`] write no field).
    To {
        kind: &'static str,
        field: &'static str,
        how: How,
    },
    /// A field the kind computes: never written.
    Computed {
        kind: &'static str,
        field: &'static str,
    },
    /// Another file gives it; the reason names it.
    Redundant(&'static str),
    /// No field holds it, or it is a Tier 2 section: reported with the reason.
    Reported(&'static str),
    /// An empty placeholder or nothing to import.
    Ignored(&'static str),
    /// A field with no `.planning` source, written with `value`, reported `filled` with `why`.
    Filled {
        kind: &'static str,
        field: &'static str,
        value: &'static str,
        why: &'static str,
    },
}

/// How a source value becomes a field's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// The text as written (trimmed): a string, a date, a timestamp, a number.
    Text,
    /// Each item of a list or of the bullets, verbatim.
    List,
    /// Each row or bullet as a block of named members.
    Blocks,
    /// A word read as one of the field's enum values (or a bool).
    Word,
    /// The id of the entity the source names (a phase, a plan, a requirement), as its path.
    Ref,
    /// The entity's id or title: it writes no field.
    Id,
}

/// The field of an [`How::Id`] home that gives the entity's id.
pub const ID: &str = "(id)";
/// The field of a home that gives the entity's title.
pub const TITLE: &str = "(title)";

/// The reason of a Tier 2 section inside a Tier 1 file (ADR 0391 D3).
pub const LATER: &str =
    "a Tier 2 section: its kind exists, and this phase reports it instead of mapping it";

/// One known section, key or table row of a Tier 1 format, and its homes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// The `FATES` name of the format (`NN-MM-PLAN.md`).
    pub file: &'static str,
    /// The section, key or row as the document writes it (`## Core Value`, `depends_on`, `<objective>`).
    pub key: &'static str,
    pub homes: &'static [Home],
}

/// Every Tier 1 format's rows, one `const` per format (a plan's and a summary's shared rows apart, as a quick plan and
/// a quick summary have them too).
pub const SECTIONS: &[&[Row]] = &[
    sections::PROJECT,
    sections::REQUIREMENTS,
    sections::ROADMAP,
    sections::PLAN,
    sections::PLAN_SHARED,
    sections::SUMMARY,
    sections::SUMMARY_SHARED,
    sections::CONTEXT,
    sections::DISCUSSION_LOG,
    sections::RESEARCH,
    sections::VERIFICATION,
    sections::UAT,
    sections::QUICK,
    sections::QUICK_PLAN,
    sections::QUICK_SUMMARY,
    sections::SECURITY,
];

/// Every row of [`SECTIONS`].
pub fn rows() -> impl Iterator<Item = &'static Row> {
    SECTIONS.iter().flat_map(|rows| rows.iter())
}

/// The row of the format `file` for the section or key `key`.
pub fn section(file: &str, key: &str) -> Option<&'static Row> {
    rows().find(|row| row.file == file && row.key == key)
}

#[cfg(test)]
mod tests;
