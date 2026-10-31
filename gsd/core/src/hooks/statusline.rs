//! Ported from gsd-core v1.19.0 hooks/gsd-statusline.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `statusline` (D-r44-13). The line is the model, the GSD state, the directory and the context meter. The state
//! segment is read from the graph instead of STATE.md: the milestone and its computed progress, and the first phase
//! that is not complete, from the same computation `progress_report` answers. The meter's numbers are written to the
//! session's state for the context monitor.
//!
//! `statusline.show_git` adds the branch and the work-state markers after the directory, from the status the host reads
//! (`git status --porcelain=v2 --branch`, ADR 0320): `main+2~1?3↑1`, or `main✓` when clean. Known cost
//! (REVIEW-k20c-part2 m1): the hook declares the status read statically, so the host runs `git status` on every render
//! even while `show_git` is off, under the git adapter's deadline. Upstream spawns nothing then and drops the segment
//! at 1.5 s. A fact flag gated by a setting, and a shorter deadline, are the kernel's to add.
//!
//! Dropped: the environment and `autoCompactWindow` overrides of the auto-compact threshold (no environment),
//! state freshness (`show_state_freshness` stays declared and inert: there is no STATE.md stamp),
//! the current-task segment (a file under `~/.claude`) and the update banner (a lockfile), and the colours: the
//! kernel writes a hook's control characters out as text (ADR 0230, 0231), so the line is plain. One more
//! difference: the phase counts read `completed/total`, the milestone's computed counters, where upstream reads
//! `current/total`.
//!
//! Also ports gsd-core bin/lib modules: context-utilization.cjs.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::facts::Facts;
use crate::commands::reports;

mod format;

pub use format::StateSeg;
use format::{Line, Meter};

/// The status word of a phase label, in the vocabulary the state segment uses.
fn status_word(label: &str) -> &'static str {
    match label {
        "Pending" => "planning",
        "Planned" | "In Progress" => "executing",
        "Executed" | "Needs Review" => "verifying",
        "Complete" => "completed",
        _ => "",
    }
}

/// A count as the milestone's computed progress holds it.
fn count(progress: &Value, key: &str) -> Option<f64> {
    progress.get(key).and_then(Value::as_f64)
}

/// The state segment from the graph: the current milestone, its computed progress and its first phase that is not
/// complete. A graph without computed values (a view whose checks were skipped) gives an empty segment.
pub(super) fn state_of(graph: &CommandGraph) -> StateSeg {
    let mut seg = StateSeg::default();
    if let Some(m) = reports::current(graph) {
        seg.milestone = m.id.clone();
        seg.milestone_name = m.text("name").unwrap_or_default().to_string();
        if let Some(p) = m.fields.get("progress") {
            seg.percent = count(p, "percent");
            seg.completed_phases = count(p, "phases_complete");
            seg.total_phases = count(p, "phases");
            seg.phase_num = seg
                .completed_phases
                .map(|n| n.to_string())
                .unwrap_or_default();
            seg.phase_total = seg.total_phases.map(|n| n.to_string()).unwrap_or_default();
        }
    }
    if let Ok(report) = reports::progress(graph)
        && let Some(open) = report["phases"]
            .as_array()
            .and_then(|ps| ps.iter().find(|p| p["status"] != "Complete"))
    {
        seg.phase_name = open["name"].as_str().unwrap_or_default().to_string();
        seg.status = status_word(open["status"].as_str().unwrap_or_default()).into();
    }
    if seg.milestone.is_empty() {
        seg.phase_num.clear();
        seg.phase_total.clear();
    }
    seg
}

/// The settings the line reads.
pub(super) struct Options {
    pub position: String,
    pub compact: bool,
    pub tokens: bool,
    pub last_command: bool,
    pub show_git: bool,
}

impl Options {
    fn current() -> Options {
        let s = Settings::current();
        Options {
            position: s
                .string("statusline.context_position")
                .unwrap_or("end")
                .into(),
            compact: s.string("statusline.state_format") == Some("compact"),
            tokens: s.bool("statusline.show_context_tokens") == Some(true),
            last_command: s.bool("statusline.show_last_command") == Some(true),
            show_git: s.bool("statusline.show_git") == Some(true),
        }
    }
}

/// The line, and the session state the meter leaves for the context monitor.
pub(super) fn statusline(f: &Facts, o: &Options) -> HookOutput {
    let status = f.status.clone().unwrap_or_default();
    let model = format::compact_model(
        status["model"]["display_name"]
            .as_str()
            .filter(|n| !n.is_empty())
            .unwrap_or("Claude"),
    );
    let dir = status["workspace"]["current_dir"]
        .as_str()
        .unwrap_or(&f.cwd);
    let meter: Option<Meter> = format::context_meter(&status["context_window"]);
    let ctx = meter
        .as_ref()
        .map(|m| {
            let suffix = if o.tokens {
                format::token_suffix(status["context_window"].get("current_usage"))
            } else {
                String::new()
            };
            format::render_meter(m, &suffix)
        })
        .unwrap_or_default();
    let workstream = f
        .project_state
        .get("active_workstream")
        .and_then(Value::as_str);
    let seg = f.graph.map(state_of).unwrap_or_default();
    let mut text = if o.compact {
        format::format_compact(&seg)
    } else {
        format::format_full(&seg)
    };
    if let Some(w) = workstream.filter(|w| !w.is_empty()) {
        text = if text.is_empty() {
            format!("ws {w}")
        } else {
            format!("{text} · ws {w}")
        };
    }
    let middle = (!text.is_empty()).then_some(text);
    let last = f
        .session_state
        .as_ref()
        .and_then(|s| s.get("workflow")?.get("command")?.as_str())
        .filter(|_| o.last_command)
        .map(|c| format!(" │ last: {c}"))
        .unwrap_or_default();
    let git = if o.show_git {
        format::git_segment(
            f.git
                .as_ref()
                .and_then(|g| g.status.as_deref())
                .and_then(format::parse_git_status)
                .as_ref(),
        )
    } else {
        String::new()
    };
    let line = format::compose(&Line {
        model: &model,
        ctx: &ctx,
        middle: middle.as_deref(),
        dirname: Facts::base(dir),
        last_command: &last,
        git: &git,
        position: &o.position,
        ..Line::default()
    });
    let mut out = HookOutput {
        statusline: Some(line),
        ..HookOutput::default()
    };
    if let (Some(m), Some(_)) = (&meter, &f.session) {
        let mut ctx = json!({"remaining_percentage": m.remaining, "used_pct": m.used, "at": f.now});
        if let (Some(u), Some(l)) = (m.used_tokens, m.limit_tokens) {
            ctx["used_tokens"] = json!(u);
            ctx["threshold_tokens"] = json!(l);
        }
        out.state = Some(StatePatch {
            session: Some(json!({ "context": ctx })),
            project: None,
        });
    }
    out
}

pub(super) fn declare(c: &mut ContributionsBuilder) {
    c.hook("statusline", HookEvent::Statusline, |h| {
        h.description("Renders the status line from the graph and the session")
            .reads_graph()
            .reads_state()
            .reads_git_status()
            .git_when("statusline.show_git")
            .handler(|call| Ok(statusline(&Facts::of(call), &Options::current())));
    });
}

#[cfg(test)]
mod tests;
