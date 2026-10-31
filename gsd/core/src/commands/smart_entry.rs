//! Ported from gsd-core v1.18.0 src/smart-entry.cts and src/runtime-slash.cts, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `smart_entry` (D-r43-12): `/gsd-next`'s router. It ports `classify`, `isComplete`, `isIdleStranded`, `actionsFor`,
//! `phaseSummary`, `buildSummary` and `progressLine` of upstream's `smart-entry`, and `formatGsdSlash` of
//! `runtime-slash`, over signals read from the graph a command receives (ADR 0299) in place of STATE.md and git.
//!
//! A command runs no git, so `git_dirty`, `git_unpushed` and `stale_activity` are false and `state_commits_behind` is
//! null: **idle-stranded is never derived**. A phase's computed `status` is STATE's word (`planning`, `executing`,
//! `verifying`, `milestone complete`), so the regexes of `classify` read what they read upstream.
//!
//! gsd-core modules this file ports: smart-entry.cjs.

use std::cmp::Ordering;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::reports;
use super::support::{answer, run};

/// `SITUATIONS`: every value `classify` can return.
pub const SITUATIONS: [&str; 11] = [
    "no-project",
    "paused",
    "blocked",
    "verify-failed",
    "needs-first-phase",
    "planning",
    "executing",
    "verify-pending",
    "idle-stranded",
    "complete",
    "unknown",
];

/// `SmartEntrySignals`.
#[derive(Clone, Debug, Default)]
pub struct Signals {
    pub current_phase: Option<String>,
    pub total_phases: Option<i64>,
    pub status: String,
    pub progress: Option<i64>,
    pub has_planning: bool,
    pub has_roadmap: bool,
    pub git_dirty: bool,
    pub git_unpushed: bool,
    pub paused: bool,
    pub blockers: Vec<String>,
    pub has_git: bool,
    pub verify_failed: bool,
    pub stale_activity: bool,
    pub roadmap_total_phases: Option<i64>,
    pub roadmap_completed_phases: Option<i64>,
    pub milestone_phases: Vec<String>,
    pub state_commits_behind: Option<i64>,
    pub state_commit_stale: Option<bool>,
}

/// `SmartEntryAction`.
#[derive(Clone, Debug)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub command: String,
    pub recommended: bool,
}

impl Signals {
    /// `SmartEntrySignals` as upstream's JSON prints it.
    pub fn to_json(&self) -> Value {
        json!({
            "current_phase": self.current_phase, "total_phases": self.total_phases, "status": self.status,
            "progress": self.progress, "has_planning": self.has_planning, "has_roadmap": self.has_roadmap,
            "git_dirty": self.git_dirty, "git_unpushed": self.git_unpushed, "paused": self.paused,
            "blockers": self.blockers, "has_git": self.has_git, "verify_failed": self.verify_failed,
            "stale_activity": self.stale_activity, "roadmap_total_phases": self.roadmap_total_phases,
            "roadmap_completed_phases": self.roadmap_completed_phases, "milestone_phases": self.milestone_phases,
            "state_commits_behind": self.state_commits_behind, "state_commit_stale": self.state_commit_stale,
        })
    }
}

impl Action {
    fn to_json(&self) -> Value {
        json!({ "id": self.id, "label": self.label, "command": self.command, "recommended": self.recommended })
    }
}

/// `comparePhaseNum` for the numbers a phase has here (`02`, `05.1`, `6A`): dotted parts, numeric where both are.
pub fn compare_phase(a: &str, b: &str) -> Ordering {
    let part = |s: &str| -> (u64, String) {
        let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
        (digits.parse().unwrap_or(0), s[digits.len()..].to_string())
    };
    let (mut left, mut right) = (a.split('.'), b.split('.'));
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => match part(x).cmp(&part(y)) {
                Ordering::Equal => {}
                other => return other,
            },
        }
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Where `word` occurs in lower-case `text`, with a word boundary before it (`start`) and/or after it (`end`).
fn occurs(text: &str, word: &str, start: bool, end: bool) -> bool {
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        (!start || before.is_none_or(|c| !is_word(c)))
            && (!end || after.is_none_or(|c| !is_word(c)))
    })
}

/// `in.progress`: the two words with any one character between.
fn in_dot_progress(text: &str, first: &str, last: &str) -> bool {
    text.match_indices(first).any(|(at, _)| {
        let mut rest = text[at + first.len()..].chars();
        rest.next().is_some() && rest.as_str().starts_with(last)
    })
}

