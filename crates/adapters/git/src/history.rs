//! The log `Gix` answers: which commits a log keeps and how a commit reads, and the bytes a bounded read may take;
//! the diff is in [`diff`].

use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::Path;

use gix::bstr::ByteSlice as _;
use specforge_effects::contained::withheld;
use specforge_effects::git::{
    BLOB_CAP, GitCommit, GitError, GitLog, GitPerson, LogQuery, PickaxeReads, Sha, is_binary,
};

use crate::{failed, id};

mod diff;

pub(crate) use diff::{file_diff, index_tree, tree_of};

/// The commits `query` keeps, newest first, or the oldest ones with `query.oldest`.
pub(crate) fn log(repo: &gix::Repository, query: &LogQuery) -> Result<GitLog, GitError> {
    use gix::revision::walk::Sorting;
    use gix::traverse::commit::simple::CommitTimeOrder;
    let tips = query.tips.iter().map(id).collect::<Result<Vec<_>, _>>()?;
    let hidden = query.hidden.iter().map(id).collect::<Result<Vec<_>, _>>()?;
    let mut walk = repo
        .rev_walk(tips)
        .with_hidden(hidden)
        .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst));
    if query.first_parent {
        walk = walk.first_parent_only();
    }
    let walk = walk.all().map_err(|e| failed("walk the history", &e))?;
    let paths = wanted_paths(&query.paths);
    let filter = Filter::of(query)?;
    let mut log = GitLog::default();
    let mut count = 0;
    // with `oldest`, the last `max` kept, newest first: only their ids until the walk ends
    let mut oldest = std::collections::VecDeque::new();
    // the work done so far: commits stepped onto, and bytes read (commit objects and the blobs a pickaxe searches)
    let mut walked = 0;
    let read = Reads::up_to(query.budget.map(|budget| budget.bytes));
    for info in walk {
        if let Some(budget) = query.budget {
            if walked >= budget.commits || read.spent() {
                log.stopped = true;
                break;
            }
            walked += 1;
        }
        let info = info.map_err(|e| failed("walk the history", &e))?;
        let listed = query.oldest || query.max.is_none_or(|max| log.commits.len() < max);
        if paths.is_empty() && filter.is_none() && !listed {
            // past the listing, a commit is only counted, as `rev-list --count` counts it
            count += 1;
            continue;
        }
        let commit = repo
            .find_commit(info.id)
            .map_err(|e| failed(&format!("read {}", info.id), &e))?;
        read.take(commit.data.len() as u64);
        if !keeps(filter.as_ref(), repo, &commit, &paths, &read)? {
            continue;
        }
        if !paths.is_empty() && !touches(repo, &commit, &paths, query.first_parent)? {
            continue;
        }
        if !listed && filter.as_ref().is_some_and(|f| f.pickaxe.is_some()) {
            // counting the rest would read every blob of the history: one more kept commit says there is more
            return Ok(log);
        }
        count += 1;
        if query.oldest {
            oldest.push_back(info.id);
            if query.max.is_some_and(|max| oldest.len() > max) {
                oldest.pop_front();
            }
        } else if listed {
            log.commits.push(commit_of(&commit)?);
        }
    }
    for kept in oldest {
        let commit = repo
            .find_commit(kept)
            .map_err(|e| failed(&format!("read {kept}"), &e))?;
        log.commits.push(commit_of(&commit)?);
    }
    // a walk that stopped did not count the rest
    log.count = (!log.stopped).then_some(count);
    Ok(log)
}

/// The bytes a log or a diff read so far, against the most it may (none: no bound). Counted before each read, so the
/// work past the bound is one object at most.
pub(crate) struct Reads {
    read: Cell<u64>,
    limit: Option<u64>,
    /// Nothing was taken yet: the first read is always allowed, so one file's diff always reads it.
    first: Cell<bool>,
}

impl Reads {
    pub(crate) fn up_to(limit: Option<u64>) -> Self {
        Reads {
            read: Cell::new(0),
            limit,
            first: Cell::new(true),
        }
    }

