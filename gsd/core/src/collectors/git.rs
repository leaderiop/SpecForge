//! `gsd-git`: records, in the plan's own file, every commit that names a task by a `Gsd-Task: <plan>/<task>` trailer
//! (ADR 0331 D3), or a plan by GSD's subject convention `{type}({phase}-{plan}): {task}`
//! (`gsd-core/references/git-integration.md:72`).
//!
//! It is a writing collector that reads the project's repository through the host's git port (ADR 0352): one typed
//! log from `HEAD`, and one diff for each commit it records. It runs no `git` program and needs no report, so a
//! `--no-run` collect records commits too; an unborn repository records nothing. The answer is an edit plan that
//! creates one `commit` entity per new commit, in the file of the plan it names. It records nothing it already holds
//! (the 12-hex name is the key), so a second run is a no-op.
//!
//! The trailer comes first: one that names a task (`01-01/T1`, the configured `project_code` prefix stripped from the
//! plan as from a scope) fixes the plan and the task, whatever the subject says. Otherwise the subject decides: its
//! scope names the plan, and its description after `: ` the task whose title it equals. `linked_by` says which found
//! the task. A trailer that is unreadable (two different values, or one that is no `<plan>/<task>`) or names no task
//! falls back to the subject, and the plan's title notes it. A commit that names no plan either way (`feat(gsd): x`,
//! `docs(pause): y`, a scope no plan has, or one that several plans have) is skipped, as `/gsd-undo` skips it
//! (`scope === id`). GSD's executor writes the trailer (`gsd-executor.md`'s commit step, ADR 0351).
//!
//! Each recorded commit also names the recorded commits of its plan it descends from (`descends_from`), through every
//! parent: the log is read with merges, so a RED made on a branch merged before the GREEN counts (#5275). A merge
//! itself is not recorded.
//!
//! A commit's `files` and `created` are project-relative: a path outside the project is left out, its lines still
//! counted. The diff lists a rename as a deletion and an addition (`--no-renames`), so both paths are touched.
//!
//! What a commit says is untrusted: control characters in a subject or a path become U+FFFD, and a commit whose time
//! is out of range is skipped and named in the plan's title. The host refuses a whole plan for one value it cannot
//! write, so one bad commit must never reach it.
//!
//! Also ports gsd-core bin/lib modules: git-base-branch.cjs.

mod subject;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{Value, json};
use specforge_extension_sdk::host::{
    Denial, GitChange, GitDiffHead, GitDiffQuery, GitLogCommit, GitLogQuery, HostError, git,
};
use specforge_extension_sdk::prelude::*;

use super::graph::find_kind;
use crate::time::utc;

const NAME: &str = "gsd-git";

/// The trailer token that names a task, matched in any case as git matches a trailer's token.
const TASK_TRAILER: &str = "Gsd-Task";

/// `commit.linked_by`: what found a commit's task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinkedBy {
    /// Its `Gsd-Task:` trailer, written as the trailer's own key.
    Trailer,
    /// Its subject's title.
    Title,
}

impl LinkedBy {
    /// The value `commit.linked_by` holds.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            LinkedBy::Trailer => "Gsd-Task",
            LinkedBy::Title => "title",
        }
    }

    /// The variant whose value `commit.linked_by` holds; `None` for any other text.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        [LinkedBy::Trailer, LinkedBy::Title]
            .into_iter()
            .find(|by| by.as_str() == text)
    }
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.collector(NAME, |k| {
        // A branch ref: a repository with a commit.
        k.detect_files(&[".git/refs/heads/*", ".git/packed-refs"])
            .writes()
            .collect(collect);
    });
}

/// One commit of the log; `files` and on are its diff's, filled when it is recorded.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    /// None for a root commit, two or more for a merge.
    pub parents: Vec<String>,
    pub at: String,
    /// The values of its `Gsd-Task:` trailers, read by [`trailer`].
    pub trailers: Vec<String>,
    pub subject: String,
    pub files: Vec<String>,
    pub created: Vec<String>,
    pub insertions: i64,
    pub deletions: i64,
}

