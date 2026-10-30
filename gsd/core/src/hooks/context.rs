//! Ported from gsd-core v1.18.0 hooks/gsd-context-monitor.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `context_monitor` and `context_compacted` (D-r44-12).
//!
//! Upstream's statusline writes the context metrics to a bridge file in the temp directory and the monitor reads
//! it back; here the statusline writes them to the session's state, `context` (`remaining_percentage`, `used_pct`,
//! `used_tokens`, `threshold_tokens`, `at`), and the monitor keeps its own in `monitor` (`calls`, `level`,
//! `compacted_at`). A compaction deletes the metrics and records when it happened; a reading taken within a minute
//! of it is the compaction's own render and is ignored, as upstream's watermark does.
//!
//! Dropped: the CRITICAL spawn of `state record-session --stopped-at` (a hook writing the stopped-at point would
//! be a `writes` plan of a pause, and `/gsd-pause-work` records it), the Stop and SubagentStop registrations, and
//! upstream's non-GSD-project wording (the hook runs only where `@gsd/core` is loaded).

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::facts::Facts;

const WARNING: f64 = 35.0;
const CRITICAL: f64 = 25.0;
const STALE_SECONDS: i64 = 60;
const DEBOUNCE_CALLS: i64 = 5;
const COMPACT_GRACE_SECONDS: i64 = 60;
const WATERMARK_SKEW_SECONDS: i64 = 5;

/// The warning and critical fire-points from the two settings, as upstream's `resolveThresholds` takes them: a value
/// is usable when it is a number in 0..=100, otherwise that key falls back alone; a pair whose critical is not
/// strictly below its warning reverts both.
pub(super) fn resolve_thresholds(warning: Option<&Value>, critical: Option<&Value>) -> (f64, f64) {
    let usable = |v: Option<&Value>, fallback: f64| {
        v.and_then(Value::as_f64)
            .filter(|n| n.is_finite() && (0.0..=100.0).contains(n))
            .unwrap_or(fallback)
    };
    let (w, c) = (usable(warning, WARNING), usable(critical, CRITICAL));
    if c < w { (w, c) } else { (WARNING, CRITICAL) }
}

/// Seconds since the epoch of `YYYY-MM-DDTHH:MM:SS[.fff]Z`.
fn epoch(s: &str) -> Option<i64> {
    let (date, time) = s.strip_suffix('Z')?.split_once('T')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let mut t = time.split(':');
    let (h, mi) = (
        t.next()?.parse::<i64>().ok()?,
        t.next()?.parse::<i64>().ok()?,
    );
    let sec = t.next()?.split('.').next()?.parse::<i64>().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146_097 + doe - 719_468) * 86_400 + h * 3600 + mi * 60 + sec)
}

/// A number as JavaScript prints it.
fn num(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn patch(session: Value) -> HookOutput {
    HookOutput {
        state: Some(StatePatch {
            session: Some(session),
            project: None,
        }),
        ..HookOutput::default()
    }
}

/// Warns, once in five tool uses, that the context is running out.
pub(super) fn context_monitor(f: &Facts, (warning, critical): (f64, f64)) -> HookOutput {
    let Some(state) = f.session_state.as_ref() else {
        return HookOutput::default();
    };
    let Some(ctx) = state.get("context") else {
        return HookOutput::default();
    };
    let Some(remaining) = ctx.get("remaining_percentage").and_then(Value::as_f64) else {
        return HookOutput::default();
    };
    let now = epoch(&f.now);
    let at = ctx.get("at").and_then(Value::as_str).and_then(epoch);
    let monitor = state.get("monitor");
    let compacted = monitor
        .and_then(|m| m.get("compacted_at"))
        .and_then(Value::as_str)
        .and_then(epoch);
    if let (Some(w), Some(n)) = (compacted, now)
        && w <= n + WATERMARK_SKEW_SECONDS
        && !at.is_some_and(|a| a > w + COMPACT_GRACE_SECONDS)
    {
        return HookOutput::default();
    }
    if let (Some(a), Some(n)) = (at, now)
        && n - a > STALE_SECONDS
    {
        return HookOutput::default();
    }
    if remaining > warning {
        return HookOutput::default();
    }
    let level = monitor.and_then(|m| m.get("level")).and_then(Value::as_str);
    let calls = monitor
        .and_then(|m| m.get("calls"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
        + 1;
    let is_critical = remaining <= critical;
    let current = if is_critical { "critical" } else { "warning" };
    let escalated = is_critical && level == Some("warning");
    if level.is_some() && calls < DEBOUNCE_CALLS && !escalated {
        return patch(json!({"monitor": {"calls": calls}}));
    }
    let used = ctx
        .get("used_pct")
        .and_then(Value::as_f64)
        .unwrap_or(100.0 - remaining);
    let tokens = |k: &str| ctx.get(k).and_then(Value::as_f64).filter(|n| n.is_finite());
    let detail = match (tokens("used_tokens"), tokens("threshold_tokens")) {
        (Some(u), Some(t)) => format!(
            " ({}k of {}k usable tokens)",
            (u / 1000.0).round() as i64,
            (t / 1000.0).round() as i64
        ),
        _ => String::new(),
    };
    let usage = format!(
        "Usage at {}%{detail}. Remaining: {}%. ",
        num(used),
        num(remaining)
    );
    let message = if is_critical {
        format!(
            "CONTEXT CRITICAL: {usage}Context is nearly exhausted. Do NOT start new complex work or write handoff \
             files, GSD state is already tracked in the project's state. Inform the user so they can run \
             /gsd-pause-work at the next natural stopping point."
        )
    } else {
        format!(
            "CONTEXT WARNING: {usage}Context is getting limited. Avoid starting new complex work. If not between \
             defined plan steps, inform the user so they can prepare to pause."
        )
    };
    let mut out = patch(json!({"monitor": {"calls": 0, "level": current}}));
    out.decision = Some(Decision::Allow);
    out.context = Some(message);
    out
}

/// Forgets the metrics and the debounce, and marks when the compaction began.
pub(super) fn context_compacted(f: &Facts) -> HookOutput {
    let at = if f.now.is_empty() {
        Value::Null
    } else {
        json!(f.now)
    };
    patch(json!({"context": null, "monitor": {"calls": null, "level": null, "compacted_at": at}}))
}

pub(super) fn declare(c: &mut ContributionsBuilder) {
    c.hook("context_monitor", HookEvent::PostTool, |h| {
        h.description("Warns when the context window is running out")
            .tools(&["shell", "write", "edit", "agent"])
            .when("hooks.context_warnings")
            .reads_state()
            .handler(|call| {
                let s = Settings::current();
                let th = resolve_thresholds(
                    s.get("hooks.context_warning_threshold"),
                    s.get("hooks.context_critical_threshold"),
                );
                Ok(context_monitor(&Facts::of(call), th))
            });
    });
    c.hook("context_compacted", HookEvent::PreCompact, |h| {
        h.description("Resets the context monitor when the host compacts")
            .handler(|call| Ok(context_compacted(&Facts::of(call))));
    });
}

#[cfg(test)]
mod tests;
