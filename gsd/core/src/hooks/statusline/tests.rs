//! Ported from gsd-core v1.19.0 tests/gsd-statusline.test.cjs, tests/gsd-statusline-state.property.test.cjs and tests/gsd-statusline-git.property.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The pure parts of the line (the two state formats, the status word, the bar, the token count, the model badge,
//! the meter and the layout) are ported case for case. What read STATE.md, git, the todo files, the update cache or
//! the environment is skipped with its reason; the graph that replaces STATE.md is tested below the port.

use serde_json::{Value, json};

use super::*;
use crate::hooks::accounting;
use crate::hooks::facts::Facts;
use format::{
    Line, StateSeg, compact_model, compose, context_meter, format_compact, format_full,
    format_tokens, progress_bar, short_status, token_suffix,
};

const SUITE: &str = include_str!(gsd_kit::snapshot!("tests/gsd-statusline.test.cjs"));
const STATE_SUITE: &str = include_str!(gsd_kit::snapshot!(
    "tests/gsd-statusline-state.property.test.cjs"
));
const GIT_SUITE: &str = include_str!(gsd_kit::snapshot!(
    "tests/gsd-statusline-git.property.test.cjs"
));

/// A state segment from `key, value` pairs named as upstream's state object names them.
fn seg(pairs: &[(&str, &str)]) -> StateSeg {
    let mut s = StateSeg::default();
    for (k, v) in pairs {
        let num = || v.parse::<f64>().ok();
        match *k {
            "milestone" => s.milestone = v.to_string(),
            "milestoneName" => s.milestone_name = v.to_string(),
            "status" => s.status = v.to_string(),
            "phaseNum" => s.phase_num = v.to_string(),
            "phaseTotal" => s.phase_total = v.to_string(),
            "phaseName" => s.phase_name = v.to_string(),
            "activePhase" => s.active_phase = v.to_string(),
            "nextAction" => s.next_action = v.to_string(),
            "nextPhases" => s.next_phases = v.split(',').map(str::to_string).collect(),
            "percent" => s.percent = num(),
            "completedPhases" => s.completed_phases = num(),
            "totalPhases" => s.total_phases = num(),
            "noActiveWorkstream" => s.no_active_workstream = true,
            other => panic!("no field {other}"),
        }
    }
    s
}

fn full(p: &[(&str, &str)]) -> String {
    format_full(&seg(p))
}

fn compact(p: &[(&str, &str)]) -> String {
    format_compact(&seg(p))
}

fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for d in chars.by_ref() {
                if d == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A status JSON as Claude Code writes it.
fn status(model: Value, size: Option<u64>, remaining: Option<f64>, usage: Value) -> Value {
    let mut w = json!({});
    if let Some(s) = size {
        w["context_window_size"] = json!(s);
    }
    if let Some(r) = remaining {
        w["remaining_percentage"] = json!(r);
    }
    if !usage.is_null() {
        w["current_usage"] = usage;
    }
    json!({"model": {"display_name": model}, "workspace": {"current_dir": "/t/myproject"}, "context_window": w})
}

fn opts(position: &str, compact: bool, tokens: bool) -> Options {
    Options {
        position: position.into(),
        compact,
        tokens,
        last_command: false,
        show_git: false,
    }
}

fn line_of(status: Value, o: &Options) -> String {
    let f = Facts {
        host: "claude-code".into(),
        status: Some(status),
        ..Facts::default()
    };
    plain(&statusline(&f, o).statusline.expect("a line"))
}

/// The meter at `remaining` percent on a 1M-token window.
fn used_at(remaining: f64) -> i64 {
    context_meter(&json!({"context_window_size": 1_000_000, "remaining_percentage": remaining}))
        .expect("a meter")
        .used
}

mod graph;
mod line;
mod state;

fn all_ported() -> Vec<String> {
    let mut ported = Vec::new();
    let mut port = |t: &str| ported.push(t.to_string());
    state::state_ported(&mut port);
    state::word_ported(&mut port);
    line::line_ported(&mut port);
    ported
}

const GIT: &str = "git: no STATE.md stamp: the state is the graph";
const GIT_REAL: &str = "git: the kernel runs git, not the hook; tests/cli/hooks_statusline.rs runs the segment against a real repository";
const GIT_PROPERTIES: &str = "env: fast-check properties over generated porcelain output; the cases of parseGitStatus and buildGitSegment are the examples they generalise";
const ENV: &str = "env: the auto-compact window overrides read the environment and Claude Code's settings files; the meter uses the model window";
const FS: &str = "fs: reads STATE.md, a transcript, a todo file or the update cache; the graph replaces STATE.md";
const BANNER: &str = "diverges: [update-banner] the update banner is dropped (D-r44-13)";
const SETTING: &str = "runtime: the kernel validates a setting against its declared domain";

const SKIPPED: &[(&str, &str)] = &[
    (
        "parseStateMd reads the frontmatter block the one fence owner finds > ",
        FS,
    ),
    ("parseStateMd > ", FS),
    ("parseStateMd #2833 lifecycle fields > ", FS),
    ("isInstalledAheadOfLatest > ", BANNER),
    ("readGsdState > ", FS),
    (
        "context meter: 100% is the auto-compact threshold (#2219, #4985) > CLAUDE_CODE_AUTO_COMPACT_WINDOW",
        ENV,
    ),
    (
        "context meter: 100% is the auto-compact threshold (#2219, #4985) > ACW exceeds",
        ENV,
    ),
    (
        "context meter: 100% is the auto-compact threshold (#2219, #4985) > autoCompactEnabled",
        ENV,
    ),
    (
        "context meter: 100% is the auto-compact threshold (#2219, #4985) > isAutoCompactDisabled",
        ENV,
    ),
    (
        "context meter resolves the auto-compact window the way Claude Code does (#4985) > ",
        ENV,
    ),
    (
        "auto-compact window and context meter: properties (#4985) > parseAutoCompactWindowEnv",
        ENV,
    ),
    (
        "auto-compact window and context meter: properties (#4985) > canonicalModelKey",
        ENV,
    ),
    (
        "auto-compact window and context meter: properties (#4985) > the threshold",
        ENV,
    ),
    (
        "auto-compact window and context meter: properties (#4985) > the settings merge",
        ENV,
    ),
    (
        "context meter boundary: acw at/near totalCtx does not pin used at 100% (#1194) > acw = totalCtx",
        ENV,
    ),
    (
        "todo-resolution: resolves in_progress task from the newest matching todos file (#305) > ",
        FS,
    ),
    ("default (flag absent) output has no \"last:\" suffix", FS),
    (
        "flag=true",
        "fs: reads the session transcript for the last slash command; the marker (T4) holds it in the session state and tests/cli/hooks_statusline.rs shows it",
    ),
    ("readLastSlashCommand returns null", FS),
    ("config-set", SETTING),
    (
        "config schema: statusline.state_format > config-set",
        SETTING,
    ),
    (
        "state_format via renderStatusline > ",
        "env: runs the hook on a project directory with a STATE.md; state_of's tests below and tests/cli/hooks_statusline.rs run the formats on the graph",
    ),
    // v1.19.0 adds, under this prefix, `read-only git spawns opt out of the optional index lock (#5048)`:
    // 'readGitStatus passes GIT_OPTIONAL_LOCKS=0', 'the env inherits the parent environment, not a bare replacement'
    // and 'the command and its bounds are unchanged'. The kernel's GitCli runs that `git status` and sets
    // GIT_OPTIONAL_LOCKS=0 on every call (crates/adapters/git/src/lib.rs `call`), pinned by
    // crates/adapters/git/tests/history.rs `every_git_call_opts_out_of_the_optional_index_lock`.
    (
        "readGitStatus + parseGitStatus against a real repo > ",
        GIT_REAL,
    ),
    ("package-identity exports > ", BANNER),
    ("worker result shape contract > ", BANNER),
    ("buildBannerOutput lineage guard > ", BANNER),
    ("evaluateUpdateCache lineage guard > returns", BANNER),
    (
        "evaluateUpdateCache lineage guard > gsd-statusline.js: #3582 cold tree",
        "runtime: the compiled runtime library of the node hook",
    ),
    (
        "evaluateUpdateCache lineage guard > gsd-statusline.js: #2734 STATE.md freshness marker > ",
        GIT,
    ),
];

#[test]
fn the_statusline_accounts_for_every_upstream_case() {
    assert_eq!(all_ported().len(), 103);
    accounting::account(SUITE, &all_ported(), SKIPPED);
}

const STATE_SKIPPED: &[(&str, &str)] = &[("state-head freshness properties (#2734) > ", GIT)];

#[test]
fn the_git_properties_are_skipped_whole() {
    accounting::account(
        GIT_SUITE,
        &[],
        &[
            (
                "gsd-statusline git segment: parseGitStatus properties > ",
                GIT_PROPERTIES,
            ),
            (
                "gsd-statusline git segment: buildGitSegment properties > ",
                GIT_PROPERTIES,
            ),
        ],
    );
}
