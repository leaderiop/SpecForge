//! The body lines (0-based, inclusive runs) rewritten because the git command they describe became a `specforge git`
//! verb whose answer reads differently: ship's TDD audit takes each commit as one line of JSON from `specforge git log
//! --pick '.commits[]'` (its sha, subject and trailers) where upstream split `\x1f`/`\x1e`-delimited records.
//! `port_workflows` reads them as more `REWRITTEN_SPANS` rows.

pub const SPANS: &[(&str, usize, usize)] = &[
    ("gsd-core/workflows/ship.md", 322, 322),
    ("gsd-core/workflows/ship.md", 325, 325),
    ("gsd-core/workflows/ship.md", 337, 337),
];
