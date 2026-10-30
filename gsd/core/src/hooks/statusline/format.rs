//! Ported from gsd-core v1.18.0 hooks/gsd-statusline.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The statusline's pure parts: the GSD state segment in its two formats, the status word, the progress bar, the
//! token count, the model badge, the context meter and the layout. What feeds them is `statusline.rs`.

use serde_json::Value;

/// The GSD state segment's inputs, as upstream's `readGsdState` builds them from STATE.md. An empty string is
/// absent, as it is in JavaScript's `if (s.status)`.
#[derive(Debug, Clone, Default)]
pub struct StateSeg {
    pub milestone: String,
    pub milestone_name: String,
    pub percent: Option<f64>,
    pub completed_phases: Option<f64>,
    pub total_phases: Option<f64>,
    pub active_phase: String,
    pub next_action: String,
    pub next_phases: Vec<String>,
    pub status: String,
    pub phase_num: String,
    pub phase_total: String,
    pub phase_name: String,
    pub no_active_workstream: bool,
}

const NO_ACTIVE_WORKSTREAM: &str = "no active workstream";

/// JavaScript's `Math.round`.
fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `[█████░░░░░] 50%`, or nothing for no percent. `parseInt` truncates and the bar clamps to 0..=100.
pub fn progress_bar(percent: Option<f64>) -> String {
    let Some(p) = percent.filter(|p| !p.is_nan()) else {
        return String::new();
    };
    let pct = p.trunc().clamp(0.0, 100.0) as usize;
    let filled = pct / 10;
    format!("[{}{}] {pct}%", "█".repeat(filled), "░".repeat(10 - filled))
}

fn is_done(s: &StateSeg) -> bool {
    s.percent == Some(100.0)
        || (s.total_phases.is_some_and(|t| t > 0.0) && s.completed_phases == s.total_phases)
}

/// The state segment of `statusline.state_format: "full"`.
pub fn format_full(s: &StateSeg) -> String {
    if s.no_active_workstream {
        return NO_ACTIVE_WORKSTREAM.into();
    }
    let mut parts: Vec<String> = Vec::new();
    if !s.milestone.is_empty() || !s.milestone_name.is_empty() {
        let name = if s.milestone_name == "milestone" {
            ""
        } else {
            &s.milestone_name
        };
        let bar = progress_bar(s.percent);
        let pieces: Vec<&str> = [s.milestone.as_str(), name, bar.as_str()]
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect();
        if !pieces.is_empty() {
            parts.push(pieces.join(" "));
        }
    }
    if !s.active_phase.is_empty() {
        let phase = format!("Phase {}", s.active_phase);
        parts.push(if s.status.is_empty() {
            phase
        } else {
            format!("{phase} {}", s.status)
        });
    } else if !s.next_action.is_empty() && !s.next_phases.is_empty() {
        parts.push(format!(
            "next {} {}",
            s.next_action,
            s.next_phases.join("/")
        ));
    } else if is_done(s) {
        parts.push("milestone complete".into());
    } else {
        if !s.status.is_empty() {
            parts.push(s.status.clone());
        }
        if !s.phase_num.is_empty() && !s.phase_total.is_empty() {
            parts.push(if s.phase_name.is_empty() {
                format!("ph {}/{}", s.phase_num, s.phase_total)
            } else {
                format!("{} ({}/{})", s.phase_name, s.phase_num, s.phase_total)
            });
        }
    }
    parts.join(" · ")
}

/// `normalizeStateStatus`: a value of the declared vocabulary becomes its token, anything else is returned as is.
fn normalize_status(status: &str) -> String {
    let key = status
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let exact = match key.as_str() {
        "paused" | "stopped" => "paused",
        "executing" | "in progress" | "ready to execute" => "executing",
        "planning" | "ready to plan" | "planning complete" => "planning",
        "discussing" => "discussing",
        "verifying" | "phase complete — ready for verification" => "verifying",
        "completed"
        | "done"
        | "complete"
        | "phase complete"
        | "all phases complete"
        | "milestone complete" => "completed",
        "unknown" => "unknown",
        _ => "",
    };
    if !exact.is_empty() {
        return exact.into();
    }
    let words: Vec<&str> = key.split(' ').collect();
    let one = |n: usize| words.len() == n && words.iter().all(|w| !w.is_empty());
    let token = match words.as_slice() {
        ["executing", "phase", _] if one(3) => "executing",
        ["planning", "phase", _] if one(3) => "planning",
        ["verifying", "phase", _] if one(3) => "verifying",
        ["phase", _, "complete"] if one(3) => "completed",
        [_, "milestone", "complete"] if one(3) => "completed",
        _ if is_complete_glyph(&key) => "completed",
        _ => return status.into(),
    };
    token.into()
}

