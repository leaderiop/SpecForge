//! The files GSD keeps, one table: every name of `gsd/core/spec/README.md`'s "Every GSD file maps to a kind", then
//! every name upstream's text keeps under `.planning/` that the table lacks, each with the kind it became (ADR 0332
//! D8). [`planning_names`] finds them in a text: the port's tests read it, and an importer of a GSD project can.

use std::ops::Range;

/// What a link into the tracked release's blobs starts with: upstream's text quoted by its address (ADR 0332 D4), which
/// names upstream's files, not a project's.
pub const UPSTREAM_BLOB: &str = concat!(
    "https://github.com/open-gsd/gsd-core/blob/v",
    crate::tracked_version!(),
    "/"
);

/// What a "written nowhere" name became: the README's list of what a prompt shows and stores nowhere.
const SHOWN: &str = "none: shown to the user, not stored";

/// `(name, kinds)`: first the README table's names, in its order (a cell naming several files gives a row each; prose
/// entries and `config.json` give none), then the names it lacks. A name is the README's form (`NN-CONTEXT.md`,
/// `milestones/*`) or a glob, where `*` is any run of characters but `/`.
pub const PLANNING_FILES: &[(&str, &str)] = &[
    ("PROJECT.md", "project, requirement, decision"),
    ("REQUIREMENTS.md", "requirement"),
    ("ROADMAP.md", "milestone, phase"),
    ("MILESTONES.md", "milestone, phase"),
    ("milestones/*", "milestone, phase"),
    (
        "STATE.md",
        "computed (progress, position) + pause, blocker + session state",
    ),
    ("RETROSPECTIVE.md", "retrospective, lesson, methodology"),
    ("LEARNINGS.md", "retrospective, lesson, methodology"),
    ("METHODOLOGY.md", "retrospective, lesson, methodology"),
    (
        "NN-CONTEXT.md",
        "context, decision, deferred_idea, discussion",
    ),
    (
        "NN-DISCUSSION-LOG.md",
        "context, decision, deferred_idea, discussion",
    ),
    (
        "NN-SPEC.md",
        "phase_spec, locked_req, interview, edge, spec_prohibition",
    ),
    ("NN-RESEARCH.md", "research and its items"),
    ("research/*.md", "research and its items"),
    (
        "NN-MM-PLAN.md",
        "plan, task, checkpoint, truth, artifact, key_link, prohibition, disposition, user_setup",
    ),
    (
        "NN-MM-SUMMARY.md",
        "summary, deliverable, deviation, decision",
    ),
    (".continue-here.md", "handoff"),
    (
        "NN-VERIFICATION.md",
        "verification, verify_gap, override, human_item, and verdicts",
    ),
    ("NN-UAT.md", "uat_session, uat_test, uat_gap"),
    (
        "NN-VALIDATION.md",
        "validation, validation_row, wave0_item, manual_check",
    ),
    ("v<version>-MILESTONE-AUDIT.md", "milestone_audit"),
    ("WINDOWS.md", "broken_window"),
    ("NN-REFACTOR.md", "refactor_proposal"),
    (
        "quick/*",
        "plan, summary, context, decision, research (a quick plan has no phase)",
    ),
    ("workstreams/*", "workstream (a scope)"),
    (
        "SECURITY.md",
        "threat, trust_boundary, security_audit, accepted_risk, audit_run",
    ),
    (
        "AI-SPEC.md",
        "ai_spec, ai_failure, eval_dimension, guardrail, eval_review",
    ),
    (
        "NN-EVAL-REVIEW.md",
        "ai_spec, ai_failure, eval_dimension, guardrail, eval_review",
    ),
    (
        "NN-UI-SPEC.md",
        "ui_spec, tokens, components, ui_review, findings",
    ),
    (
        "NN-UI-REVIEW.md",
        "ui_spec, tokens, components, ui_review, findings",
    ),
    ("codebase/*", "codebase kinds, pattern_map"),
    ("intel/*", "codebase kinds, pattern_map"),
    ("NN-PATTERNS.md", "codebase kinds, pattern_map"),
    ("debug/*", "ops kinds"),
    ("NN-REVIEW.md", "ops kinds"),
    ("todos/*", "ops kinds"),
    ("seeds/*", "ops kinds"),
    ("threads/*", "ops kinds"),
    ("BACKLOG.md", "ops kinds"),
    ("USER-PROFILE.md", "ops kinds"),
    // written nowhere, on purpose (the README's list under the table)
    ("REVIEWS.md", SHOWN),
    ("reports/*.md", SHOWN),
    ("onboarding/SUMMARY.md", SHOWN),
    ("INGEST-CONFLICTS.md", SHOWN),
    ("estimation-calibration.json", SHOWN),
    ("WORKSPACE.md", SHOWN),
    ("MILESTONE_SUMMARY-v*.md", SHOWN),
    ("SESSION_REPORT*.md", SHOWN),
    ("*-session-report.md", SHOWN),
    // the names upstream's text keeps under `.planning/` that the table lacks
    ("*continue-here*.md", "handoff"),
    ("HANDOFF.json", "handoff"),
    ("*HANDOFF*.md", "handoff"),
    ("agent-history.json", "handoff (session state)"),
    ("current-agent-id.txt", "handoff (session state)"),
    ("USER-SETUP.md", "user_setup"),
    ("deferred-items.md", "deferred_idea"),
    ("DECISIONS-INDEX.md", "decision (a query of the decisions)"),
    ("active-workstream", "workstream (session state)"),
    (".gsd-ws-arg", "workstream (session state)"),
    (".gsd-outgoing-milestone", "milestone (session state)"),
    (
        ".gsd-allow-shrink",
        "none: the milestone close's guard file, which the graph does not need",
    ),
    ("SEED-*.md", "ops kinds"),
    ("quick-batches/*", "plan, summary (a batch of quick plans)"),
    ("ui-reviews/*", "ui_review"),
    (
        "MEMORY.md",
        "none: the agent's memory notes, outside the graph",
    ),
    (
        "memory/*",
        "none: the agent's memory notes, outside the graph",
    ),
    ("async-jobs/*", "none: async jobs are not part of @gsd/core"),
    (
        "deliberations/*",
        "none: deliberations are not part of @gsd/core",
    ),
    ("graphs/*", "none: the knowledge graph a capability builds"),
    ("logs/*", "none: health logs are not part of @gsd/core"),
    ("sketches/*", "none: throwaway output outside the spec tree"),
    ("spikes/*", "none: throwaway output outside the spec tree"),
    ("tmp/*", "none: scratch files"),
];

