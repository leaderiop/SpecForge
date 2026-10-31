//! The 28 files of gsd-core 1.18.0's `hooks/` (the inventory manifest's `hooks` family), each with what became of it
//! here (D-r44-1). `tests/hooks.rs` compares the rows with the vendored manifest and with what `declare` contributes.

/// What became of one upstream hook file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Ported: the contributions that carry its behaviour.
    Hooks(&'static [&'static str]),
    /// Not ported, and why.
    Dropped(&'static str),
    /// The Cursor or Windsurf rendering of an event another row ports; the host is named. Cursor's is rendered;
    /// Windsurf is a later host, so its rows are named and open.
    Rendering(&'static str),
}

use Disposition::{Dropped, Hooks, Rendering};

const UPDATES: &str = "specforge updates extensions through its lockfile (/gsd-update, D-r43-16)";
const NO_STOP: &str = "no hook of @gsd/core registers on Stop, SubagentStart or SubagentStop (context.rs), so Cursor has no line to render";
const CURSOR: &str = "cursor";
const WINDSURF: &str = "windsurf";

/// One row per upstream file, in the manifest's order.
pub const HOOK_ROSTER: &[(&str, Disposition)] = &[
    (
        "gsd-agent-isolation-guard.js",
        Dropped(
            "no worktree isolation until the `specforged` harness: @gsd/core runs its executors in the checkout, so the guard would always allow",
        ),
    ),
    ("gsd-check-update-worker.js", Dropped(UPDATES)),
    ("gsd-check-update.js", Dropped(UPDATES)),
    ("gsd-config-reload.js", Hooks(&["config_reload"])),
    (
        "gsd-context-monitor.js",
        Hooks(&["context_monitor", "context_compacted"]),
    ),
    ("gsd-cursor-post-tool.js", Rendering(CURSOR)),
    ("gsd-cursor-pre-tool.js", Rendering(CURSOR)),
    ("gsd-cursor-session-start.js", Rendering(CURSOR)),
    ("gsd-cursor-stop.js", Dropped(NO_STOP)),
    ("gsd-cursor-subagent-start.js", Dropped(NO_STOP)),
    ("gsd-cursor-subagent-stop.js", Dropped(NO_STOP)),
    (
        "gsd-ensure-canonical-path.js",
        Dropped("rendered files include nothing from ~/.claude"),
    ),
    (
        "gsd-graphify-update.sh",
        Dropped("@gsd/codebase: graphify is absorbed, SpecForge is the graph"),
    ),
    (
        "gsd-node-runner.sh",
        Dropped("plumbing of the Node implementation"),
    ),
    ("gsd-phase-boundary.sh", Hooks(&["phase_boundary"])),
    ("gsd-prompt-guard.js", Hooks(&["prompt_guard"])),
    ("gsd-read-guard.js", Hooks(&["read_guard"])),
    (
        "gsd-read-injection-scanner.js",
        Hooks(&["injection_scanner"]),
    ),
    ("gsd-secret-read-guard.js", Hooks(&["secret_read_guard"])),
    ("gsd-session-state.sh", Hooks(&["session_state"])),
    ("gsd-statusline.js", Hooks(&["statusline"])),
    ("gsd-update-banner.js", Dropped(UPDATES)),
    ("gsd-validate-commit.sh", Hooks(&["validate_commit"])),
    ("gsd-windsurf-pre-command.js", Rendering(WINDSURF)),
    ("gsd-windsurf-pre-write.js", Rendering(WINDSURF)),
    // The marker has no upstream file: it closes gap 32, the workflow guard's inability to tell a running workflow.
    (
        "gsd-workflow-guard.js",
        Hooks(&["workflow_marker", "workflow_guard"]),
    ),
    (
        "gsd-worktree-path-guard.js",
        Hooks(&["worktree_path_guard"]),
    ),
    ("gsd-write-guard.js", Hooks(&["write_guard"])),
];
