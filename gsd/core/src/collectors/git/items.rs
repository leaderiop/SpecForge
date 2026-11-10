//! A recorded commit's `items_changed` (ADR 0352 D1): the lines its diff wrote or deleted, carried to the worktree by
//! one more diff, met with the spans of the `item` facts the store holds. GSD knows no language: the packs mint the
//! ids, and a fact's span says where it is written now.
//!
//! A line the commit wrote changes the item that holds its image in the worktree. A deletion is the gap between two
//! lines: it changes the item that holds both neighbours, so one at an item's edge does not. A line rewritten since
//! has no image and names nothing: on a first collect of an old history, an item whose every line was rewritten since
//! is not named. A file that holds items whose lines no answer of the port held (past one answer even alone, over its
//! blob cap, or withheld from the call) names nothing at all: the list is unknown, not short.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::Fact;
use specforge_extension_sdk::host::{GitDiffHead, GitDiffQuery, GitHunk};

use super::{Commit, in_repository, whole_diff};

/// The `item` facts with a place, by repository path: each one's lines (0-based, inclusive) and id.
pub(super) struct Items {
    by_path: BTreeMap<String, Vec<(usize, usize, String)>>,
}

/// What a commit's diff touched in one file of its version: a line it wrote, or the gap before a line where it deleted
/// some (0-based).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Touched {
    Line(usize),
    Gap(usize),
}

impl Items {
    /// The `item` facts of `facts`, their project-relative files under the project at `prefix` of the repository; none
    /// when no item fact is served (no `@code/core`, or nothing collected): `items_changed` is then left unwritten.
    pub(super) fn of(facts: &[Fact], prefix: &str) -> Option<Items> {
        let mut items = facts.iter().filter(|f| f.kind == "item").peekable();
        items.peek()?;
        let mut by_path: BTreeMap<String, Vec<(usize, usize, String)>> = BTreeMap::new();
        for fact in items {
            for at in &fact.at {
                // 1-based and inclusive on the wire; a line 0 is no place.
                if at.start_line == 0 || at.end_line < at.start_line {
                    continue;
                }
                by_path
                    .entry(in_repository(prefix, &at.file))
                    .or_default()
                    .push((at.start_line - 1, at.end_line - 1, fact.id.clone()));
            }
        }
        Some(Items { by_path })
    }

    /// The ids of the items `commit` changed, sorted, from its diff's changed ranges by repository path. One more
    /// diff, of the commit against the worktree over the files that hold items, carries its lines. `None` when a file
    /// that holds items has lines no answer of the port held, in either diff: the list would be short, and the field
    /// is written once.
    pub(super) fn changed(&self, commit: &Commit) -> Result<Option<Vec<String>>, String> {
        let (sha, hunks) = (&commit.hash, &commit.hunks);
        if commit
            .unread
            .iter()
            .any(|path| self.by_path.contains_key(path))
        {
            return Ok(None);
        }
        let paths: Vec<String> = hunks
            .keys()
            .filter(|path| self.by_path.contains_key(*path))
            .cloned()
            .collect();
        if paths.is_empty() {
            return Ok(Some(Vec::new()));
        }
        let since = whole_diff(
            &GitDiffQuery {
                base: Some(sha.to_string()),
                head: GitDiffHead::Worktree,
                paths: paths.clone(),
                patch: false,
                hunks: true,
                renames: false,
            },
            &format!("diff {} with the worktree", &sha[..12.min(sha.len())]),
        )?;
        // A file in the forward diff whose lines no answer held, binary aside, leaves its lines unknown.
        if since
            .iter()
            .any(|f| paths.contains(&f.path) && f.hunks.is_none() && !f.binary)
        {
            return Ok(None);
        }
        let mut ids = BTreeSet::new();
        for path in &paths {
            // A file absent from the forward diff is as the commit left it; one in it without hunks (binary) keeps no
            // line.
            let forward = since
                .iter()
                .find(|f| &f.path == path)
                .map(|f| f.hunks.as_deref());
            let image = |line: usize| match forward {
                None => Some(line),
                Some(None) => None,
                Some(Some(hunks)) => carried(line, hunks),
            };
            let items = &self.by_path[path];
            let holding = |lines: &[usize]| -> Vec<String> {
                items
                    .iter()
                    .filter(|(start, end, _)| lines.iter().all(|l| start <= l && l <= end))
                    .map(|(_, _, id)| id.clone())
                    .collect()
            };
            for touched in touched(&hunks[path]) {
                match touched {
                    Touched::Line(line) => {
                        if let Some(now) = image(line) {
                            ids.extend(holding(&[now]));
                        }
                    }
                    Touched::Gap(at) => {
                        let neighbours = at
                            .checked_sub(1)
                            .and_then(|before| Some([image(before)?, image(at)?]));
                        if let Some(lines) = neighbours {
                            ids.extend(holding(&lines));
                        }
                    }
                }
            }
        }
        Ok(Some(ids.into_iter().collect()))
    }
}

/// What `hunks` touched in the commit's version of a file: each line an after range holds, and a gap where an after
/// range is empty.
fn touched(hunks: &[GitHunk]) -> Vec<Touched> {
    hunks
        .iter()
        .flat_map(|h| {
            let (start, end) = (h.after_start as usize, h.after_end as usize);
            if start == end {
                vec![Touched::Gap(start)]
            } else {
                (start..end).map(Touched::Line).collect()
            }
        })
        .collect()
}

/// Where `line` of the older side is on the newer one through `hunks`, ordered by their older range: none when a hunk
/// rewrote or deleted it.
fn carried(line: usize, hunks: &[GitHunk]) -> Option<usize> {
    let mut shift: isize = 0;
    for h in hunks {
        let (before, after) = (
            h.before_start as usize..h.before_end as usize,
            h.after_start..h.after_end,
        );
        if line < before.start {
            break;
        }
        if before.contains(&line) {
            return None;
        }
        shift += after.len() as isize - before.len() as isize;
    }
    usize::try_from(line as isize + shift).ok()
}