/// `isComplete`: `/\b(milestone\s+complete|all\s+phases\s+complete|complete(d)?)\b/i` is a whole word `complete(d)`.
fn status_is_complete(status: &str) -> bool {
    let status = status.to_lowercase();
    occurs(&status, "complete", true, true) || occurs(&status, "completed", true, true)
}

fn is_complete(s: &Signals) -> bool {
    if let (Some(total), Some(done)) = (s.roadmap_total_phases, s.roadmap_completed_phases) {
        if total == 0 || done < total {
            return false;
        }
    } else {
        let Some(current) = &s.current_phase else {
            return false;
        };
        if let Some(last) = s.milestone_phases.last() {
            if compare_phase(current, last) == Ordering::Less {
                return false;
            }
        } else {
            let Some(total) = s.total_phases else {
                return false;
            };
            if compare_phase(current, &total.to_string()) == Ordering::Less {
                return false;
            }
        }
    }
    status_is_complete(&s.status)
}

/// `isIdleStranded`: `/\bcomplete(d)?|done|shipped|paused\b/i`, a clean tree, then unpushed or stale.
fn is_idle_stranded(s: &Signals) -> bool {
    let status = s.status.to_lowercase();
    if occurs(&status, "complete", true, false)
        || status.contains("done")
        || status.contains("shipped")
        || occurs(&status, "paused", false, true)
    {
        return false;
    }
    if s.git_dirty {
        return false;
    }
    s.git_unpushed || s.stale_activity
}

/// `classify`: the first matching predicate wins.
pub fn classify(s: &Signals) -> &'static str {
    let status = s.status.to_lowercase();
    if !s.has_planning {
        "no-project"
    } else if s.paused {
        "paused"
    } else if !s.blockers.is_empty() {
        "blocked"
    } else if s.verify_failed {
        "verify-failed"
    } else if s.total_phases.is_none_or(|t| t <= 0) || !s.has_roadmap {
        "needs-first-phase"
    } else if is_complete(s) {
        "complete"
    } else if occurs(&status, "planning", true, false) || occurs(&status, "planned", false, true) {
        "planning"
    } else if occurs(&status, "execute", true, false)
        || occurs(&status, "executing", true, false)
        || status.contains("active")
        || in_dot_progress(&status, "in", "progress")
        || occurs(&status, "building", false, true)
    {
        "executing"
    } else if occurs(&status, "verif", true, false) || status.contains("review") {
        // `review` also covers upstream's `needs.review` and `pending.review`, which contain it.
        "verify-pending"
    } else if is_idle_stranded(s) {
        "idle-stranded"
    } else {
        "unknown"
    }
}

/// `formatGsdSlash`: `/gsd-<name>`, or `$gsd-<name>` (lower-case) on codex; the arguments after the first space stay.
pub fn format_gsd_slash(name: &str, runtime: &str) -> String {
    let lower = name.to_lowercase();
    let bare = ["/gsd-", "/gsd:", "$gsd-", "$gsd:", "gsd-", "gsd:"]
        .iter()
        .find(|p| lower.starts_with(**p))
        .map_or(name, |p| &name[p.len()..]);
    if bare.trim().is_empty() {
        return String::new();
    }
    let (token, tail) = bare.split_at(bare.find(char::is_whitespace).unwrap_or(bare.len()));
    if runtime.to_lowercase() == "codex" {
        format!("$gsd-{}{tail}", token.to_lowercase())
    } else {
        format!("/gsd-{token}{tail}")
    }
}

fn act(id: &str, label: &str, command: &str, recommended: bool) -> Action {
    Action {
        id: id.into(),
        label: label.into(),
        command: command.into(),
        recommended,
    }
}

