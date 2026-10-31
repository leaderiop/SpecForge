//! The 78 workflow, reference, mode, step, detail and template items of the inventory manifest that `@gsd/core` does
//! not embed (D-r44-17): each has the extension that owns it, the typed command that replaced it, or a reason.

/// Why an upstream prompt asset is not in `src/prompts/assets.rs`.
#[derive(Debug, Clone, Copy)]
pub enum Why {
    /// Another `@gsd/*` extension serves the command or capability the asset belongs to.
    Owned(&'static str),
    /// A typed `specforge gsd <command>` of `@gsd/core` replaced the prose.
    Command(&'static str),
    /// Not part of `@gsd/core`, with the reason.
    NotApplicable(&'static str),
}

use Why::{Command, NotApplicable, Owned};

const OPS: &str = "@gsd/ops";
const AI: &str = "@gsd/ai";
const CODEBASE: &str = "@gsd/codebase";
const UI: &str = "@gsd/ui";
const UPDATES: &str = "specforge updates extensions through its lockfile (/gsd-update, D-r43-16)";
const SUB_REPOS: &str = "per-repository releases over planning.sub_repos; a sub-repository is another SpecForge project (settings.md)";

/// `(family, item, why)`, in the manifest's order within each family.
pub const NOT_EMBEDDED: &[(&str, &str, Why)] = &[
    ("workflows", "add-backlog.md", Owned(OPS)),
    ("workflows", "add-phase.md", Command("phase")),
    ("workflows", "add-todo.md", Owned(OPS)),
    ("workflows", "ai-integration-phase.md", Owned(AI)),
    ("workflows", "audit-fix.md", Owned(OPS)),
    ("workflows", "check-todos.md", Owned(OPS)),
    ("workflows", "cleanup.md", Command("cleanup")),
    ("workflows", "code-review-fix.md", Owned(OPS)),
    ("workflows", "code-review.md", Owned(OPS)),
    ("workflows", "debug.md", Owned(OPS)),
    ("workflows", "docs-update.md", Owned(CODEBASE)),
    ("workflows", "edit-phase.md", Command("phase")),
    ("workflows", "eval-review.md", Owned(AI)),
    ("workflows", "explore.md", Owned(OPS)),
    ("workflows", "forensics.md", Owned(OPS)),
    ("workflows", "health.md", Command("health")),
    ("workflows", "help.md", Command("help")),
    ("workflows", "import.md", Owned("@gsd/import")),
    ("workflows", "inbox.md", Owned(OPS)),
    ("workflows", "insert-phase.md", Command("phase")),
    ("workflows", "list-seeds.md", Owned(OPS)),
    ("workflows", "map-codebase.md", Owned(CODEBASE)),
    (
        "workflows",
        "node-repair.md",
        NotApplicable(
            "upstream's repair loop for a failed task; a rendered text names it as not part of @gsd/core (NOT_PORTED in tests/ported_text.rs)",
        ),
    ),
    ("workflows", "note.md", Owned(OPS)),
    ("workflows", "plant-seed.md", Owned(OPS)),
    ("workflows", "profile-user.md", Owned(OPS)),
    (
        "workflows",
        "reapply-patches.md",
        NotApplicable(
            "re-merges local edits after an update overwrote installed files; install refuses an edited file (`edited_file`) and `--force` is explicit",
        ),
    ),
    ("workflows", "remove-phase.md", Command("phase")),
    ("workflows", "scan.md", Owned(CODEBASE)),
    ("workflows", "secure-phase.md", Owned("@gsd/security")),
    ("workflows", "sketch-wrap-up.md", Owned(OPS)),
    ("workflows", "sketch.md", Owned(OPS)),
    ("workflows", "spike-wrap-up.md", Owned(OPS)),
    ("workflows", "spike.md", Owned(OPS)),
    ("workflows", "stats.md", Command("stats")),
    (
        "workflows",
        "sync-skills.md",
        NotApplicable(
            "copies skills between runtimes; `specforge install <host>` writes each host's skills from the one source",
        ),
    ),
    ("workflows", "thread.md", Owned(OPS)),
    ("workflows", "ui-phase.md", Owned(UI)),
    ("workflows", "ui-review.md", Owned(UI)),
    ("workflows", "update.md", NotApplicable(UPDATES)),
    ("references", "ai-evals.md", Owned(AI)),
    ("references", "ai-frameworks.md", Owned(AI)),
    ("references", "api-coverage.md", Owned(AI)),
    (
        "references",
        "artifact-types.md",
        NotApplicable(
            "the taxonomy of .planning artifacts; the artifacts here are the kinds of `spec/kinds`",
        ),
    ),
    ("references", "common-bug-patterns.md", Owned(OPS)),
    ("references", "debugger-bug-taxonomy.md", Owned(OPS)),
    ("references", "debugger-fix-acceptance.md", Owned(OPS)),
    ("references", "debugger-philosophy.md", Owned(OPS)),
    ("references", "debugger-prevention.md", Owned(OPS)),
    ("references", "debugger-rca-branching.md", Owned(OPS)),
    ("references", "debugger-repro-hardening.md", Owned(OPS)),
    ("references", "debugger-sbfl.md", Owned(OPS)),
    ("references", "debugger-semantic-recall.md", Owned(OPS)),
    ("references", "debugger-techniques.md", Owned(OPS)),
    (
        "references",
        "decimal-phase-calculation.md",
        Command("phase"),
    ),
    (
        "references",
        "git-planning-commit.md",
        NotApplicable(
            "commits .planning files through gsd-tools' `commit`; spec files are committed with the user's own git and the gsd-git collector reads the commits",
        ),
    ),
    (
        "references",
        "offer-next.md",
        NotApplicable(
            "the auto-advance routing after a phase, extracted from execute-phase; `/gsd-next` (smart_entry) classifies the state and names the next command",
        ),
    ),
    ("references", "phase-id-convention.md", Command("phase")),
    (
        "references",
        "planner-graphify-auto-update.md",
        Owned(CODEBASE),
    ),
    (
        "references",
        "planner-human-verify-mode.md",
        NotApplicable(
            "the planner's rule for `workflow.human_verify_mode`; the setting is declared and the planner agent body carries the rule",
        ),
    ),
    ("references", "sketch-interactivity.md", Owned(OPS)),
    ("references", "sketch-theme-system.md", Owned(OPS)),
    ("references", "sketch-tooling.md", Owned(OPS)),
    ("references", "sketch-variant-patterns.md", Owned(OPS)),
    ("references", "thinking-models-debug.md", Owned(OPS)),
    ("references", "ui-consideration-probe.md", Owned(UI)),
    ("references", "user-profiling.md", Owned(OPS)),
    ("workflow_modes", "help/modes/brief.md", Command("help")),
    ("workflow_modes", "help/modes/default.md", Command("help")),
    (
        "workflow_modes",
        "help/modes/full.compact.md",
        Command("help"),
    ),
    ("workflow_modes", "help/modes/full.md", Command("help")),
    ("workflow_modes", "help/modes/topic.md", Command("help")),
    (
        "workflow_steps",
        "code-review/steps/dispatch-fix.md",
        Owned(OPS),
    ),
    (
        "workflow_steps",
        "code-review/steps/structural-pre-pass.md",
        Owned(OPS),
    ),
    (
        "workflow_steps",
        "complete-milestone/steps/release-map.md",
        NotApplicable(SUB_REPOS),
    ),
    (
        "workflow_steps",
        "docs-update/steps/dispatch-monorepo-packages.md",
        Owned(CODEBASE),
    ),
    (
        "workflow_steps",
        "update/steps/channel-banner.md",
        NotApplicable(UPDATES),
    ),
    (
        "workflow_detail",
        "docs-update/detail/elaboration.md",
        Owned(CODEBASE),
    ),
];
