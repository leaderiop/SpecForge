//! `gsd-git`: records, in the plan's own file, every commit that names a task by a `Gsd-Task: <plan>/<task>` trailer
//! (ADR 0331 D3), or a plan by GSD's subject convention `{type}({phase}-{plan}): {task}`
//! (`gsd-core/references/git-integration.md:72`).
//!
//! It is a writing collector: the host runs `git log` with the user's consent and hands the captured output to
//! `collect`, with the graph. The answer is an edit plan that creates one `commit` entity per new commit, in the file
//! of the plan it names. It records nothing it already holds (the 12-hex name is the key), so a second run is a no-op.
//!
//! The trailer comes first: one that names a task (`01-01/T1`, the configured `project_code` prefix stripped from the
//! plan as from a scope) fixes the plan and the task, whatever the subject says. Otherwise the subject decides: its
//! scope names the plan, and its description after `: ` the task whose title it equals. `linked_by` says which found
//! the task. A trailer that is unreadable (two different values, or one that is no `<plan>/<task>`) or names no task
//! falls back to the subject, and the plan's title notes it. A commit that names no plan either way (`feat(gsd): x`,
//! `docs(pause): y`, a scope no plan has, or one that several plans have) is skipped, as `/gsd-undo` skips it
//! (`scope === id`). GSD's executor writes no trailer yet (S1 adds it to `gsd-executor.md`'s commit step).
//!
//! Each recorded commit also names the recorded commits of its plan it descends from (`descends_from`), through every
//! parent: the log is read with merges, so a RED made on a branch merged before the GREEN counts (#5275). A merge
//! itself is not recorded.
//!
//! Everything taken from `git log` is untrusted. Records start at a NUL (`%x00`), which no commit message can hold, so
//! a subject cannot forge one; control characters in a subject or a path become U+FFFD; a record whose hash is not
//! hex or whose time is out of range is skipped and named in the plan's title. The host refuses a whole plan for one
//! value it cannot write, so one bad commit must never reach it.
//!
//! Also ports gsd-core bin/lib modules: git-base-branch.cjs.

mod subject;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::graph::find_kind;
use crate::time::utc;

const NAME: &str = "gsd-git";

/// One record separator (`%x00`), the unit separator between the fields of the header line (`%x1f`), and the
/// separator between the values of the trailer field (`%x1e`).
const RECORD: char = '\0';
const FIELD: char = '\u{1f}';
const VALUES: char = '\u{1e}';

/// The header line: the hash, the parents, the author time, the `Gsd-Task:` trailers (unfolded, so a value never
/// starts a line of its own) and the subject, last, so a unit separator in a subject stays in it.
const FORMAT: &str = "--format=%x00%H%x1f%P%x1f%at%x1f%(trailers:key=Gsd-Task,valueonly,unfold,separator=%x1e)%x1f%s";

/// `commit.linked_by`: what found a commit's task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinkedBy {
    /// Its `Gsd-Task:` trailer, written as the trailer's own key (owner decision 04-U1).
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
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.collector(NAME, |k| {
        k.input_format("git-log")
            .run(&[
                "git",
                "-c",
                "core.quotepath=off",
                "-c",
                "i18n.logOutputEncoding=UTF-8",
                "log",
                "--no-renames",
                "--numstat",
                "--summary",
                FORMAT,
            ])
            .capture_stdout()
            .writes()
            .collect(collect);
    });
}

/// One commit of the log.
#[derive(Debug, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    /// `%P`: none for a root commit, two or more for a merge.
    pub parents: Vec<String>,
    pub at: String,
    /// The `Gsd-Task:` trailer values as git printed them, joined by `%x1e`; read by [`trailer`].
    pub trailers: String,
    pub subject: String,
    pub files: Vec<String>,
    pub created: Vec<String>,
    pub insertions: i64,
    pub deletions: i64,
}

