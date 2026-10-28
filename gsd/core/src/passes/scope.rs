//! The scope pass: `gsd/E312` and `gsd/W317`.
//!
//! E312: no task action reduces scope. GSD has no code matcher: the planner is told never to write these phrases in a
//! task action (`agents/gsd-planner.md:81-85`, `references/planner-antipatterns.md:90-93`, MIT, `LICENSE-GSD`), and
//! the rule is that prohibition. The checker's wider list (stub, hours, too complex) is judgment: `gsd/W311` (r40).
//! Upstream's checker (`agents/gsd-plan-checker.md:430-446`) cross-references each hit with the context decision the
//! plan claims to implement before it blocks, so a hit is exempt when the action cites a decision of the plan's phase
//! (`\bD-NN\b`, as E310 matches). A dotted version is no hit: `v1.0` is a release, not a scope cut.
//!
//! W317: the post-planning gap report (`src/gap-checker.cts`). The items are the phase's requirement ids and every
//! `D-NN` of its context; an item is covered when `\b<id>\b` is in the concatenated text of the phase's plans.

use specforge_extension_sdk::prelude::*;

use super::plans::{children, has_word, name_of, of_kind, phase_of, strings, texts};
use crate::digest::scope_of;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("scope", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = scope_reduction(input);
    found.extend(post_planning_gaps(input));
    found
}

/// The planner's prohibited phrases in a task action (`agents/gsd-planner.md:81-85`), in its order. The fourth
/// bullet there ("any language that reduces a source artifact decision to less than what was specified") is judgment.
pub const SCOPE_REDUCTION: &[&str] = &[
    "v1",
    "v2",
    "simplified version",
    "static for now",
    "hardcoded for now",
    "future enhancement",
    "placeholder",
    "basic version",
    "minimal implementation",
    "will be wired later",
    "dynamic in future phase",
    "skip for now",
];

/// A version (`v1.0`, `3.v1`) reads as one token: a `.` between two alphanumerics, one of them a digit, becomes a word
/// character, so `v1` is not a hit in it. A sentence-ending `v1.` still is.
fn without_versions(action: &str) -> String {
    let chars: Vec<char> = action.chars().collect();
    (0..chars.len())
        .map(|i| {
            let at = |j: Option<usize>| j.and_then(|j| chars.get(j)).copied().unwrap_or(' ');
            let (before, after) = (at(i.checked_sub(1)), at(Some(i + 1)));
            let inside = before.is_ascii_alphanumeric()
                && after.is_ascii_alphanumeric()
                && (before.is_ascii_digit() || after.is_ascii_digit());
            if chars[i] == '.' && inside {
                '_'
            } else {
                chars[i]
            }
        })
        .collect()
}

/// The action cites a decision of the plan's phase: the cross-reference upstream makes before it blocks.
fn cites_decision(input: &PassInput, task: &PassEntity, action: &str) -> bool {
    phase_of(input, crate::digest::scope_of(&task.id)).is_some_and(|phase| {
        children(input, phase, "decision")
            .iter()
            .any(|d| has_word(action, name_of(&d.id), false))
    })
}

fn scope_reduction(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "task")
        .filter_map(|task| {
            let action = task.fields.get("action")?;
            let plain = without_versions(action);
            let phrase = SCOPE_REDUCTION
                .iter()
                .find(|phrase| has_word(&plain, phrase, true))?;
            if cites_decision(input, task, action) {
                return None;
            }
            Some(
                PassDiagnostic::new(
                    "E312",
                    PassSeverity::Error,
                    format!("task '{}' reduces scope: its action says \"{phrase}\"", task.id),
                )
                .with_suggestion("deliver what the decision says, or split the plan: never simplify a user decision; a task that cites the decision it implements (D-NN) is not reported")
                .with_entity(&task.id),
            )
        })
        .collect()
}

/// One row of the gap table.
pub struct Row {
    pub source: &'static str,
    pub item: String,
    pub covered: bool,
}