/// `actionsFor`: the ordered actions of a situation, the recommended one first.
pub fn actions_for(situation: &str, s: &Signals, runtime: &str) -> Vec<Action> {
    let cmd = |name: &str| format_gsd_slash(name, runtime);
    let a = |id: &str, label: &str, rec: bool| act(id, label, &cmd(id), rec);
    let next = |label: &str| act("progress-next", label, &cmd("progress --next"), true);
    let phase_n = s.current_phase.clone().unwrap_or_default();
    let planned = if phase_n.is_empty() { "1" } else { &phase_n };
    let exec_label = if phase_n.is_empty() {
        "Continue executing".to_string()
    } else {
        format!("Continue executing phase {phase_n}")
    };
    match situation {
        "no-project" => vec![
            a("new-project", "Start a new project", true),
            a("map-codebase", "Map an existing codebase", false),
            a("quick", "Quick task", false),
            a("help", "Show help", false),
        ],
        "paused" => vec![
            a("resume-work", "Resume work", true),
            a("progress", "Show progress", false),
            a("quick", "Quick task", false),
            a("help", "Show help", false),
        ],
        "blocked" => vec![
            a("debug", "Debug the blocker", true),
            a("verify-work", "Verify current work", false),
            a("capture", "Capture a note", false),
            a("progress", "Show progress", false),
        ],
        "verify-failed" => vec![
            a("verify-work", "Re-verify work", true),
            a("debug", "Debug the failure", false),
            a("code-review", "Review recent work", false),
            a("progress", "Show progress", false),
        ],
        "needs-first-phase" => vec![
            a("discuss-phase", "Discuss the first phase", true),
            a("plan-phase", "Plan phase 1", false),
            a("quick", "Quick task", false),
            a("progress", "Show progress", false),
        ],
        "planning" => vec![
            next(&format!("Advance to the next step (plan phase {planned})")),
            a("plan-phase", &format!("Plan phase {planned}"), false),
            a("discuss-phase", "Discuss before planning", false),
            a("progress", "Show progress", false),
        ],
        "executing" => vec![
            next("Advance to the next step"),
            a("execute-phase", &exec_label, false),
            a("quick", "Quick task", false),
            a("code-review", "Review recent work", false),
        ],
        "verify-pending" => vec![
            next("Advance to the next step (verify)"),
            a("verify-work", "Verify work", false),
            a("code-review", "Review recent work", false),
            a("ship", "Ship completed work", false),
        ],
        "idle-stranded" => vec![
            a("ship", "Ship committed work", true),
            a("complete-milestone", "Complete the milestone", false),
            a("progress", "Show progress", false),
            a("capture", "Capture a note", false),
        ],
        "complete" => vec![
            a("new-milestone", "Start a new milestone", true),
            a("extract-learnings", "Extract learnings", false),
            a("quick", "Quick task", false),
            a("progress", "Show progress", false),
        ],
        _ => vec![
            a("progress", "Show progress", true),
            act(
                "progress-next",
                "Advance to the next step",
                &cmd("progress --next"),
                false,
            ),
            a("quick", "Quick task", false),
            a("help", "Show help", false),
        ],
    }
}

/// `phaseSummary`.
fn phase_summary(s: &Signals) -> String {
    let phase = s.current_phase.as_deref();
    let at = phase.and_then(|p| {
        s.milestone_phases
            .iter()
            .position(|id| compare_phase(id, p) == Ordering::Equal)
    });
    if let Some(at) = at {
        return format!("Phase {} of {}", at + 1, s.milestone_phases.len());
    }
    let shown = |n: Option<i64>| n.map_or("?".to_string(), |n| n.to_string());
    let legacy = format!(
        "Phase {} of {}",
        phase.unwrap_or("?"),
        shown(s.total_phases)
    );
    let (Some(phase), Some(total), Some(roadmap)) = (phase, s.total_phases, s.roadmap_total_phases)
    else {
        return legacy;
    };
    if roadmap <= 0
        || (total == roadmap && compare_phase(phase, &total.to_string()) != Ordering::Greater)
    {
        return legacy;
    }
    match s.roadmap_completed_phases {
        None => format!("Phase {phase} ({roadmap} roadmap phases)"),
        Some(done) => format!("Phase {phase} ({done} of {roadmap} roadmap phases complete)"),
    }
}

fn progress_line(tail: &str, s: &Signals) -> String {
    let mut parts: Vec<String> = Vec::new();
    if s.current_phase.is_some() && s.total_phases.is_some() {
        parts.push(phase_summary(s));
    } else if let Some(phase) = &s.current_phase {
        parts.push(format!("Phase {phase}"));
    }
    if let Some(progress) = s.progress {
        parts.push(format!("{progress}%"));
    }
    parts.push(tail.to_string());
    parts.join(" · ")
}