    /// Whether `bytes` more may be read, counting them when they may: the first always may, and once one may not, none
    /// may after it.
    pub(crate) fn take(&self, bytes: u64) -> bool {
        let first = self.first.replace(false);
        let total = self.read.get().saturating_add(bytes);
        if let Some(limit) = self.limit
            && !first
            && total > limit
        {
            self.read.set(u64::MAX);
            return false;
        }
        self.read.set(total);
        true
    }

    /// [`Self::take`] without the first read's pass: whether `bytes` more fit what the bound has left, counting them
    /// when they do; once they do not, nothing may be read after them.
    pub(crate) fn take_within(&self, bytes: u64) -> bool {
        if self
            .limit
            .is_some_and(|limit| self.read.get().saturating_add(bytes) > limit)
        {
            self.read.set(u64::MAX);
            return false;
        }
        self.take(bytes)
    }

    /// Whether there is a bound at all.
    pub(crate) fn bounded(&self) -> bool {
        self.limit.is_some()
    }

    /// Whether the bound is reached: nothing more is read.
    pub(crate) fn spent(&self) -> bool {
        self.limit.is_some_and(|limit| self.read.get() >= limit)
    }
}

/// The most bytes a compiled `--grep` or `-S` pattern may take: a larger one is refused, never built.
const PATTERN_CAP: usize = 1 << 20;

/// A typed pattern, compiled with `^` and `$` at each line as git's regex matching has them.
fn pattern(what: &str, source: &str) -> Result<regex::bytes::Regex, GitError> {
    regex::bytes::RegexBuilder::new(source)
        .multi_line(true)
        .size_limit(PATTERN_CAP)
        .build()
        .map_err(|e| GitError(format!("the {what} pattern {source:?} is no regex: {e}")))
}

/// What a log keeps of each commit besides its paths: `--grep`, `--since`, `--until`, `--no-merges` and `-S`.
pub(crate) struct Filter {
    grep: Option<regex::bytes::Regex>,
    since: Option<i64>,
    until: Option<i64>,
    no_merges: bool,
    pickaxe: Option<regex::bytes::Regex>,
    pickaxe_reads: PickaxeReads,
}

impl Filter {
    /// The filter `query` asks; `None` when it keeps every commit.
    pub(crate) fn of(query: &specforge_effects::git::LogQuery) -> Result<Option<Filter>, GitError> {
        let filter = Filter {
            grep: query
                .grep
                .as_deref()
                .map(|p| pattern("--grep", p))
                .transpose()?,
            since: query.since,
            until: query.until,
            no_merges: query.no_merges,
            pickaxe: query
                .pickaxe
                .as_deref()
                .map(|p| pattern("-S", p))
                .transpose()?,
            pickaxe_reads: query.pickaxe_reads,
        };
        let none = filter.grep.is_none()
            && filter.since.is_none()
            && filter.until.is_none()
            && !filter.no_merges
            && filter.pickaxe.is_none();
        Ok((!none).then_some(filter))
    }
}

/// Whether `filter` keeps `commit` (no filter keeps every one); its paths are the log's.
pub(crate) fn keeps(
    filter: Option<&Filter>,
    repo: &gix::Repository,
    commit: &gix::Commit<'_>,
    paths: &[String],
    read: &Reads,
) -> Result<bool, GitError> {
    let Some(filter) = filter else {
        return Ok(true);
    };
    let at = |e: &dyn std::fmt::Display| failed(&format!("read {}", commit.id), e);
    let parents = commit.parent_ids().count();
    if filter.no_merges && parents > 1 {
        return Ok(false);
    }
    if filter.since.is_some() || filter.until.is_some() {
        let time = commit
            .committer()
            .map_err(|e| at(&e))?
            .time()
            .map_err(|e| at(&e))?
            .seconds;
        if filter.since.is_some_and(|since| time < since)
            || filter.until.is_some_and(|until| time > until)
        {
            return Ok(false);
        }
    }
    if let Some(grep) = &filter.grep {
        let message = commit.message_raw().map_err(|e| at(&e))?;
        if !grep.is_match(message) {
            return Ok(false);
        }
    }
    match &filter.pickaxe {
        Some(pickaxe) => picked(
            repo,
            commit,
            pickaxe,
            paths,
            parents,
            filter.pickaxe_reads,
            read,
        ),
        None => Ok(true),
    }
}