/// The README's placeholder prefixes, longest first: a row `NN-MM-PLAN.md` names `PLAN.md` and `<anything>-PLAN.md`.
const PLACEHOLDERS: &[&str] = &["NN-MM-", "NN-", "v<version>-"];

/// Each byte range of `text` that names a GSD file, with the row it matches, or `".planning"` for the folder.
///
/// `.planning` is a hit wherever it stands. Otherwise a candidate is a maximal run of ASCII letters, digits and
/// `_{}$*<>.-/` (a trailing `.` dropped); a path into the asset tree (a `gsd-core`, `agents` or `commands/gsd`
/// segment) or a URL's (`//` after `:`) is not a name. A candidate matches a directory or path row (`quick/*`,
/// `onboarding/SUMMARY.md`) by its segments, and otherwise a file row by its last segment: equal to the row's name
/// less its placeholder prefix, or ending in `-` and that name, or matching the row's glob. Of the rows that match,
/// the most specific (the most characters that are not `*`) wins.
pub fn planning_names(text: &str) -> Vec<(Range<usize>, &'static str)> {
    let mut out: Vec<(Range<usize>, &'static str)> = text
        .match_indices(".planning")
        .map(|(at, s)| (at..at + s.len(), ".planning"))
        .collect();
    for range in candidates(text) {
        let candidate = &text[range.clone()];
        let url = text[..range.start].ends_with(':') && candidate.starts_with("//");
        if url || in_asset_tree(candidate) {
            continue;
        }
        if let Some(row) = row_of(candidate) {
            out.push((range, row));
        }
    }
    out.sort_by_key(|(r, _)| (r.start, r.end));
    out
}

/// The maximal runs of name characters, each without a trailing `.`.
fn candidates(text: &str) -> Vec<Range<usize>> {
    let name = |b: u8| b.is_ascii_alphanumeric() || b"_{}$*<>.-/".contains(&b);
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !name(bytes[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && name(bytes[i]) {
            i += 1;
        }
        let end = start + text[start..i].trim_end_matches('.').len();
        if end > start {
            out.push(start..end);
        }
    }
    out
}

/// Whether `candidate` is a path into the extension's asset tree, which names an asset, not a GSD file.
fn in_asset_tree(candidate: &str) -> bool {
    let segments: Vec<&str> = candidate.split('/').collect();
    segments.iter().any(|s| *s == "gsd-core" || *s == "agents")
        || segments.windows(2).any(|w| w == ["commands", "gsd"])
}

/// The most specific row `candidate` matches: a directory or path row first, then a file row by its last segment.
fn row_of(candidate: &str) -> Option<&'static str> {
    let segments: Vec<&str> = candidate.split('/').collect();
    let best = |rows: &mut dyn Iterator<Item = &'static str>| {
        rows.max_by_key(|row| row.chars().filter(|c| *c != '*').count())
    };
    let pathed = best(
        &mut PLANNING_FILES
            .iter()
            .map(|(row, _)| *row)
            .filter(|row| row.contains('/') && path_matches(row, &segments)),
    );
    if pathed.is_some() {
        return pathed;
    }
    let last = segments.iter().rev().find(|s| !s.is_empty())?;
    best(
        &mut PLANNING_FILES
            .iter()
            .map(|(row, _)| *row)
            .filter(|row| !row.contains('/') && file_matches(row, last)),
    )
}

