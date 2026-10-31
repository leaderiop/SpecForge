//! The 72 commands of gsd-core v1.18.0 and who serves each (D-r43-14): the table "Who owns each of upstream's 72
//! commands" as code. A rendered text may name a `/gsd-<x>` only when `x` is a row here, and `tests/roster.rs` checks
//! the rows against the vendored `commands.json` and the README's Commands table. The routers and `/gsd-next` read the
//! slash commands the project serves, not this table (ADR 0316).
//!
//! Also ports gsd-core bin/lib modules: command-roster.cjs.

#![allow(dead_code, reason = "each test binary uses some of it")]

/// Who serves an upstream command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// A command of `@gsd/core`, served as a command-backed slash command.
    CoreCommand,
    /// A prompt of `@gsd/core`, served as a prompt-backed slash command.
    CorePrompt,
    /// One of the six `/gsd-ns-*` routers of `@gsd/core`.
    Router,
    /// A capability extension that is not part of `@gsd/core`.
    Extension(&'static str),
}

impl Owner {
    /// `@gsd/core` serves the command.
    pub fn is_core(self) -> bool {
        !matches!(self, Owner::Extension(_))
    }
}

/// Upstream's `commands/gsd/` stems, sorted, each with its owner.
pub static ROSTER: &[(&str, Owner)] = &[
    ("add-tests", Owner::CorePrompt),
    ("ai-integration-phase", Owner::Extension("@gsd/ai")),
    ("audit-fix", Owner::Extension("@gsd/ops")),
    ("audit-milestone", Owner::CorePrompt),
    ("audit-uat", Owner::CorePrompt),
    ("autonomous", Owner::CorePrompt),
    ("capture", Owner::Extension("@gsd/ops")),
    ("cleanup", Owner::CoreCommand),
    ("code-review", Owner::Extension("@gsd/ops")),
    ("complete-milestone", Owner::CorePrompt),
    ("config", Owner::CorePrompt),
    ("debug", Owner::Extension("@gsd/ops")),
    ("discuss-phase", Owner::CorePrompt),
    ("docs-update", Owner::Extension("@gsd/codebase")),
    ("eval-review", Owner::Extension("@gsd/ai")),
    ("execute-phase", Owner::CorePrompt),
    ("explore", Owner::Extension("@gsd/ops")),
    ("extract-learnings", Owner::CorePrompt),
    ("fast", Owner::CorePrompt),
    ("forensics", Owner::Extension("@gsd/ops")),
    ("graphify", Owner::Extension("@gsd/codebase")),
    ("health", Owner::CoreCommand),
    ("help", Owner::CoreCommand),
    ("import", Owner::Extension("@gsd/import")),
    ("inbox", Owner::Extension("@gsd/ops")),
    ("ingest-docs", Owner::CorePrompt),
    ("manager", Owner::CorePrompt),
    ("map-codebase", Owner::Extension("@gsd/codebase")),
    ("mempalace-capture", Owner::Extension("@gsd/ops")),
    ("mempalace-recall", Owner::Extension("@gsd/ops")),
    ("milestone-summary", Owner::CorePrompt),
    ("mvp-phase", Owner::CorePrompt),
    ("new-milestone", Owner::CorePrompt),
    ("new-project", Owner::CorePrompt),
    ("next", Owner::CorePrompt),
    ("ns-context", Owner::Router),
    ("ns-ideate", Owner::Router),
    ("ns-manage", Owner::Router),
    ("ns-project", Owner::Router),
    ("ns-review", Owner::Router),
    ("ns-workflow", Owner::Router),
    ("onboard", Owner::CorePrompt),
    ("pause-work", Owner::CorePrompt),
    ("phase", Owner::CoreCommand),
    ("plan-phase", Owner::CorePrompt),
    ("plan-review-convergence", Owner::CorePrompt),
    ("pr-branch", Owner::CorePrompt),
    ("profile-user", Owner::Extension("@gsd/ops")),
    ("progress", Owner::CorePrompt),
    ("quick-batch", Owner::CorePrompt),
    ("quick", Owner::CorePrompt),
    ("resume-work", Owner::CorePrompt),
    ("review-backlog", Owner::CorePrompt),
    ("review", Owner::CorePrompt),
    ("secure-phase", Owner::Extension("@gsd/security")),
    ("settings", Owner::CorePrompt),
    ("ship", Owner::CorePrompt),
    ("sketch", Owner::Extension("@gsd/ops")),
    ("spec-phase", Owner::CorePrompt),
    ("spike", Owner::Extension("@gsd/ops")),
    ("stats", Owner::CoreCommand),
    ("surface", Owner::CoreCommand),
    ("thread", Owner::Extension("@gsd/ops")),
    ("ui-phase", Owner::Extension("@gsd/ui")),
    ("ui-review", Owner::Extension("@gsd/ui")),
    ("ultraplan-phase", Owner::CorePrompt),
    ("undo", Owner::CorePrompt),
    ("update", Owner::CorePrompt),
    ("validate-phase", Owner::CorePrompt),
    ("verify-work", Owner::CorePrompt),
    ("workspace", Owner::CorePrompt),
    ("workstreams", Owner::CoreCommand),
];

/// The owner of the upstream command `stem` (`plan-phase`).
pub fn owner(stem: &str) -> Option<Owner> {
    ROSTER.iter().find(|(n, _)| *n == stem).map(|(_, o)| *o)
}