impl Commit {
    /// `commit` as the log answers it, or its name when its author time is not in `0..=253402300799` (the years 1970
    /// to 9999).
    fn of(commit: &GitLogCommit) -> Result<Commit, String> {
        let seconds = commit.author.seconds;
        if !(0..=253_402_300_799).contains(&seconds) {
            return Err(commit.sha.chars().take(12).collect());
        }
        Ok(Commit {
            hash: commit.sha.clone(),
            parents: commit.parents.clone(),
            at: utc(seconds),
            trailers: commit
                .trailers
                .iter()
                .filter(|t| t.token.eq_ignore_ascii_case(TASK_TRAILER))
                .flat_map(|t| t.values.iter().cloned())
                .collect(),
            subject: clean(&commit.subject),
            ..Commit::default()
        })
    }

    /// Its changes against its first parent (every file of a root commit), renames apart: the files under the project
    /// at `prefix` of the repository, project-relative and sorted, the ones it added, and the lines of every file.
    fn changes(&mut self, prefix: &str) -> Result<(), String> {
        let diff = git::diff(&GitDiffQuery {
            base: self.parents.first().cloned(),
            head: GitDiffHead::Commit(self.hash.clone()),
            paths: Vec::new(),
            patch: false,
            hunks: false,
            renames: false,
        })
        .map_err(|e| {
            said(
                &format!("diff {}", &self.hash[..12.min(self.hash.len())]),
                &e,
            )
        })?;
        for file in &diff.files {
            self.insertions += i64::from(file.added);
            self.deletions += i64::from(file.removed);
            let Some(path) = in_project(prefix, &file.path) else {
                continue;
            };
            if file.status == GitChange::Added {
                self.created.push(path.clone());
            }
            self.files.push(path);
        }
        self.files.sort();
        self.created.sort();
        Ok(())
    }
}

/// What a port's refusal or failure says.
fn said(what: &str, error: &HostError) -> String {
    let why = match error {
        HostError::Denied(Denial::CallKind(why) | Denial::Scope(why)) => why,
        HostError::Failed(failure) => &failure.message,
    };
    format!("{NAME} could not {what}: {why}")
}

/// The project's directory from the repository's top (`sub/dir`, empty at the top): the project root the host lends
/// less the repository's top, both canonical.
fn prefix(cwd: Option<&str>, toplevel: &str) -> String {
    let posix = |p: &str| p.replace('\\', "/");
    let (cwd, top) = (posix(cwd.unwrap_or_default()), posix(toplevel));
    match cwd.strip_prefix(top.trim_end_matches('/')) {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => {
            rest.trim_matches('/').to_string()
        }
        _ => String::new(),
    }
}

/// `path` (from the repository's top) from the project at `prefix`, cleaned; none when it is outside the project.
fn in_project(prefix: &str, path: &str) -> Option<String> {
    let rest = if prefix.is_empty() {
        path
    } else {
        path.strip_prefix(prefix)?.strip_prefix('/')?
    };
    Some(clean(rest))
}

/// `text` with every control character (ANSI escapes, DEL, U+0085) and U+2028/2029 replaced by U+FFFD: the host
/// cannot write them back unchanged. A tab and a newline survive (the writer quotes them).
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| {
            if (c.is_control() && c != '\t' && c != '\n') || matches!(c, '\u{2028}' | '\u{2029}') {
                '\u{fffd}'
            } else {
                c
            }
        })
        .collect()
}

/// What a commit's `Gsd-Task:` trailers say.
#[derive(Debug, PartialEq, Eq)]
pub enum Trailer<'a> {
    /// There is none.
    None,
    /// One distinct `<plan>/<task>` value, as written (`01-01/T1`).
    Named(&'a str),
    /// Two different values, or one that is no `<plan>/<task>`: no task, as upstream counts a doubled `gate-status:`
    /// as missing (`ship.md:335`).
    Unreadable,
}

impl Trailer<'_> {
    /// What the plan's title says of a trailer that names no task: its value (cleaned, cut at 60 characters), or
    /// `unreadable`; `None` when there is no trailer.
    fn shown(&self) -> Option<String> {
        match self {
            Trailer::None => None,
            Trailer::Named(value) => Some(clean(value).chars().take(60).collect()),
            Trailer::Unreadable => Some("unreadable".to_string()),
        }
    }
}

/// `id` without the configured `code` prefix and its `-` (`APP-01-01` → `01-01`); `None` when there is no code or `id`
/// does not start with it. Only the configured code is stripped: an id that only looks coded is matched literally
/// (#5271).
fn uncoded<'v>(id: &'v str, code: Option<&str>) -> Option<&'v str> {
    id.strip_prefix(code?)?.strip_prefix('-')
}

