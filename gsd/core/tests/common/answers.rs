//! The members a `specforge gsd <command>` answer can print, for each command whose answer an asset or a prompt picks
//! (ADR 0343 D3). `--pick` of a member an answer lacks prints nothing and exits 0 (ADR 0325 D2), so a pick of a member
//! no answer ever prints reads as "none" on every run. The table is a test oracle: `ported_text` holds every pick to
//! it, and the cli harness holds it to every JSON answer the tests read. A row is the union over the command's modes.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

/// `(command as typed after specforge gsd, every root member its JSON answer can print)`, sorted by command.
pub const ANSWERS: &[(&str, &[&str])] = &[
    (
        "branch-check",
        &[
            "branch",
            "detached",
            "protected",
            "agent",
            "unreadable",
            "blocks_commit",
        ],
    ),
    (
        "gate-command",
        &["target", "scope", "source", "argv", "note"],
    ),
    ("milestone-branches", &["milestone", "strategy", "branches"]),
    (
        "phase",
        &[
            "wrote",
            "phase",
            "set",
            "promoted",
            "to",
            "removed",
            "deleted",
            "renumbered",
        ],
    ),
    (
        "progress-report",
        &[
            "milestone_version",
            "milestone_name",
            "phases",
            "total_plans",
            "total_summaries",
            "percent",
            "phase_scope",
        ],
    ),
    ("quick-init", &["items"]),
    ("record-conflict", &["conflict", "reused", "wrote"]),
    ("security-hooks", &["activeHooks", "skippedHooks"]),
    ("start-plan", &["plan", "started", "head", "recorded"]),
    (
        "user-story-validate",
        &["valid", "errors", "role", "capability", "outcome"],
    ),
    (
        "verification-debt",
        &[
            "milestone",
            "current",
            "archived",
            "archived_total",
            "blockers",
        ],
    ),
    (
        "workstreams",
        &[
            "workstreams",
            "active",
            "wrote",
            "workstream",
            "complete",
            "open",
            "active_workstream",
        ],
    ),
    ("worktree-contains", &["path", "top", "inside"]),
];

/// The members `command`'s answer can print, when the table has its row.
pub fn members(command: &str) -> Option<&'static [&'static str]> {
    ANSWERS
        .iter()
        .find(|(name, _)| *name == command)
        .map(|(_, members)| *members)
}
