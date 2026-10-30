//! Ported from gsd-core v1.18.0 hooks/gsd-validate-commit.sh, hooks/gsd-phase-boundary.sh and hooks/gsd-session-state.sh, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The three community hooks (D-r44-14, D-r44-15), each off until `hooks.community` is on.
//!
//! `validate_commit` refuses a commit whose subject is not a Conventional Commit of a known type or is longer than 72
//! characters. The host reads the message (`-m`; the heredoc form arrives as its `$(cat <<'EOF'` text) and hands it over; a commit that takes its
//! message from elsewhere (`-F`, `-C`, `--fixup`, the editor) has none and is let through, as upstream lets an
//! unreadable message through. A `fixup!` or `squash!` subject is let through too. The subject rule is upstream's
//! `^([a-z][a-z0-9-]*)(\(.+\))?:[[:space:]].+`, matched by hand: the crate has no regex dependency.
//!
//! `phase_boundary` reminds, after a write of a spec file, that a phase's status and progress are computed from the
//! spec: upstream asks whether STATE.md should be updated, and there is no STATE.md to update.
//!
//! `session_state` starts a session with the project's progress line, its open blockers, an open pause, the active
//! workstream and the `mode` setting, where upstream prints STATE.md's first 20 lines and the config's mode. Without
//! a project it suggests `/gsd-new-project`, as upstream does without `.planning/`.

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use super::facts::Facts;
use super::statusline::state_of;
use crate::commands::reports;

/// The types every project accepts.
const BUILTIN_TYPES: [&str; 10] = [
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore",
];

/// The longest subject.
const SUBJECT_LIMIT: usize = 72;

