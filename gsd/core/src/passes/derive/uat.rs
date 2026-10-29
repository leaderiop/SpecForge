//! The computed values of UAT (`spec/verify.md`, D-r39-8): a session's totals and status, and a gap's resolution.
//!
//! The status is `uat-predicate.cts:106-129`: a pending test keeps the session `testing`; it is `complete` when no test
//! is blocked, every skipped test has a `Deferred follow-up` reason and every issue has a resolved gap; otherwise
//! `diagnosed` when every open gap has a root cause, else `partial`. A gap is resolved once a complete
//! `gap_closure` plan names it.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::execute::{summaries, value};
use super::progress::Statuses;
use crate::passes::plans::{link, links, of_kind, text};

fn result(test: &PassEntity) -> &str {
    text(test, "result").unwrap_or("pending")
}

/// The complete plan whose `gap_closure` names `gap`.
fn closer<'a>(input: &'a PassInput, gap: &str, statuses: &Statuses) -> Option<&'a PassEntity> {
    of_kind(input, "plan")
        .filter(|p| links(input, p, "gap_closure").contains(&gap))
        .find(|p| statuses.get(p.id.as_str()) == Some(&"complete"))
}

/// `Deferred follow-up…`, case-insensitive, at the start of the reason.
pub fn is_deferred(reason: Option<&str>) -> bool {
    reason.is_some_and(|r| {
        r.trim_start()
            .to_lowercase()
            .starts_with("deferred follow-up")
    })
}

/// What the session's tests and gaps say, for `status`.
pub struct Facts {
    pub pending: bool,
    pub blocked: bool,
    pub unexcused_skip: bool,
    pub unresolved_issue: bool,
    /// Open gaps, and whether each has a root cause.
    pub open_gaps: Vec<bool>,
}

pub fn status(f: &Facts) -> &'static str {
    if f.pending {
        "testing"
    } else if !f.blocked && !f.unexcused_skip && !f.unresolved_issue {
        "complete"
    } else if !f.open_gaps.is_empty() && f.open_gaps.iter().all(|c| *c) {
        "diagnosed"
    } else {
        "partial"
    }
}

pub fn values(input: &PassInput, statuses: &Statuses) -> Vec<PassValue> {
    let mut out = Vec::new();
    let summaries = summaries(input);
    for gap in of_kind(input, "uat_gap") {
        let closed = closer(input, &gap.id, statuses);
        out.push(value(
            &gap.id,
            "status",
            if closed.is_some() { "resolved" } else { "open" },
        ));
        if let Some(plan) = closed {
            out.push(value(&gap.id, "resolved_by", plan.id.clone()));
            if let Some(at) = summaries
                .get(plan.id.as_str())
                .and_then(|s| text(s, "completed"))
            {
                out.push(value(&gap.id, "resolved_at", at));
            }
        }
    }
    for session in of_kind(input, "uat_session") {
        let tests: Vec<&PassEntity> = of_kind(input, "uat_test")
            .filter(|t| link(input, t, "session") == Some(session.id.as_str()))
            .collect();
        let count = |r: &str| tests.iter().filter(|t| result(t) == r).count();
        out.push(value(
            &session.id,
            "totals",
            json!({
                "total": tests.len(),
                "passed": count("pass"),
                "issues": count("issue"),
                "pending": count("pending"),
                "skipped": count("skipped"),
                "blocked": count("blocked"),
            }),
        ));
        let gaps_of = |test: &PassEntity| -> Vec<&PassEntity> {
            of_kind(input, "uat_gap")
                .filter(|g| link(input, g, "test") == Some(test.id.as_str()))
                .collect()
        };
        let resolved = |g: &PassEntity| closer(input, &g.id, statuses).is_some();
        let facts = Facts {
            pending: count("pending") > 0,
            blocked: count("blocked") > 0,
            unexcused_skip: tests
                .iter()
                .any(|t| result(t) == "skipped" && !is_deferred(text(t, "reason"))),
            unresolved_issue: tests.iter().any(|t| {
                let gaps = gaps_of(t);
                // Upstream: every gap of the issue is resolved.
                result(t) == "issue" && !(!gaps.is_empty() && gaps.iter().all(|g| resolved(g)))
            }),
            open_gaps: tests
                .iter()
                .flat_map(|t| gaps_of(t))
                .filter(|g| !resolved(g))
                .map(|g| text(g, "root_cause").is_some())
                .collect(),
        };
        out.push(value(&session.id, "status", status(&facts)));
    }
    out
}

#[cfg(test)]
mod tests;
