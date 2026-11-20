//! The `Gsd-Task:` trailer: what a commit's trailer values say, read for their shape only (ADR 0331 D3, ADR 0380), and
//! the task a value names, which the graph decides.

use specforge_extension_sdk::prelude::*;

use super::{clean, uncoded};
use crate::collectors::graph::find_kind;

/// What a commit's `Gsd-Task:` trailers say.
#[derive(Debug, PartialEq, Eq)]
pub enum Trailer<'a> {
    /// There is none.
    None,
    /// One distinct `<scope>/<task>` value, as written (`01-01/T1`, `v1.0/01/01-01/T1`).
    Named(&'a str),
    /// Two different values, or one that is no `<scope>/<task>`: no task, as upstream counts a doubled `gate-status:`
    /// as missing (`ship.md:335`).
    Unreadable,
}

impl Trailer<'_> {
    /// What the plan's title says of a trailer that names no task: its value (cleaned, cut at 60 characters), or
    /// `unreadable`; `None` when there is no trailer.
    pub(super) fn shown(&self) -> Option<String> {
        match self {
            Trailer::None => None,
            Trailer::Named(value) => Some(clean(value).chars().take(60).collect()),
            Trailer::Unreadable => Some("unreadable".to_string()),
        }
    }
}

/// The trailer `values` say: each trimmed, empty ones dropped, repeats counted once. A value is `<scope>/<task>`,
/// split at its last `/` (ADR 0380): the task a [`part`], and the scope `/`-separated parts ([`parts`]), so the plan
/// as a subject's scope writes it (`01-01/T1`) and the task's full id (`v1.0/01/01-01/T1`) are both read. The shape
/// is all it checks: the graph decides which task, if any, a value names.
pub fn trailer(values: &[String]) -> Trailer<'_> {
    let mut named: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();
    named.sort_unstable();
    named.dedup();
    match named[..] {
        [] => Trailer::None,
        [one]
            if one
                .rsplit_once('/')
                .is_some_and(|(scope, task)| parts(scope) && part(task)) =>
        {
            Trailer::Named(one)
        }
        _ => Trailer::Unreadable,
    }
}

/// One segment of a trailer value: non-empty ASCII letters, digits, `.`, `_` and `-` (no space, tab, separator, `:`
/// or control character).
fn part(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Whether every `/`-separated segment of a trailer's scope is a [`part`]: a leading, doubled or trailing `/` leaves an
/// empty one, so a rooted key or an empty segment is never read.
fn parts(scope: &str) -> bool {
    scope.split('/').all(part)
}

/// The task a trailer value names (`01-01/T1`, `v1.0/01/01-01/T1`), as written (the exact id, else a unique
/// `/`-boundary suffix) or with the configured `code` prefix stripped from its scope (`APP-01-01/T1`), as
/// [`super::plan_by_scope`] strips it from a subject's scope.
pub(super) fn task_named<'n>(
    nodes: &'n [GraphNode],
    code: Option<&str>,
    value: &str,
) -> Option<&'n GraphNode> {
    find_kind(nodes, value, "task").or_else(|| {
        let (scope, task) = value.rsplit_once('/')?;
        find_kind(nodes, &format!("{}/{task}", uncoded(scope, code)?), "task")
    })
}

#[cfg(test)]
mod tests;
