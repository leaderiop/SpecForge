//! Ported from gsd-core v1.18.0 src/complexity-trigger.cts (`analyzeSource`, `isAnalyzablePath`, `evaluateCandidates`), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The decision-point complexity scorer of the refactor trigger (D-r41-15): `strip` blanks comments and literals,
//! `scan` scores each function, and [`evaluate`] picks the functions whose score crosses the threshold or jumped from
//! their baseline. Upstream's baseline file, proposal file and git adapter are not ported: SpecForge keeps the baseline
//! in the disposed `refactor_proposal`s and the files in the plan's `files_modified`.
//!
//! Also ports gsd-core bin/lib modules: clusters.cjs, estimate-cli.cjs.
//!
//! gsd-core modules this file ports: complexity-trigger.cjs.

mod scan;
mod strip;

#[cfg(test)]
mod tests;

pub use strip::Unparseable;

/// Upstream's `DEFAULTS`: a score above 15, or a rise of more than 5 over the baseline, is a candidate.
pub const THRESHOLD: i64 = 15;
pub const JUMP_DELTA: i64 = 5;

/// Upstream's `ANALYZABLE_EXTENSIONS`.
const ANALYZABLE_EXTENSIONS: &[&str] = &[".js", ".cjs", ".mjs", ".ts", ".cts", ".mts"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionScore {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub score: u32,
}

/// The score of every function of `source`, or [`Unparseable`] when a literal or a comment never ends.
pub fn analyze_source(source: &str) -> Result<Vec<FunctionScore>, Unparseable> {
    // CRLF is normalised first, so scores and line numbers are the same either way.
    let normalized: Vec<char> = source.replace("\r\n", "\n").chars().collect();
    let stripped = strip::strip_literals(&normalized)?;
    Ok(scan::scan_functions(&stripped))
}

/// Whether the file is scored: an analyzable extension, not under `tests/` or `gsd-core/bin/lib/`.
pub fn is_analyzable_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let name = normalized.rsplit('/').next().unwrap_or_default();
    let extension = match name.rfind('.') {
        Some(0) | None => "",
        Some(dot) => &name[dot..],
    };
    ANALYZABLE_EXTENSIONS.contains(&extension)
        && normalized != "tests"
        && !normalized.starts_with("tests/")
        && normalized != "gsd-core/bin/lib"
        && !normalized.starts_with("gsd-core/bin/lib/")
}

/// One analysed file: its scores, or why it was skipped.
pub struct Analyzed {
    pub file: String,
    pub functions: Result<Vec<FunctionScore>, Unparseable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub file: String,
    pub name: String,
    pub start_line: usize,
    pub score: i64,
    /// `None` when the function has no baseline: never `0`, and never read as a jump.
    pub baseline: Option<i64>,
    /// `None` when `baseline` is `None`: never `score`.
    pub delta: Option<i64>,
    /// `threshold`, `jump`, or both in that order.
    pub reasons: Vec<&'static str>,
}

/// Upstream's `evaluateCandidates`: the functions over `threshold`, or up by more than `jump_delta` from their baseline
/// (`file::name` to score), best first. A threshold or jump that is not positive falls back to the default.
pub fn evaluate(
    analyzed: &[Analyzed],
    baseline: &std::collections::BTreeMap<String, i64>,
    threshold: i64,
    jump_delta: i64,
) -> Vec<Candidate> {
    let threshold = if threshold > 0 { threshold } else { THRESHOLD };
    let jump_delta = if jump_delta > 0 {
        jump_delta
    } else {
        JUMP_DELTA
    };
    let mut candidates = Vec::new();
    for entry in analyzed {
        let Ok(functions) = &entry.functions else {
            continue;
        };
        for function in functions {
            let score = i64::from(function.score);
            let before = baseline
                .get(&format!("{}::{}", entry.file, function.name))
                .copied();
            let mut reasons = Vec::new();
            if score > threshold {
                reasons.push("threshold");
            }
            let delta = before.map(|b| score - b);
            if delta.is_some_and(|d| d > jump_delta) {
                reasons.push("jump");
            }
            if !reasons.is_empty() {
                candidates.push(Candidate {
                    file: entry.file.clone(),
                    name: function.name.clone(),
                    start_line: function.start_line,
                    score,
                    baseline: before,
                    delta,
                    reasons,
                });
            }
        }
    }
    // Score descending, delta descending (no baseline last), then file, name and line ascending.
    candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(b.delta.cmp(&a.delta))
            .then(a.file.cmp(&b.file))
            .then(a.name.cmp(&b.name))
            .then(a.start_line.cmp(&b.start_line))
    });
    candidates
}