/// The trailer `values` say: each trimmed, empty ones dropped, repeats counted once.
pub fn trailer(values: &[String]) -> Trailer<'_> {
    let mut named: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();
    named.sort_unstable();
    named.dedup();
    // Each side of the one `/`: ASCII letters, digits, `.`, `_` and `-` (no space, tab, separator or control).
    let part = |p: &str| {
        !p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    };
    match named[..] {
        [] => Trailer::None,
        [one]
            if one
                .split_once('/')
                .is_some_and(|(plan, task)| part(plan) && part(task)) =>
        {
            Trailer::Named(one)
        }
        _ => Trailer::Unreadable,
    }
}

/// The task a trailer value names (`01-01/T1`), as written or with the configured `code` prefix stripped from its plan
/// (`APP-01-01/T1`), as [`plan_by_scope`] strips it from a scope.
fn task_named<'n>(
    nodes: &'n [GraphNode],
    code: Option<&str>,
    value: &str,
) -> Option<&'n GraphNode> {
    find_kind(nodes, value, "task").or_else(|| {
        let (plan, task) = value.split_once('/')?;
        find_kind(nodes, &format!("{}/{task}", uncoded(plan, code)?), "task")
    })
}

/// `title` compared as GSD compares a task description: trimmed, case-folded, whitespace runs collapsed.
fn folded(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// A task description as the executor words it (`gsd-executor.md:565`, `git-integration.md`): without a squash-merge's
/// ` (#12)`, and without the TDD verbs `add failing test for ` and `implement `.
fn described(description: &str) -> String {
    let mut text = folded(description);
    if let Some((head, tail)) = text.rsplit_once(" (#")
        && tail
            .strip_suffix(')')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    {
        text = head.to_string();
    }
    for verb in ["add failing test for ", "implement "] {
        if let Some(rest) = text.strip_prefix(verb) {
            return rest.to_string();
        }
    }
    text
}

/// The one task of `plan` that `description` names: its title equals it, or the description holds the title (after
/// `described`). Two candidates name none.
fn task_of<'a>(nodes: &'a [GraphNode], plan: &str, description: &str) -> Option<&'a str> {
    let wanted = described(description);
    let mut titled = nodes.iter().filter(|n| {
        n.kind == "task"
            && n.id
                .strip_prefix(plan)
                .is_some_and(|rest| rest.starts_with('/') && !rest[1..].contains('/'))
            && n.title
                .as_deref()
                .map(folded)
                .is_some_and(|t| !t.is_empty() && (t == wanted || wanted.contains(&t)))
    });
    match (titled.next(), titled.next()) {
        (Some(one), None) => Some(one.id.as_str()),
        _ => None,
    }
}

#[cfg(test)]
thread_local! {
    /// The commits `ancestry` stepped onto on this thread: a test bounds the walk's work by it, not by the clock.
    pub(super) static WALKED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// For each commit of `plans`, the 12-hex names of the commits of its plan it reaches through its parents, oldest
/// first. Parents are resolved before their children, so a walk stops at a commit of its own plan and takes that
/// commit's list: each commit is walked only down to its plan's nearest recorded commits (#5275).
// shortcut: a plan's first commit still walks to the root, O(log × plans); carry every plan's nearest recorded commits
// in one topological pass when a log of many plans nears the host's 30 s guest limit (`execute.md`).
fn ancestry<'a>(
    commits: &'a [Commit],
    plans: &HashMap<&'a str, &'a str>,
) -> HashMap<&'a str, Vec<String>> {
    let count = commits.len();
    let index: HashMap<&str, usize> = commits
        .iter()
        .enumerate()
        .map(|(i, c)| (c.hash.as_str(), i))
        .collect();
    // By log position (newest is 0); a parent outside the log read is not walked.
    let parents: Vec<Vec<usize>> = commits
        .iter()
        .map(|c| {
            c.parents
                .iter()
                .filter_map(|p| index.get(p.as_str()).copied())
                .collect()
        })
        .collect();
    let plan: Vec<Option<&str>> = commits
        .iter()
        .map(|c| plans.get(c.hash.as_str()).copied())
        .collect();
    // git's own order is not topological under clock skew: an iterative post-order walk puts parents first.
    let (mut order, mut done) = (Vec::with_capacity(count), vec![false; count]);
    for start in (0..count).rev() {
        let mut stack = vec![(start, false)];
        while let Some((at, finished)) = stack.pop() {
            if finished {
                order.push(at);
            } else if !done[at] {
                done[at] = true;
                stack.push((at, true));
                stack.extend(
                    parents[at]
                        .iter()
                        .filter(|&&p| !done[p])
                        .map(|&p| (p, false)),
                );
            }
        }
    }
    // `seen[c]` is the walk that last reached `c`, so no set is cleared between walks.
    let (mut memo, mut seen) = (vec![Vec::new(); count], vec![usize::MAX; count]);
    for at in order {
        let Some(own) = plan[at] else {
            continue;
        };
        let mut found = BTreeSet::new();
        let mut next = parents[at].clone();
        while let Some(up) = next.pop() {
            if seen[up] == at {
                continue;
            }
            seen[up] = at;
            #[cfg(test)]
            WALKED.with(|n| n.set(n.get() + 1));
            if plan[up] == Some(own) {
                found.insert(up);
                found.extend(&memo[up]);
            } else {
                next.extend(&parents[up]);
            }
        }
        memo[at] = found.into_iter().collect::<Vec<usize>>();
    }
    (0..count)
        .filter(|&i| plan[i].is_some())
        .map(|i| {
            // Ascending positions are newest first.
            let names = memo[i]
                .iter()
                .rev()
                .map(|&a| commits[a].hash.chars().take(12).collect())
                .collect();
            (commits[i].hash.as_str(), names)
        })
        .collect()
}