/// A path as `git log` prints it: C-quoted (`"a\tb"`, octal escapes for bytes) when it holds a tab, a quote or a
/// backslash.
fn unquote(path: &str) -> String {
    let Some(inner) = path.strip_prefix('"').and_then(|p| p.strip_suffix('"')) else {
        return clean(path);
    };
    let mut bytes = Vec::new();
    let mut chars = inner.bytes().peekable();
    while let Some(b) = chars.next() {
        if b != b'\\' {
            bytes.push(b);
            continue;
        }
        match chars.next() {
            Some(b'n') => bytes.push(b'\n'),
            Some(b't') => bytes.push(b'\t'),
            Some(b'r') => bytes.push(b'\r'),
            Some(b'a') => bytes.push(0x07),
            Some(b'b') => bytes.push(0x08),
            Some(b'f') => bytes.push(0x0c),
            Some(b'v') => bytes.push(0x0b),
            Some(digit @ b'0'..=b'7') => {
                let mut value = u32::from(digit - b'0');
                for _ in 0..2 {
                    match chars.peek() {
                        Some(next @ b'0'..=b'7') => {
                            value = value * 8 + u32::from(next - b'0');
                            chars.next();
                        }
                        _ => break,
                    }
                }
                bytes.push(u8::try_from(value & 0xff).unwrap_or(b'?'));
            }
            Some(other) => bytes.push(other),
            None => bytes.push(b'\\'),
        }
    }
    clean(&String::from_utf8_lossy(&bytes))
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

/// A full object name: 40 (SHA-1) or 64 (SHA-256) lower-case hex digits.
fn is_hash(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The commits of `git log --numstat --summary` with [`FORMAT`], newest first as git prints them,
/// and the hash (as far as it is printable) of each record that is not a valid commit: a hash or a parent that is not
/// hex, a time that is not in `0..=253402300799` (the years 1970 to 9999). A binary file counts 0 lines.
pub fn parse_all(log: &str) -> (Vec<Commit>, Vec<String>) {
    let (mut commits, mut skipped) = (Vec::new(), Vec::new());
    for text in log.split(RECORD).filter(|t| !t.trim().is_empty()) {
        match record(text) {
            Ok(commit) => commits.push(commit),
            Err(name) => skipped.push(name),
        }
    }
    (commits, skipped)
}

#[cfg(test)]
pub fn parse(log: &str) -> Vec<Commit> {
    parse_all(log).0
}

fn record(text: &str) -> Result<Commit, String> {
    let mut lines = text.lines();
    let mut head = lines.next().unwrap_or_default().splitn(5, FIELD);
    let (hash, parents, at, trailers, subject) = (
        head.next().unwrap_or_default().trim(),
        head.next().unwrap_or_default(),
        head.next().unwrap_or_default().trim(),
        head.next().unwrap_or_default(),
        head.next().unwrap_or_default(),
    );
    let parents: Vec<String> = parents.split_whitespace().map(String::from).collect();
    let time = at
        .parse::<i64>()
        .ok()
        .filter(|t| (0..=253_402_300_799).contains(t));
    let (true, true, Some(time)) = (is_hash(hash), parents.iter().all(|p| is_hash(p)), time) else {
        return Err(clean(&hash.chars().take(12).collect::<String>()));
    };
    let mut commit = Commit {
        hash: hash.to_string(),
        parents,
        at: utc(time),
        trailers: trailers.to_string(),
        subject: clean(subject),
        files: Vec::new(),
        created: Vec::new(),
        insertions: 0,
        deletions: 0,
    };
    for line in lines {
        if let Some(rest) = line.strip_prefix(" create mode ") {
            // `create mode 100644 path`: the mode, then the path (which may hold spaces).
            if let Some((_, path)) = rest.split_once(' ') {
                commit.created.push(unquote(path));
            }
            continue;
        }
        let mut columns = line.splitn(3, '\t');
        if let (Some(added), Some(deleted), Some(path)) =
            (columns.next(), columns.next(), columns.next())
        {
            commit.insertions += added.parse::<i64>().unwrap_or(0);
            commit.deletions += deleted.parse::<i64>().unwrap_or(0);
            commit.files.push(unquote(path));
        }
    }
    Ok(commit)
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

/// The trailer `values` (git's `%x1e`-separated list) say: each trimmed, empty ones dropped, repeats counted once.
pub fn trailer(values: &str) -> Trailer<'_> {
    let mut named: Vec<&str> = values
        .split(VALUES)
        .map(str::trim)
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
    let nodes = input.graph.as_ref().map_or(&[][..], |g| &g.nodes[..]);
    let mut known: BTreeSet<String> = nodes
        .iter()
        .filter(|n| n.kind == "commit")
        .map(|n| n.id.clone())
        .collect();
    let mut builder = PlanBuilder::for_collect(input);
    let mut recorded = 0;
    let mut unread = Vec::new();
    let (commits, mut skipped) = parse_all(input.stdout.as_deref().unwrap_or(""));
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
    let plans: HashMap<&str, &str> = commits
        .iter()
        .filter_map(|c| Some((c.hash.as_str(), link(nodes, code, c)?.plan.id.as_str())))
        .collect();
    let mut descends = ancestry(&commits, &plans);
    // Oldest first, so the entities land in the order the commits were made.
    for commit in commits.iter().rev() {
        let name: String = commit.hash.chars().take(12).collect();
        if known.contains(&name) {
            continue;
        }
        let Some(linked) = link(nodes, code, commit) else {
            continue;
        };
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
        known.insert(name);
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