/// A type name in the config: `^[a-z][a-z0-9-]*$`.
fn is_type_name(t: &str) -> bool {
    let mut chars = t.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `:[[:space:]].+` at the start of `s`.
fn colon_then_text(s: &str) -> bool {
    let Some(rest) = s.strip_prefix(':') else {
        return false;
    };
    let mut chars = rest.chars();
    chars.next().is_some_and(char::is_whitespace) && chars.next().is_some()
}

/// The type of `subject` when it is `type(scope): text` or `type: text`.
fn type_of(subject: &str) -> Option<&str> {
    let end = subject
        .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
        .unwrap_or(subject.len());
    let (kind, rest) = subject.split_at(end);
    if !kind.starts_with(|c: char| c.is_ascii_lowercase()) {
        return None;
    }
    let matched = colon_then_text(rest)
        || rest.strip_prefix('(').is_some_and(|inner| {
            // `\(.+\)` is greedy and may backtrack to any `)` after one character of scope
            inner
                .char_indices()
                .skip(1)
                .any(|(at, c)| c == ')' && colon_then_text(&inner[at + 1..]))
        });
    matched.then_some(kind)
}

/// The subject of a message: its first line, or the first line of the body when the message is the command-substituted
/// heredoc form `$(cat <<'EOF' … EOF)`, whose first line is the opener (upstream's #3802).
fn subject_of(message: &str) -> &str {
    let mut lines = message.lines();
    let first = lines.next().unwrap_or_default();
    let opener = first
        .strip_prefix("$(cat <<")
        .map(|o| o.trim_start_matches('-').trim());
    if opener.is_some_and(|o| !o.is_empty() && !o.contains(char::is_whitespace)) {
        return lines.find(|l| !l.trim().is_empty()).unwrap_or_default();
    }
    first
}

/// The verdict on a commit's message, given the types the config adds.
pub(super) fn validate_commit(f: &Facts, extra: &[String]) -> HookOutput {
    let Some(message) = f.commit_message.as_deref().filter(|m| !m.is_empty()) else {
        return HookOutput::allow();
    };
    let subject = subject_of(message);
    if subject.starts_with("fixup! ") || subject.starts_with("squash! ") {
        return HookOutput::allow();
    }
    let mut types: Vec<&str> = BUILTIN_TYPES.to_vec();
    for t in extra {
        if is_type_name(t) && !types.contains(&t.as_str()) {
            types.push(t);
        }
    }
    if !type_of(subject).is_some_and(|t| types.contains(&t)) {
        return HookOutput::deny(format!(
            "CONVENTIONAL_COMMITS_VIOLATION: Commit message must follow Conventional Commits: \
             <type>(<scope>): <subject>. Valid types: {}. \
             Subject must be <=72 chars, lowercase, imperative mood, no trailing period.",
            types.join(", ")
        ));
    }
    if subject.chars().count() > SUBJECT_LIMIT {
        return HookOutput::deny(
            "COMMIT_SUBJECT_TOO_LONG: Commit subject must be 72 characters or less.",
        );
    }
    HookOutput::allow()
}

/// After a write of a spec file: the status and progress of a phase are computed, not recorded.
pub(super) fn phase_boundary(f: &Facts) -> HookOutput {
    if !matches!(f.class.as_str(), "write" | "edit") {
        return HookOutput::default();
    }
    let Some(file) = f.paths.iter().find(|p| p.in_spec_root) else {
        return HookOutput::default();
    };
    HookOutput::allow().with_context(format!(
        "Spec file modified: {}\nA phase's status and progress are computed from the spec, \
         so there is no state file to update. Run `specforge gsd progress-report` to see where the phases stand.",
        file.path
    ))
}

/// The progress line of the project in `graph`.
fn progress_line(graph: &CommandGraph) -> Option<String> {
    let seg = state_of(graph);
    if seg.milestone.is_empty() {
        return None;
    }
    let mut line = format!("{} {}", seg.milestone, seg.milestone_name)
        .trim_end()
        .to_string();
    if let (Some(done), Some(total)) = (seg.completed_phases, seg.total_phases) {
        line.push_str(&format!(": {done}/{total} phases complete"));
    }
    if let Some(p) = seg.percent {
        line.push_str(&format!(" ({p}%)"));
    }
    if let Ok(report) = reports::progress(graph)
        && let Some(open) = report["phases"]
            .as_array()
            .and_then(|ps| ps.iter().find(|p| p["status"] != "Complete"))
    {
        line.push_str(&format!(
            "; phase {} {}: {} plan(s), {} summary(ies)",
            open["number"].as_str().unwrap_or_default(),
            open["name"].as_str().unwrap_or_default(),
            open["plans"],
            open["summaries"]
        ));
    }
    Some(line)
}

/// What a session starts with.
pub(super) fn session_state(f: &Facts, mode: &str) -> HookOutput {
    let mut lines = vec!["## Project State Reminder".to_string(), String::new()];
    match f
        .graph
        .filter(|g| g.nodes_of_kind("project").next().is_some())
    {
        None => lines
            .push("No project found - suggest /gsd-new-project if starting new work.".to_string()),
        Some(graph) => {
            lines.push(
                progress_line(graph)
                    .unwrap_or_else(|| "The project has no milestone yet.".to_string()),
            );
            let blockers: Vec<&str> = graph
                .nodes_of_kind("blocker")
                .filter(|b| !b.has_field("resolved"))
                .map(|b| b.text("text").unwrap_or(&b.id))
                .collect();
            if !blockers.is_empty() {
                lines.push(format!("Open blockers: {}", blockers.join("; ")));
            }
            if graph.nodes_of_kind("pause").next().is_some() {
                lines.push("Work is paused: /gsd-resume-work picks it up.".to_string());
            }
        }
    }
    if let Some(ws) = f
        .project_state
        .get("active_workstream")
        .and_then(Value::as_str)
        .filter(|w| !w.is_empty())
    {
        lines.push(format!("Active workstream: {ws}"));
    }
    lines.push(String::new());
    lines.push(format!("Config: \"mode\": \"{mode}\""));
    HookOutput::allow().with_context(lines.join("\n"))
}

pub(super) fn declare(c: &mut ContributionsBuilder) {
    c.hook("validate_commit", HookEvent::PreCommit, |h| {
        h.description("Refuses a commit whose subject is not a Conventional Commit")
            .tools(&["shell"])
            .when("hooks.community")
            .handler(|call| {
                let extra: Vec<String> = Settings::current()
                    .get("hooks.commit_types")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(validate_commit(&Facts::of(call), &extra))
            });
    });
    c.hook("phase_boundary", HookEvent::PostTool, |h| {
        h.description("Reminds, after a spec file is written, that a phase's progress is computed")
            .tools(&["write", "edit"])
            .when("hooks.community")
            .handler(|call| Ok(phase_boundary(&Facts::of(call))));
    });
    c.hook("session_state", HookEvent::SessionStart, |h| {
        h.description("Starts a session with the project's progress, blockers and workstream")
            .when("hooks.community")
            .reads_graph()
            .reads_state()
            .handler(|call| {
                let mode = Settings::current()
                    .string("mode")
                    .unwrap_or("interactive")
                    .to_string();
                Ok(session_state(&Facts::of(call), &mode))
            });
    });
}

#[cfg(test)]
mod tests;