/// Whether the segments of a candidate match the row `row` holding a `/`: a directory row (`quick/*`,
/// `research/*.md`) by its directory standing first or after a `.planning` or placeholder segment (`$DIR`, `{dir}`,
/// `<dir>`), the candidate going on past it, and for `*.md` its last segment after it being a `.md` file, if any; a
/// path row (`onboarding/SUMMARY.md`) by the candidate's last segments.
fn path_matches(row: &str, segments: &[&str]) -> bool {
    if segments.len() < 2 {
        return false;
    }
    let Some((dir, tail)) = row.split_once("/*") else {
        let want: Vec<&str> = row.split('/').collect();
        return segments.ends_with(&want);
    };
    let lead = |s: &str| s == ".planning" || s.starts_with(['$', '{', '<']);
    (0..segments.len()).any(|i| {
        segments[i] == dir
            && segments[..i].iter().all(|s| lead(s))
            && match segments[i + 1..].iter().rev().find(|s| !s.is_empty()) {
                Some(last) => last.ends_with(tail),
                None => true,
            }
    })
}

/// Whether the last segment `c` of a candidate matches the file row `row`.
fn file_matches(row: &str, c: &str) -> bool {
    if row.contains('*') {
        return glob(row, c);
    }
    let base = PLACEHOLDERS
        .iter()
        .find_map(|p| row.strip_prefix(p))
        .unwrap_or(row);
    c == base || c.strip_suffix(base).is_some_and(|head| head.ends_with('-'))
}

/// Whether `text` matches `pattern`, where `*` is any run of characters (no `/` is in either).
fn glob(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let (first, rest) = parts.split_first().expect("split gives one part at least");
    let Some(mut left) = text.strip_prefix(first) else {
        return false;
    };
    let Some((last, middle)) = rest.split_last() else {
        return left.is_empty();
    };
    for part in middle {
        match left.find(part) {
            Some(at) => left = &left[at + part.len()..],
            None => return false,
        }
    }
    left.len() >= last.len() && left.ends_with(last)
}

#[cfg(test)]
mod tests;