/// `naturalKey`: digit runs padded to 8, so `REQ-2` sorts before `REQ-10`.
fn natural_key(item: &str) -> String {
    let mut key = String::new();
    let mut digits = String::new();
    for c in item.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        if !digits.is_empty() {
            key.push_str(&format!("{digits:0>8}"));
            digits.clear();
        }
        key.push(c);
    }
    key.pop();
    key
}

/// `sortRows`: REQUIREMENTS.md rows, then CONTEXT.md rows, each by natural key.
pub fn sort_rows(rows: &mut [Row]) {
    let order = |source: &str| if source == "REQUIREMENTS.md" { 0 } else { 1 };
    rows.sort_by(|a, b| {
        order(a.source)
            .cmp(&order(b.source))
            .then_with(|| natural_key(&a.item).cmp(&natural_key(&b.item)))
    });
}

/// `formatGapTable` plus the summary line, as `gsd-tools query check.gap-analysis-plan-post` words its `table`.
pub fn table(rows: &[Row]) -> String {
    if rows.is_empty() {
        return "## Post-Planning Gap Analysis\n\nNo requirements or decisions to check.\n".into();
    }
    let body: Vec<String> = rows
        .iter()
        .map(|r| {
            let status = if r.covered {
                "✓ Covered"
            } else {
                "✗ Not covered"
            };
            format!("| {} | {} | {status} |", r.source, r.item)
        })
        .collect();
    format!(
        "## Post-Planning Gap Analysis\n\n| Source | Item | Status |\n|--------|------|--------|\n{}\n\n{}\n",
        body.join("\n"),
        summary(rows)
    )
}

/// `⚠ {u} of {n} items not covered by any plan`, or `✓ All {n} items covered by plans`.
pub fn summary(rows: &[Row]) -> String {
    let uncovered = rows.iter().filter(|r| !r.covered).count();
    if uncovered == 0 {
        format!("✓ All {} items covered by plans", rows.len())
    } else {
        format!(
            "⚠ {uncovered} of {} items not covered by any plan",
            rows.len()
        )
    }
}

/// W317, per planned phase (one with at least one plan: GSD runs the gate at `plan:post`).
fn post_planning_gaps(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "phase")
        .filter_map(|phase| {
            let plans: Vec<&PassEntity> = children(input, &phase.id, "plan");
            if plans.is_empty() {
                return None;
            }
            let text: String = plans
                .iter()
                .flat_map(|plan| std::iter::once(*plan).chain(children_of_plan(input, &plan.id)))
                .flat_map(texts)
                .collect::<Vec<_>>()
                .join("\n");
            let mut rows: Vec<Row> = strings(phase, "requirements")
                .into_iter()
                .map(|item| (item, "REQUIREMENTS.md"))
                .chain(
                    children(input, &phase.id, "decision")
                        .into_iter()
                        .filter(|d| d.fields.get("origin").map(String::as_str) == Some("context"))
                        .map(|d| (name_of(&d.id).to_string(), "CONTEXT.md")),
                )
                .map(|(item, source)| Row {
                    covered: has_word(&text, &item, false),
                    source,
                    item,
                })
                .collect();
            sort_rows(&mut rows);
            let open: Vec<&str> = rows
                .iter()
                .filter(|r| !r.covered)
                .map(|r| r.item.as_str())
                .collect();
            (!open.is_empty()).then(|| {
                PassDiagnostic::new(
                    "W317",
                    PassSeverity::Warning,
                    format!("{}: {}", summary(&rows), open.join(", ")),
                )
                .with_suggestion(table(&rows))
                .with_entity(&phase.id)
            })
        })
        .collect()
}

/// Everything scoped by a plan: its tasks, checkpoints and must-haves.
fn children_of_plan<'a>(
    input: &'a PassInput,
    plan: &'a str,
) -> impl Iterator<Item = &'a PassEntity> {
    input
        .entities
        .iter()
        .filter(move |e| scope_of(&e.id) == plan)
}

#[cfg(test)]
mod tests;