/// `buildSummary`.
pub fn build_summary(situation: &str, s: &Signals) -> String {
    match situation {
        "no-project" => "No project yet — start fresh or map an existing codebase".into(),
        "paused" => "Work is paused — pick up where you left off".into(),
        "blocked" => {
            let count = if s.blockers.is_empty() {
                String::new()
            } else {
                format!(" · {} blocker(s)", s.blockers.len())
            };
            format!("Blocked{count} — resolve before continuing")
        }
        "verify-failed" => "Recent verification failed — re-verify or debug".into(),
        "needs-first-phase" => "Project initialized — plan your first phase".into(),
        "planning" => format!("{} — needs a plan", phase_summary(s)),
        "executing" => progress_line("executing", s),
        "verify-pending" => progress_line("ready to verify", s),
        "idle-stranded" => "Committed work not shipped — time to ship".into(),
        "complete" => "All phases complete — start a new milestone".into(),
        _ => "Unsure of state — showing progress".into(),
    }
}

/// One phase of the milestone, from the graph.
struct PhaseRow<'a> {
    id: &'a str,
    number: &'a str,
    status: &'a str,
}

/// The signals of the project in `graph` (D-r43-12): STATE's words from the computed phase `status`.
pub fn signals(graph: &CommandGraph) -> Result<Signals, String> {
    let mut s = Signals {
        has_planning: graph.nodes_of_kind("project").next().is_some(),
        ..Signals::default()
    };
    let milestone = reports::current(graph);
    s.has_roadmap = milestone.is_some();
    s.paused = graph.nodes_of_kind("pause").next().is_some();
    s.blockers = graph
        .nodes_of_kind("blocker")
        .filter(|b| !b.has_field("resolved"))
        .map(|b| b.text("text").unwrap_or(&b.id).to_string())
        .collect();
    let Some(milestone) = milestone else {
        return Ok(s);
    };
    let scope = format!("{}/", milestone.id);
    let mut rows = Vec::new();
    for p in graph
        .nodes_of_kind("phase")
        .filter(|p| p.id.starts_with(&scope))
    {
        let number = p.id.rsplit('/').next().unwrap_or_default();
        // The sentinel 999 (backlog) phases are left out, as `progress_report` does.
        if number.starts_with("999.") {
            continue;
        }
        let status = p.text("status").ok_or_else(|| {
            format!(
                "the graph carries no computed status for {}; run `specforge check`",
                p.id
            )
        })?;
        rows.push(PhaseRow {
            id: &p.id,
            number,
            status,
        });
    }
    rows.sort_by(|a, b| compare_phase(a.number, b.number));
    let done = rows.iter().filter(|p| p.status == "complete").count() as i64;
    let total = rows.len() as i64;
    s.total_phases = Some(total);
    s.roadmap_total_phases = Some(total);
    s.roadmap_completed_phases = Some(done);
    s.milestone_phases = rows.iter().map(|p| p.number.to_string()).collect();
    s.progress = reports::progress(graph)?["percent"].as_i64();
    let current = rows.iter().find(|p| p.status != "complete");
    s.current_phase = current.or(rows.last()).map(|p| p.number.to_string());
    s.status = match current.map(|p| p.status) {
        None if total > 0 => "milestone complete",
        Some("not_started") => "planning",
        Some("planned" | "in_progress") => "executing",
        Some("executed" | "needs_review") => "verifying",
        _ => "",
    }
    .to_string();
    if let Some(p) = current {
        s.verify_failed = graph
            .nodes_of_kind("verification")
            .any(|v| v.text("phase") == Some(p.id) && v.text("status") == Some("gaps_found"));
    }
    Ok(s)
}

fn smart_entry(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    // the host that asked: codex writes `$gsd-x`, the others `/gsd-x`
    let host = call.host().unwrap_or("claude-code");
    let runtime = host.strip_suffix("-code").unwrap_or(host);
    let s = signals(call.graph())?;
    let situation = classify(&s);
    let actions = actions_for(situation, &s, runtime);
    let recommended = actions
        .iter()
        .find(|a| a.recommended)
        .map_or("progress", |a| a.id.as_str());
    let summary = build_summary(situation, &s);
    let command = actions.iter().find(|a| a.id == recommended).map_or_else(
        || format_gsd_slash("progress", runtime),
        |a| a.command.clone(),
    );
    let human = format!("{summary}\nRecommended: {recommended} → {command}\n");
    Ok(answer(
        call,
        json!({
            "situation": situation, "recommended": recommended, "summary": summary, "signals": s.to_json(),
            "actions": actions.iter().map(Action::to_json).collect::<Vec<_>>(),
        }),
        human,
        None,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("smart_entry", |cmd| {
        cmd.description("Classify the project's situation and name the next actions, as gsd-tools smart-entry does (idle-stranded is never derived: a command runs no git)")
            .handler(|call| run(call, smart_entry));
    });
}

#[cfg(test)]
mod tests;
