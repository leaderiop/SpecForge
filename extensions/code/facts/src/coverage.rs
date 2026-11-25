//! The `coverage` facts of a coverage report (ADR 0388): what a reader (`specforge-code-lcov`) read for each file, and
//! the per-item counts minted from it, each citing the innermost served `item` that holds the report's lines.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::Fact;

use crate::places::Places;
use crate::report::{id_part, COVERAGE};

/// What a coverage report says of one file. An empty vector means the report gave no record of that family for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoverageFile {
    /// The file, project-relative, with `/` separators.
    pub file: String,
    pub functions: Vec<Function>,
    /// Each line's `(1-based line, hits)`.
    pub lines: Vec<(usize, u64)>,
    pub branches: Vec<Branch>,
}

/// A function of a coverage report: where it starts, its name as the report wrote it, and how often it ran.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Function {
    /// Its 1-based start line; 0 when the report gave none.
    pub line: usize,
    pub name: String,
    pub hits: u64,
}

/// A branch of a coverage report: its line, block and branch as written, and how often it was taken (`None`: its
/// condition was never evaluated, LCOV's `-`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Branch {
    pub line: usize,
    pub block: String,
    pub branch: String,
    pub taken: Option<u64>,
}

/// One file's records, its sections merged: a line's hits add up, a function by `(line, name)` and a branch by
/// `(line, block, branch)`, a branch untaken (`None`) only when every record of it was.
#[derive(Default)]
struct Merged<'f> {
    lines: BTreeMap<usize, u64>,
    functions: BTreeMap<(usize, &'f str), u64>,
    branches: BTreeMap<(usize, &'f str, &'f str), Option<u64>>,
}

/// The `(found, hit)` counts of each family one fact gathers; `None` for a family its files held no record of.
#[derive(Default)]
struct Counts {
    item: Option<String>,
    file: String,
    functions: Option<(u64, u64)>,
    lines: Option<(u64, u64)>,
    branches: Option<(u64, u64)>,
}

/// The `coverage` facts of `files`, attributed by the places of the `served` `item` facts, sorted by id:
///
/// - `cov:<item id>` for each item the innermost holder of a line, a branch's line or a function's start line, with
///   `item`, `file` (the smallest file attributed to it) and the counts `functions_found`/`functions_hit` (distinct
///   functions, those run), `lines_found`/`lines_hit` and `branches_found`/`branches_hit` (taken), a family's two only
///   when its file held a record of it;
/// - `cov:@<file>` for what no item holds in a file, with `file` and the counts and no `item`.
///
/// The report's own file totals (LCOV `LF`, `FNF`, …) are never read: a total is not an item's.
pub fn coverage_facts(files: &[CoverageFile], served: &[Fact]) -> Vec<Fact> {
    let places = Places::new(served);
    let mut merged: BTreeMap<&str, Merged> = BTreeMap::new();
    for file in files {
        let into = merged.entry(file.file.as_str()).or_default();
        for &(line, hits) in &file.lines {
            let kept = into.lines.entry(line).or_default();
            *kept = kept.saturating_add(hits);
        }
        for function in &file.functions {
            let kept = into
                .functions
                .entry((function.line, function.name.as_str()))
                .or_default();
            *kept = kept.saturating_add(function.hits);
        }
        for branch in &file.branches {
            let key = (branch.line, branch.block.as_str(), branch.branch.as_str());
            let kept = into.branches.entry(key).or_insert(None);
            *kept = match (*kept, branch.taken) {
                (Some(a), Some(b)) => Some(a.saturating_add(b)),
                (a, b) => a.or(b),
            };
        }
    }
    let mut facts: BTreeMap<String, Counts> = BTreeMap::new();
    for (&file, records) in &merged {
        // Each fact a record of this file reaches holds every family the file has, at zero until counted.
        let mut count =
            |line: usize, family: fn(&mut Counts) -> &mut Option<(u64, u64)>, hit: bool| {
                let item = places.innermost(file, line);
                let id = match item {
                    Some(item) => format!("{}:{item}", COVERAGE.scheme),
                    None => format!("{}:@{}", COVERAGE.scheme, id_part(file)),
                };
                let counts = facts.entry(id).or_insert_with(|| Counts {
                    item: item.map(str::to_string),
                    file: file.to_string(),
                    ..Counts::default()
                });
                if file < counts.file.as_str() {
                    counts.file = file.to_string();
                }
                let present = [
                    (!records.functions.is_empty(), &mut counts.functions),
                    (!records.lines.is_empty(), &mut counts.lines),
                    (!records.branches.is_empty(), &mut counts.branches),
                ];
                for (held, slot) in present {
                    if held {
                        slot.get_or_insert((0, 0));
                    }
                }
                if let Some((found, hits)) = family(counts) {
                    *found += 1;
                    *hits += u64::from(hit);
                }
            };
        for (&(line, _), &hits) in &records.functions {
            count(line, |c| &mut c.functions, hits > 0);
        }
        for (&line, &hits) in &records.lines {
            count(line, |c| &mut c.lines, hits > 0);
        }
        for (&(line, _, _), &taken) in &records.branches {
            count(line, |c| &mut c.branches, taken.is_some_and(|t| t > 0));
        }
    }
    facts
        .into_iter()
        .map(|(id, counts)| {
            let mut fact = Fact::new(COVERAGE.kind, id);
            if let Some(item) = counts.item {
                fact = fact.field("item", item);
            }
            fact = fact.field("file", counts.file);
            let families = [
                ("functions", counts.functions),
                ("lines", counts.lines),
                ("branches", counts.branches),
            ];
            for (family, found_hit) in families {
                if let Some((found, hit)) = found_hit {
                    fact = fact
                        .field(format!("{family}_found"), found)
                        .field(format!("{family}_hit"), hit);
                }
            }
            fact
        })
        .collect()
}
