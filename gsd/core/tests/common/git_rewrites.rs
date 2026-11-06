//! The body lines (0-based, inclusive runs) rewritten because the git command they describe became a `specforge git`
//! verb whose answer reads differently: ship's TDD audit takes each commit as one line of JSON from `specforge git log
//! --pick '.commits[]'` (its sha, subject and trailers) where upstream split `\x1f`/`\x1e`-delimited records.
//! `port_workflows` reads them as more `REWRITTEN_SPANS` rows.

pub const SPANS: &[(&str, usize, usize)] = &[
    ("gsd-core/workflows/ship.md", 322, 322),
    ("gsd-core/workflows/ship.md", 325, 325),
    ("gsd-core/workflows/ship.md", 337, 337),
    ("gsd-core/workflows/remove-workspace.md", 66, 66),
    ("gsd-core/workflows/remove-workspace.md", 74, 74),
    ("gsd-core/references/worktree-path-safety.md", 36, 39),
    ("gsd-core/references/worktree-path-safety.md", 41, 41),
    ("gsd-core/references/worktree-path-safety.md", 104, 104),
    ("gsd-core/workflows/pr-branch.md", 240, 246),
    ("gsd-core/workflows/pr-branch.md", 271, 272),
    ("gsd-core/workflows/pr-branch.md", 498, 499),
    ("gsd-core/workflows/pr-branch.md", 529, 529),
    ("gsd-core/workflows/pr-branch.md", 564, 564),
];