/// How many times `rx` matches the blob `id` names, its bytes taken from `read`; none for a binary blob, one over
/// [`BLOB_CAP`], which `-S` does not read, or one past what `read` has left.
fn matches(
    repo: &gix::Repository,
    id: Option<gix::ObjectId>,
    rx: &regex::bytes::Regex,
    read: &Reads,
) -> Option<usize> {
    let Some(id) = id else {
        return Some(0);
    };
    let header = repo.find_header(id).ok()?;
    if header.kind() != gix::objs::Kind::Blob || header.size() > BLOB_CAP {
        return None;
    }
    if !read.take(header.size()) {
        return None;
    }
    let blob = repo.find_blob(id).ok()?;
    (!is_binary(&blob.data)).then(|| rx.find_iter(&blob.data).count())
}

/// Whether `commit` changes how many times `rx` matches in a file at `paths` against its one parent, or against
/// nothing for a root commit (`-S`): a merge, whose diff git does not show, never does.
fn picked(
    repo: &gix::Repository,
    commit: &gix::Commit<'_>,
    rx: &regex::bytes::Regex,
    paths: &[String],
    parents: usize,
    reads: PickaxeReads,
    read: &Reads,
) -> Result<bool, GitError> {
    use gix::object::tree::diff::ChangeDetached as Change;
    if parents > 1 {
        return Ok(false);
    }
    let at = |e: &dyn std::fmt::Display| failed(&format!("read {}", commit.id), e);
    let new = commit.tree().map_err(|e| at(&e))?;
    let old = match commit.parent_ids().next() {
        Some(parent) => repo
            .find_commit(parent)
            .map_err(|e| at(&e))?
            .tree()
            .map_err(|e| at(&e))?,
        None => repo.empty_tree(),
    };
    let changes = repo
        .diff_tree_to_tree(&old, &new, None)
        .map_err(|e| failed("diff two trees", &e))?;
    for change in changes {
        let (location, source, old, new) = match change {
            Change::Addition {
                location,
                entry_mode,
                id,
                ..
            } => (location, None, None, entry_mode.is_blob().then_some(id)),
            Change::Deletion {
                location,
                entry_mode,
                id,
                ..
            } => (location, None, entry_mode.is_blob().then_some(id), None),
            Change::Modification {
                location,
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
            } => (
                location,
                None,
                previous_entry_mode.is_blob().then_some(previous_id),
                entry_mode.is_blob().then_some(id),
            ),
            Change::Rewrite {
                location,
                source_location,
                source_entry_mode,
                source_id,
                entry_mode,
                id,
                ..
            } => (
                location,
                Some(source_location),
                source_entry_mode.is_blob().then_some(source_id),
                entry_mode.is_blob().then_some(id),
            ),
        };
        if old.is_none() && new.is_none() || !kept(&location.to_str_lossy(), paths) {
            continue;
        }
        let hidden = |path: &gix::bstr::BStr| withheld(&path.to_str_lossy());
        if reads == PickaxeReads::NoWithheld
            && (hidden(location.as_ref()) || source.as_ref().is_some_and(|s| hidden(s.as_ref())))
        {
            continue;
        }
        if let (Some(before), Some(after)) =
            (matches(repo, old, rx, read), matches(repo, new, rx, read))
            && before != after
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The paths a log or a diff keeps, slash-separated with no trailing slash; empty when one of them is the top.
pub(crate) fn wanted_paths(paths: &[String]) -> Vec<String> {
    let paths: Vec<String> = paths
        .iter()
        .map(|p| p.replace('\\', "/").trim_matches('/').to_string())
        .collect();
    if paths.iter().any(|p| p.is_empty() || p == ".") {
        return Vec::new();
    }
    paths
}

/// Whether `location` is at or under one of `paths` (every location when there are none), as git's pathspecs match
/// it.
// shortcut: the paths are literal pathspecs, no glob or magic, since `touches` looks each one up in a tree; upgrade
// both together if a block needs `git log -- '*.rs'`
fn kept(location: &str, paths: &[String]) -> bool {
    if paths.is_empty() {
        return true;
    }
    let literal = gix::pathspec::Defaults {
        signature: gix::pathspec::MagicSignature::empty(),
        search_mode: gix::pathspec::SearchMode::Literal,
        literal: true,
    };
    let specs = paths
        .iter()
        .filter_map(|path| gix::pathspec::parse(path.as_bytes(), literal).ok());
    let Ok(mut search) = gix::pathspec::Search::from_specs(specs, None, Path::new("")) else {
        return false;
    };
    search
        .pattern_matching_relative_path(location.into(), None, &mut |_, _, _, _| false)
        .is_some_and(|found| !found.is_excluded())
}

/// What `tree` holds at each of `paths`.
fn entries_at(
    tree: &gix::Tree<'_>,
    paths: &[String],
) -> Vec<Option<(gix::objs::tree::EntryMode, gix::ObjectId)>> {
    paths
        .iter()
        .map(|path| {
            let entry = tree.lookup_entry_by_path(path).ok()??;
            Some((entry.mode(), entry.object_id()))
        })
        .collect()
}

/// Whether `commit` changes something at `paths`. A merge counts when it differs from every parent it follows, as
/// git's default history simplification shows it.
// shortcut: git also prunes the parents a merge is the same as from the walk; upgrade if a block needs that order
fn touches(
    repo: &gix::Repository,
    commit: &gix::Commit<'_>,
    paths: &[String],
    first_parent: bool,
) -> Result<bool, GitError> {
    let tree = commit.tree().map_err(|e| failed("read a tree", &e))?;
    let mine = entries_at(&tree, paths);
    let parents: Vec<gix::ObjectId> = commit.parent_ids().map(|id| id.detach()).collect();
    let parents = if first_parent {
        &parents[..parents.len().min(1)]
    } else {
        &parents[..]
    };
    if parents.is_empty() {
        return Ok(mine.iter().any(Option::is_some));
    }
    for parent in parents {
        let read = |e: &dyn std::fmt::Display| failed(&format!("read {parent}"), e);
        let theirs = repo
            .find_commit(*parent)
            .map_err(|e| read(&e))?
            .tree()
            .map_err(|e| read(&e))?;
        if entries_at(&theirs, paths) == mine {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A signature as a log prints it.
fn person(signature: gix::actor::SignatureRef<'_>) -> GitPerson {
    let time = signature.time().ok();
    GitPerson {
        name: signature.name.to_str_lossy().trim().to_string(),
        email: signature.email.to_str_lossy().trim().to_string(),
        time: time
            .map(|t| t.format_or_unix(gix::date::time::format::ISO8601_STRICT))
            .unwrap_or_default(),
        seconds: time.map_or(0, |t| t.seconds),
    }
}

/// A commit as a log answers it.
fn commit_of(commit: &gix::Commit<'_>) -> Result<GitCommit, GitError> {
    let at = |e: &dyn std::fmt::Display| failed(&format!("read {}", commit.id), e);
    let message = commit.message().map_err(|e| at(&e))?;
    let mut trailers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Some(body) = message.body() {
        for trailer in body.trailers() {
            trailers
                .entry(trailer.token.to_str_lossy().into_owned())
                .or_default()
                .push(trailer.value.to_str_lossy().into_owned());
        }
    }
    Ok(GitCommit {
        sha: Sha::new(commit.id.to_string()),
        parents: commit
            .parent_ids()
            .map(|id| Sha::new(id.to_string()))
            .collect(),
        author: person(commit.author().map_err(|e| at(&e))?),
        committer: person(commit.committer().map_err(|e| at(&e))?),
        subject: message.summary().to_str_lossy().into_owned(),
        body: message
            .body
            .map(|b| b.to_str_lossy().trim().to_string())
            .unwrap_or_default(),
        trailers,
    })
}

#[cfg(test)]
mod tests {
    use super::kept;

    #[test]
    fn a_path_keeps_itself_and_what_is_under_it_literally() {
        let paths = ["src".to_string(), "a.txt".to_string(), "*.rs".to_string()];
        for location in ["src", "src/lib/x.rs", "a.txt", "*.rs"] {
            assert!(kept(location, &paths), "{location}");
        }
        for location in ["srcx/a.rs", "a.txt2", "b.rs", "SRC/a.rs"] {
            assert!(!kept(location, &paths), "{location}");
        }
        assert!(kept("anything", &[]));
    }
}
