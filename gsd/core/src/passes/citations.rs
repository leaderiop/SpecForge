//! The one citation reader (ADR 0371 D2): which decisions a plan cites, by their id written in its text.
//!
//! A plan cites a decision when the decision's name (`D-01`) is a whole word of the plan's text, JavaScript's `\b` with
//! the case as written ([`words::word_starts`]), as upstream's `decisionMentioned` (`src/decision-coverage-support.cts`)
//! and its gap checker (`src/gap-checker.cts`) read it. The word is read with the path written before it (`01/D-01`,
//! `v1.0/01/D-01`, a rooted `/v1.0/01/D-01`) and resolved from the plan by its scope chain
//! ([`plans::written_target`], ADR 0284 D4): a bare `D-01` is the plan's own phase's decision, `01/D-01` phase 01's,
//! and a path that resolves to no decision (`src/D-01`) cites nothing.
//!
//! Where it diverges from upstream (the parity acceptance list's `[decision-citation-id-only]`): no soft phrase (upstream
//! also counts the first six normalised words of a decision's text), the path before the id is read (upstream matches
//! the id inside any path), and an id inside a fenced block or an HTML comment of a field counts (upstream's plan gate
//! strips them first).
//!
//! `gsd/E310`, the decision rows of `gsd/W317` and `gsd/E312`'s exemption read [`cited_by`] (ADR 0371 D3); `gsd/W506`
//! reads [`shipped_by`] (D4).

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::commits;
use super::plans::{children, name_of, of_kind, text, texts, written_target};
use super::words;
use crate::scope::scope_of;

/// Every decision id of the input: what a citation may resolve to.
pub fn decisions(input: &PassInput) -> BTreeSet<&str> {
    of_kind(input, "decision").map(|d| d.id.as_str()).collect()
}

/// A plan's text, what a citation is read in: upstream's designated sections (`extractPlanDesignatedSections`,
/// `src/decision-coverage-support.cts`), which are `<objective>`, the tasks and the must-haves. Here the plan's
/// `objective`, `purpose` and `output`, then every field of its tasks, checkpoints, truths, artifacts, key links and
/// prohibitions, joined by a line break. Not its context references, its verification or its success criteria, and no
/// title (the pass input carries none). Fenced code and HTML comments are not stripped.
pub fn plan_text(input: &PassInput, plan: &PassEntity) -> String {
    let mut parts: Vec<String> = ["objective", "purpose", "output"]
        .iter()
        .filter_map(|f| text(plan, f).map(String::from))
        .collect();
    for kind in [
        "task",
        "checkpoint",
        "truth",
        "artifact",
        "key_link",
        "prohibition",
    ] {
        for child in children(input, &plan.id, kind) {
            parts.extend(texts(child));
        }
    }
    parts.join("\n")
}

/// A byte of a path segment written before an id: an ASCII letter or digit, `.`, `_` or `-`.
fn is_segment(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

/// For each byte offset of `text` (and its end), where the token of a name starting there starts: the name extends
/// left while the byte before it is `/`, over the run of segment bytes before that `/`; an empty run makes the `/` a
/// rooted start and ends the walk. Computed left to right in one pass (a walk from a run's start is the walk of a name
/// starting there), so reading every occurrence costs the text's length once.
fn token_starts(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    // `run[i]`: where the run of segment bytes ending just before `i` starts.
    let mut run = Vec::with_capacity(bytes.len() + 1);
    run.push(0);
    for (i, byte) in bytes.iter().enumerate() {
        run.push(if is_segment(*byte) { run[i] } else { i + 1 });
    }
    let mut start: Vec<usize> = Vec::with_capacity(bytes.len() + 1);
    for at in 0..=bytes.len() {
        let token = match at.checked_sub(1) {
            Some(slash) if bytes[slash] == b'/' => {
                let segment = run[slash];
                if segment == slash {
                    slash
                } else {
                    start[segment]
                }
            }
            _ => at,
        };
        start.push(token);
    }
    start
}

/// The decisions among `decisions` (ids) that `text`, written in `holder`, cites: each whole-word occurrence of a
/// decision's name, read with the path written before it and resolved from `holder` by [`written_target`]. A token it
/// does not resolve cites nothing.
pub fn cited<'a>(decisions: &BTreeSet<&'a str>, holder: &str, text: &str) -> BTreeSet<&'a str> {
    let names: BTreeSet<&str> = decisions.iter().map(|id| name_of(id)).collect();
    let mut starts: Option<Vec<usize>> = None;
    let mut found = BTreeSet::new();
    for name in names {
        for at in words::word_starts(text, name) {
            let starts = starts.get_or_insert_with(|| token_starts(text));
            let token = &text[starts[at]..at + name.len()];
            found.extend(written_target(decisions, holder, token));
        }
    }
    found
}

/// The decisions the plans `plans` cite, each read from its own plan: the union of [`cited`] over each plan's
/// [`plan_text`].
pub fn cited_by<'a, 'b>(
    input: &'a PassInput,
    plans: impl IntoIterator<Item = &'b PassEntity>,
) -> BTreeSet<&'a str> {
    let decisions = decisions(input);
    plans
        .into_iter()
        .flat_map(|plan| cited(&decisions, &plan.id, &plan_text(input, plan)))
        .collect()
}

/// The decisions the plans `plans` shipped (ADR 0371 D4): what [`cited_by`] reads, then the text of each summary the
/// plan scopes and of each entity such a summary scopes (its coverage items and deviations), then the subject of each
/// recorded commit of the plan; each read from its plan. No file content and no commit body (upstream's verify gate
/// reads both, `[decision-shipped-reach]`).
pub fn shipped_by<'a, 'b>(
    input: &'a PassInput,
    plans: impl IntoIterator<Item = &'b PassEntity>,
) -> BTreeSet<&'a str> {
    let decisions = decisions(input);
    let commits = commits::commits(input);
    let mut found = BTreeSet::new();
    for plan in plans {
        found.extend(cited(&decisions, &plan.id, &plan_text(input, plan)));
        for summary in of_kind(input, "summary").filter(|s| scope_of(&s.id) == plan.id) {
            let scoped = input
                .entities
                .iter()
                .filter(|e| scope_of(&e.id) == summary.id);
            for entity in std::iter::once(summary).chain(scoped) {
                found.extend(cited(&decisions, &plan.id, &texts(entity).join("\n")));
            }
        }
        for commit in commits
            .iter()
            .filter(|c| c.plan.as_deref() == Some(plan.id.as_str()))
        {
            found.extend(cited(&decisions, &plan.id, &commit.subject));
        }
    }
    found
}

#[cfg(test)]
mod tests;