/// `/^complete\s*[✓✔✅☑]?$/` over a key whose whitespace is already single spaces.
fn is_complete_glyph(key: &str) -> bool {
    key.strip_prefix("complete").is_some_and(|rest| {
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        rest.is_empty() || matches!(rest, "✓" | "✔" | "✅" | "☑")
    })
}

/// One keyword for a status: the vocabulary's token (`paused` shouted), else the first word, at most 16 characters.
pub fn short_status(status: &str) -> Option<String> {
    if status.is_empty() {
        return None;
    }
    let norm = normalize_status(status);
    match norm.as_str() {
        "paused" => return Some("PAUSED".into()),
        "discussing" | "planning" | "executing" | "verifying" | "completed" => return Some(norm),
        _ => {}
    }
    let first = norm
        .trim()
        .split(|c: char| c.is_whitespace() || matches!(c, '—' | '–' | '-'))
        .next()
        .unwrap_or_default();
    (!first.is_empty()).then(|| first.chars().take(16).collect())
}

/// The state segment of `statusline.state_format: "compact"`.
pub fn format_compact(s: &StateSeg) -> String {
    if s.no_active_workstream {
        return NO_ACTIVE_WORKSTREAM.into();
    }
    let mut parts: Vec<String> = Vec::new();
    if !s.milestone.is_empty() {
        parts.push(s.milestone.clone());
    }
    let phase = if s.active_phase.is_empty() {
        &s.phase_num
    } else {
        &s.active_phase
    };
    if !phase.is_empty() {
        parts.push(if s.phase_total.is_empty() {
            format!("P{phase}")
        } else {
            format!("P{phase}/{}", s.phase_total)
        });
    }
    if s.active_phase.is_empty() && is_done(s) {
        parts.push("complete".into());
    } else if let Some(st) = short_status(&s.status) {
        parts.push(st);
    } else if phase.is_empty() && !s.next_action.is_empty() {
        let phases = s.next_phases.join("/");
        parts.push(if phases.is_empty() {
            format!("next {}", s.next_action)
        } else {
            format!("next {} {phases}", s.next_action)
        });
    }
    parts.join(" · ")
}

/// A token count: `156342` as `156k`, `1234567` as `1.2M`.
pub fn format_tokens(tokens: f64) -> String {
    if tokens >= 1_000_000.0 || round(tokens / 1000.0) >= 1000.0 {
        format!("{:.1}M", tokens / 1_000_000.0)
    } else if tokens >= 1000.0 {
        format!("{}k", round(tokens / 1000.0))
    } else {
        format!("{tokens}")
    }
}

/// `Number(v) || 0`.
fn number(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0.0),
        Some(Value::Bool(true)) => 1.0,
        _ => 0.0,
    }
}

/// ` (156k)` for the last turn's usage, or nothing.
pub fn token_suffix(usage: Option<&Value>) -> String {
    let Some(u) = usage.filter(|u| u.is_object()) else {
        return String::new();
    };
    let total: f64 = [
        "input_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
        "output_tokens",
    ]
    .iter()
    .map(|k| number(u.get(*k)))
    .sum();
    if total > 0.0 {
        format!(" ({})", format_tokens(total))
    } else {
        String::new()
    }
}