/// The `commit` fields of `commit` as `linked` places it.
fn fields(
    commit: &Commit,
    linked: &Linked<'_, '_>,
    descends: &[String],
) -> BTreeMap<String, Value> {
    let mut fields = BTreeMap::new();
    fields.insert("hash".to_string(), json!(commit.hash));
    if let Some((task, by)) = linked.task {
        fields.insert("task".to_string(), json!(task));
        fields.insert("linked_by".to_string(), json!(by.as_str()));
    }
    fields.insert("plan".to_string(), json!(linked.plan.id));
    if let Some(kind) = &linked.kind {
        fields.insert("type".to_string(), json!(kind));
    }
    fields.insert("subject".to_string(), json!(commit.subject));
    fields.insert("at".to_string(), json!(commit.at));
    // Written even when empty: an absent list means ancestry was never collected (W403 then falls back to `at`).
    fields.insert("descends_from".to_string(), json!(descends));
    if !commit.files.is_empty() {
        fields.insert("files".to_string(), json!(commit.files));
    }
    if !commit.created.is_empty() {
        fields.insert("created".to_string(), json!(commit.created));
    }
    // JSON numbers: the host's plan op reads them for an integer field (ADR 0311).
    fields.insert("insertions".to_string(), json!(commit.insertions));
    fields.insert("deletions".to_string(), json!(commit.deletions));
    fields
}

/// Where a commit is recorded: its plan, its task and what found it, its type, and the trailer that was not read.
struct Linked<'n, 'c> {
    plan: &'n GraphNode,
    task: Option<(&'n str, LinkedBy)>,
    /// The subject's type, lower-cased; `revert` for a revert; `None` for a subject with no `type:` header.
    kind: Option<String>,
    /// The trailer, when it names no task and the subject decided; [`Trailer::None`] otherwise.
    unread: Trailer<'c>,
}

/// The plan the subject's `scope` names, as written or with the configured `code` prefix stripped.
fn plan_by_scope<'n>(
    nodes: &'n [GraphNode],
    code: Option<&str>,
    scope: &str,
) -> Option<&'n GraphNode> {
    find_kind(nodes, scope, "plan").or_else(|| find_kind(nodes, uncoded(scope, code)?, "plan"))
}

/// Where `commit` is (or would be) recorded: by its trailer's task, else by its subject's scope and title. A merge is
/// read for its parents only. `code` is the project's `project_code` (the setting, else the project entity's field).
fn link<'n, 'c>(
    nodes: &'n [GraphNode],
    code: Option<&str>,
    commit: &'c Commit,
) -> Option<Linked<'n, 'c>> {
    if commit.parents.len() >= 2 {
        return None;
    }
    let header = subject::parse(&commit.subject);
    // A revert undoes the work: it is recorded as one, and names no task.
    let reverts = commit.subject.starts_with("Revert \"");
    let kind = if reverts {
        Some("revert".to_string())
    } else {
        header.as_ref().map(|h| h.kind.clone())
    };
    let trailer = trailer(&commit.trailers);
    let named = match trailer {
        Trailer::Named(value) => task_named(nodes, code, value),
        Trailer::None | Trailer::Unreadable => None,
    };
    // The task's own `plan` reference, not its id's prefix.
    if let Some(task) = named
        && let Some(plan) = task
            .text("plan")
            .and_then(|plan| find_kind(nodes, plan, "plan"))
    {
        return plan.file.is_some().then(|| Linked {
            plan,
            task: (!reverts).then_some((task.id.as_str(), LinkedBy::Trailer)),
            kind,
            unread: Trailer::None,
        });
    }
    let header = header?;
    let plan = plan_by_scope(nodes, code, header.scope?)?;
    plan.file.as_ref()?;
    let task = (!reverts)
        .then(|| task_of(nodes, &plan.id, header.description))
        .flatten();
    Some(Linked {
        plan,
        task: task.map(|task| (task, LinkedBy::Title)),
        kind,
        unread: trailer,
    })
}

fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let Some(repo) = git::locate().map_err(|e| said("locate the repository", &e))? else {
        return Err(format!("{NAME}: the project is in no git repository"));
    };
    // An unborn repository: nothing to record.
    let Some(head) = git::commit("HEAD").map_err(|e| said("read HEAD", &e))? else {
        return Ok(CollectOutput::default());
    };
    let log = git::log(&GitLogQuery {
        tips: vec![head],
        ..GitLogQuery::default()
    })
    .map_err(|e| said("read the log", &e))?;
    if log.truncated {
        return Err(format!(
            "{NAME}: the history of {} commits is past one answer of the host's git port",
            log.count.unwrap_or_default()
        ));
    }
    let prefix = prefix(input.cwd.as_deref(), &repo.toplevel);
    let nodes = input.graph.as_ref().map_or(&[][..], |g| &g.nodes[..]);
    let mut known: BTreeSet<String> = nodes
        .iter()
        .filter(|n| n.kind == "commit")
        .map(|n| n.id.clone())
        .collect();
    let mut builder = PlanBuilder::for_collect(input);
    let mut recorded = 0;
    let mut unread = Vec::new();
    let (mut commits, mut skipped) = (Vec::new(), Vec::new());
    for commit in &log.commits {
        match Commit::of(commit) {
            Ok(commit) => commits.push(commit),
            Err(name) => skipped.push(name),
        }
    }
    // The `project_code` setting (what GSD users set and `specforge settings` shows), else the project entity's field.
    let settings = Settings::current();
    let code = settings
        .string("project_code")
        .filter(|c| !c.is_empty())
        .or_else(|| {
            nodes
                .iter()
                .find(|n| n.kind == "project")
                .and_then(|n| n.text("project_code"))
                .filter(|c| !c.is_empty())
        });
    // Oldest first, so the entities land in the order the commits were made; each new one asks its diff once.
    let mut fresh = Vec::new();
    for (at, commit) in commits.iter().enumerate().rev() {
        let name: String = commit.hash.chars().take(12).collect();
        if !known.contains(&name) && link(nodes, code, commit).is_some() {
            known.insert(name);
            fresh.push(at);
        }
    }
    for &at in &fresh {
        commits[at].changes(&prefix)?;
    }
    let plans: HashMap<&str, &str> = commits
        .iter()
        .filter_map(|c| Some((c.hash.as_str(), link(nodes, code, c)?.plan.id.as_str())))
        .collect();
    let mut descends = ancestry(&commits, &plans);
    for &at in &fresh {
        let commit = &commits[at];
        let Some(linked) = link(nodes, code, commit) else {
            continue;
        };
        let name: String = commit.hash.chars().take(12).collect();
        if let Some(value) = linked.unread.shown() {
            unread.push(format!("trailer of {name} names no task: {value}"));
        }
        let file = linked.plan.file.as_deref().unwrap_or_default();
        builder.create(
            "commit",
            &name,
            &commit.subject,
            file,
            fields(
                commit,
                &linked,
                &descends.remove(commit.hash.as_str()).unwrap_or_default(),
            ),
            Vec::new(),
        );
        recorded += 1;
    }
    skipped.dedup();
    let mut output = CollectOutput::default();
    if recorded > 0 {
        let mut note = if skipped.is_empty() {
            String::new()
        } else {
            format!(
                " (skipped {} with an unreadable header: {})",
                skipped.len(),
                skipped.join(", ")
            )
        };
        for why in &unread {
            note.push_str(&format!(" ({why})"));
        }
        builder.title(&format!(
            "Record {recorded} commit{}{note}",
            if recorded == 1 { "" } else { "s" }
        ));
        output.plan = Some(builder.finish());
    }
    Ok(output)
}