/// `Sonnet 4.5 (1M context)` as `Sonnet 4.5 (1M)`: a trailing parenthesis ending in `context` or `ctx`.
pub fn compact_model(name: &str) -> String {
    let Some(open) = name.rfind('(').filter(|_| name.ends_with(')')) else {
        return name.into();
    };
    let inner = &name[open + 1..name.len() - 1];
    if inner.contains(')') {
        return name.into();
    }
    let lower = inner.to_lowercase();
    let cut = ["context", "ctx"]
        .iter()
        .find_map(|w| lower.strip_suffix(w).map(str::len));
    let Some(cut) = cut else { return name.into() };
    let token = inner[..cut].trim_end();
    if token.is_empty() || !inner[..cut].ends_with(char::is_whitespace) {
        return name.into();
    }
    format!("{} ({token})", name[..open].trim_end())
}

const OUTPUT_RESERVE: f64 = 20_000.0;
const SUMMARY_BUFFER: f64 = 13_000.0;

/// The meter: `used` + `remaining` = 100, where 100% is the auto-compact threshold.
#[derive(Debug, Clone, PartialEq)]
pub struct Meter {
    pub used: i64,
    pub remaining: i64,
    pub used_tokens: Option<f64>,
    pub limit_tokens: Option<f64>,
}

/// Upstream's `contextMeter` without the environment and settings overrides (D-r44-13).
pub fn context_meter(window: &Value) -> Option<Meter> {
    if !window.is_object() {
        return None;
    }
    let size = number(window.get("context_window_size"));
    let usage = window.get("current_usage").filter(|u| u.is_object());
    let remaining = window.get("remaining_percentage").filter(|r| !r.is_null());
    let clamp = |x: f64| round(x).clamp(0.0, 100.0) as i64;
    if size > 0.0 {
        let used_tokens = match (usage, remaining) {
            (Some(u), _) => Some(
                number(u.get("input_tokens"))
                    + number(u.get("cache_creation_input_tokens"))
                    + number(u.get("cache_read_input_tokens")),
            ),
            (None, Some(r)) => Some(round((100.0 - number(Some(r))) * size / 100.0)),
            _ => None,
        };
        if let Some(tokens) = used_tokens {
            let threshold = size - OUTPUT_RESERVE - SUMMARY_BUFFER;
            let limit = if threshold > 0.0 { threshold } else { size };
            let used = clamp(tokens / limit * 100.0);
            return Some(Meter {
                used,
                remaining: 100 - used,
                used_tokens: Some(tokens),
                limit_tokens: Some(limit),
            });
        }
    }
    let r = remaining?;
    let used = clamp(100.0 - number(Some(r)));
    Some(Meter {
        used,
        remaining: 100 - used,
        used_tokens: None,
        limit_tokens: None,
    })
}

/// The bar and percentage, with a skull from 80%. Upstream colours the bar by band; the kernel writes a hook's control
/// characters out as text (ADR 0230, 0231), so there is no colour to give.
pub fn render_meter(m: &Meter, suffix: &str) -> String {
    let filled = (m.used / 10) as usize;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
    let used = m.used;
    if used >= 80 {
        format!(" 💀 {bar} {used}%{suffix}")
    } else {
        format!(" {bar} {used}%{suffix}")
    }
}

/// The segments of a line.
#[derive(Debug, Clone, Default)]
pub struct Line<'a> {
    pub update: &'a str,
    pub model: &'a str,
    pub ctx: &'a str,
    pub middle: Option<&'a str>,
    pub dirname: &'a str,
    pub last_command: &'a str,
    pub git: &'a str,
    /// `front` puts the meter after the model; anything else after the directory.
    pub position: &'a str,
}

/// Upstream's `composeStatusline`.
pub fn compose(l: &Line) -> String {
    let (model, dir) = (l.model, l.dirname);
    let (update, ctx, last, git) = (l.update, l.ctx, l.last_command, l.git);
    match (l.position == "front", l.middle.filter(|m| !m.is_empty())) {
        (true, Some(m)) => format!("{update}{model}{ctx} │ {m} │ {dir}{git}{last}"),
        (true, None) => format!("{update}{model}{ctx} │ {dir}{git}{last}"),
        (false, Some(m)) => format!("{update}{model} │ {m} │ {dir}{git}{ctx}{last}"),
        (false, None) => format!("{update}{model} │ {dir}{git}{ctx}{last}"),
    }
}
