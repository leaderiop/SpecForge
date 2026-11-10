use std::rc::Rc;

use super::subject::{Header, parse as header};
use specforge_extension_sdk::host::in_process::{self, InProcessHost};
use specforge_extension_sdk::host::{
    GitDiffAnswer, GitFileChange, GitLogAnswer, GitRepo, GitSignature, GitTrailer,
};
use specforge_extension_sdk::{GraphWire, PlanOp};

use super::*;

mod items;
mod port;
mod trailers;

fn head(kind: &str, scope: Option<&str>, description: &str) -> Option<Header<'static>> {
    // The strings are 'static literals in these tests.
    let (scope, description): (Option<&'static str>, &'static str) = (
        scope.map(|s| &*Box::leak(s.to_string().into_boxed_str())),
        Box::leak(description.to_string().into_boxed_str()),
    );
    Some(Header {
        kind: kind.to_string(),
        scope,
        description,
    })
}

#[test]
fn the_header_grammar_is_undos() {
    assert_eq!(
        header("feat(03-02): issue a token"),
        head("feat", Some("03-02"), "issue a token")
    );
    assert_eq!(header("feat!: drop v1"), head("feat", None, "drop v1"));
    assert_eq!(
        header("fix(03-02)!: rename"),
        head("fix", Some("03-02"), "rename")
    );
    assert_eq!(
        header("fixup! feat(03-02): x"),
        head("feat", Some("03-02"), "x")
    );
    assert_eq!(
        header("squash! test(03-02): y"),
        head("test", Some("03-02"), "y")
    );
    assert_eq!(
        header("Revert \"feat(03-02): x\""),
        head("feat", Some("03-02"), "x")
    );
    assert_eq!(
        header("FEAT(03.1-01):   spaced  "),
        head("feat", Some("03.1-01"), "spaced")
    );
    assert_eq!(
        header("docs(): empty scope").map(|h| h.scope),
        Some(Some(""))
    );
    // One layer only: a doubly wrapped subject stays unparseable, as upstream.
    assert_eq!(header("fixup! fixup! feat(03-02): x"), None);
    for none in [
        "baseline",
        "feat",
        "feat(03-02 x",
        "feat (03-02): x",
        "(03-02): x",
        "1: x",
        "",
    ] {
        assert_eq!(header(none), None, "{none:?}");
    }
}

#[test]
fn the_header_grammar_agrees_with_the_regex() {
    let upstream = regex::Regex::new(r"(?i)^([a-z]+)(\([^)]*\))?(!)?:").expect("a pattern");
    for subject in [
        "feat(03-02): x",
        "feat!: x",
        "fix(03-02)!: x",
        "Feat(a(b): x",
        "feat(a)b): x",
        "feat(: x",
        "feat)(: x",
        "feat(a)!!: x",
        "feat:x",
        "feat : x",
        "é: x",
        "docs(pause): y",
        "feat(a)(b): x",
        "feat(03-02)",
    ] {
        assert_eq!(
            upstream.is_match(subject),
            header(subject).is_some(),
            "{subject:?}"
        );
        if let (Some(m), Some(h)) = (upstream.captures(subject), header(subject)) {
            assert_eq!(
                m.get(2).map(|g| g.as_str().trim_matches(['(', ')'])),
                h.scope,
                "{subject:?}"
            );
        }
    }
}

/// A repository the in-process host serves: its top, `HEAD` (the newest commit of the log), the log, and each
/// commit's diff by its sha. It counts the questions asked.
#[derive(Default)]
pub(super) struct Repo {
    /// The project is in no repository.
    pub(super) outside: bool,
    pub(super) top: String,
    pub(super) log: Vec<GitLogCommit>,
    pub(super) diffs: HashMap<String, Vec<GitFileChange>>,
    /// Each commit's diff against the worktree, by the commit's sha.
    pub(super) worktree: HashMap<String, Vec<GitFileChange>>,
    pub(super) truncated: bool,
    /// How many paths a diff answer holds the text of: past them it drops each file's hunks and says `truncated`, as
    /// the host's port does past its answer cap. `None` holds every one.
    pub(super) room: Option<usize>,
    pub(super) logs: std::cell::Cell<usize>,
    pub(super) asked: std::cell::RefCell<Vec<GitDiffQuery>>,
}

impl InProcessHost for Repo {
    fn git_locate(&self) -> Result<Option<GitRepo>, HostError> {
        if self.outside {
            return Ok(None);
        }
        Ok(Some(GitRepo {
            toplevel: self.top.clone(),
            shallow: false,
        }))
    }
    fn git_commit(&self, revision: &str) -> Result<Option<String>, HostError> {
        assert_eq!(revision, "HEAD");
        Ok(self.log.first().map(|c| c.sha.clone()))
    }
    fn git_log(&self, query: &GitLogQuery) -> Result<GitLogAnswer, HostError> {
        self.logs.set(self.logs.get() + 1);
        assert_eq!(query.tips, [self.log[0].sha.clone()]);
        Ok(GitLogAnswer {
            count: Some(self.log.len() as u64),
            commits: self.log.clone(),
            truncated: self.truncated,
        })
    }
    fn git_diff(&self, query: &GitDiffQuery) -> Result<GitDiffAnswer, HostError> {
        self.asked.borrow_mut().push(query.clone());
        let files = match (&query.head, &query.base) {
            (GitDiffHead::Commit(sha), _) => self.diffs.get(sha),
            (GitDiffHead::Worktree, Some(base)) => self.worktree.get(base),
            _ => panic!("a commit's diff, or a commit's against the worktree: {query:?}"),
        };
        // A path keeps itself and what is under it, as the port's literal paths do.
        let kept = |path: &str| {
            query.paths.is_empty()
                || query.paths.iter().any(|p| {
                    path == p
                        || path
                            .strip_prefix(p.as_str())
                            .is_some_and(|r| r.starts_with('/'))
                })
        };
        let mut files: Vec<GitFileChange> = files
            .into_iter()
            .flatten()
            .filter(|f| kept(&f.path))
            .cloned()
            .collect();
        let mut truncated = false;
        for file in files.iter_mut().skip(self.room.unwrap_or(usize::MAX)) {
            truncated |= file.hunks.take().is_some();
            file.patch = None;
        }
        Ok(GitDiffAnswer { files, truncated })
    }
}

/// A commit of the log: `hash`, its `parents`, its author time `at`, its subject and its `Gsd-Task:` values.
pub(super) fn logged(
    hash: &str,
    parents: &[String],
    at: i64,
    subject: &str,
    task: &[&str],
) -> GitLogCommit {
    let signature = GitSignature {
        seconds: at,
        ..GitSignature::default()
    };
    GitLogCommit {
        sha: hash.to_string(),
        parents: parents.to_vec(),
        author: signature.clone(),
        committer: signature,
        subject: subject.to_string(),
        trailers: if task.is_empty() {
            Vec::new()
        } else {
            vec![GitTrailer {
                token: "Gsd-Task".into(),
                values: task.iter().map(|v| v.to_string()).collect(),
            }]
        },
        ..GitLogCommit::default()
    }
}

/// A changed file of a diff.
pub(super) fn changed(path: &str, status: GitChange, added: u32, removed: u32) -> GitFileChange {
    GitFileChange {
        path: path.into(),
        old_path: None,
        status,
        added,
        removed,
        binary: false,
        truncated: false,
        withheld: false,
        patch: None,
        hunks: None,
    }
}

/// What `collect` answers with `repo` served (its top `/r` when none is set).
pub(super) fn served(
    mut repo: Repo,
    input: &CollectInput,
) -> (Result<CollectOutput, String>, Rc<Repo>) {
    if repo.top.is_empty() {
        repo.top = "/r".into();
    }
    let repo = Rc::new(repo);
    let output = in_process::serve(Rc::clone(&repo) as Rc<dyn InProcessHost>, || collect(input));
    (output, repo)
}

/// What `collect` answers over `log`, no commit changing a file.
pub(super) fn collected_over(log: Vec<GitLogCommit>, input: CollectInput) -> CollectOutput {
    served(
        Repo {
            log,
            ..Repo::default()
        },
        &input,
    )
    .0
    .expect("an answer")
}

#[test]
fn an_empty_or_unborn_repository_is_no_plan() {
    let input = CollectInput {
        graph: Some(Default::default()),
        ..Default::default()
    };
    let (output, repo) = served(Repo::default(), &input);
    let output = output.expect("an answer");
    assert!(output.plan.is_none() && output.entity_results.is_empty());
    assert_eq!(repo.logs.get(), 0, "no log is read without a HEAD");
}

#[test]
fn a_history_past_one_answer_fails_the_collect() {
    let repo = Repo {
        log: vec![logged(&"a".repeat(40), &[], 10, "feat(01-01): x", &[])],
        truncated: true,
        ..Repo::default()
    };
    let (output, _) = served(repo, &CollectInput::default());
    assert!(output.is_err_and(|e| e.contains("past one answer")));
}

#[test]
fn a_project_outside_any_repository_fails_the_collect() {
    let repo = Repo {
        outside: true,
        ..Repo::default()
    };
    let (output, _) = served(repo, &CollectInput::default());
    assert!(output.is_err_and(|e| e.contains("in no git repository")));
}

fn node(id: &str, kind: &str, title: &str) -> GraphNode {
    let mut node = GraphNode {
        id: id.into(),
        kind: kind.into(),
        title: Some(title.into()),
        file: Some("spec/plans.spec".into()),
        ..Default::default()
    };
    // A task's `plan` reference, as the graph export writes it: its parent plan's id.
    if kind == "task"
        && let Some((plan, _)) = id.rsplit_once('/')
    {
        node.fields.insert("plan".into(), json!(plan));
    }
    node
}

fn created(output: &CollectOutput) -> Vec<(String, BTreeMap<String, Value>)> {
    output
        .plan
        .iter()
        .flat_map(|plan| &plan.ops)
        .filter_map(|op| match op {
            PlanOp::Create { id, fields, .. } => Some((id.clone(), fields.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn only_commits_naming_a_plan_are_recorded_oldest_first() {
    let commit =
        |c: char, at: i64, subject: &str| logged(&c.to_string().repeat(40), &[], at, subject, &[]);
    let log = vec![
        commit('2', 20, "feat(01-01): Store module"),
        commit('1', 10, "docs(pause): wip"),
        commit('3', 30, "docs(01-01): complete store plan"),
        commit('4', 40, "feat(gsd): x"),
        commit('5', 50, "feat(01-01): STORE   module"),
        commit('6', 60, "feat(02-01): unknown plan"),
    ];
    let input = CollectInput {
        graph: Some(GraphWire {
            nodes: vec![
                node("v1.0/01/01-01", "plan", "Store"),
                node("v1.0/01/01-01/T1", "task", "Store module"),
                node("v1.0/01/01-01/T2", "task", "Another"),
            ],
            edges: vec![],
        }),
        revisions: [("spec/plans.spec".to_string(), "sha256:x".to_string())].into(),
        ..Default::default()
    };
    let made = created(&collected_over(log, input));
    // Oldest first: the log answers the newest first, the log above is in that order reversed by `rev`.
    assert_eq!(
        made.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        ["555555555555", "333333333333", "222222222222"]
    );
    assert_eq!(made[2].1["task"], "v1.0/01/01-01/T1");
    assert_eq!(made[2].1["plan"], "v1.0/01/01-01");
    assert_eq!(made[2].1["type"], "feat");
    assert!(
        !made[1].1.contains_key("task"),
        "a metadata commit names the plan only"
    );
    assert_eq!(
        made[0].1["task"], "v1.0/01/01-01/T1",
        "case and whitespace are folded"
    );
}

#[test]
fn a_commit_the_project_holds_is_not_recorded_again() {
    let input = CollectInput {
        graph: Some(GraphWire {
            nodes: vec![
                node("v1.0/01/01-01", "plan", "Store"),
                node("222222222222", "commit", "x"),
            ],
            edges: vec![],
        }),
        ..Default::default()
    };
    let log = vec![logged(&"2".repeat(40), &[], 20, "feat(01-01): x", &[])];
    let (output, repo) = served(
        Repo {
            log,
            ..Repo::default()
        },
        &input,
    );
    assert!(output.expect("an answer").plan.is_none());
    assert!(repo.asked.borrow().is_empty(), "no diff for a held commit");
}

#[test]
fn two_tasks_with_one_title_name_the_plan_only() {
    let input = CollectInput {
        graph: Some(GraphWire {
            nodes: vec![
                node("v1.0/01/01-01", "plan", "Store"),
                node("v1.0/01/01-01/T1", "task", "Same"),
                node("v1.0/01/01-01/T2", "task", "Same"),
            ],
            edges: vec![],
        }),
        ..Default::default()
    };
    let log = vec![logged(&"2".repeat(40), &[], 20, "feat(01-01): Same", &[])];
    let made = created(&collected_over(log, input));
    assert_eq!(made.len(), 1);
    assert!(!made[0].1.contains_key("task"));
}

const HASH: &str = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";

fn plan_graph() -> Option<GraphWire> {
    Some(GraphWire {
        nodes: vec![
            node("v1.0/01/01-01", "plan", "Store"),
            node("v1.0/01/01-01/T1", "task", "Store module"),
            node("v1.0/01/01-01/T2", "task", "Another"),
        ],
        edges: vec![],
    })
}

#[test]
fn control_characters_in_a_subject_or_a_path_are_replaced() {
    let path = "dir/\u{1b}a\u{2028}b.js";
    let repo = Repo {
        log: vec![logged(
            HASH,
            &[],
            20,
            "fix(01-01): colour \u{1b}[31mred\u{1b}[0m\u{2028}x\u{7f}",
            &[],
        )],
        diffs: [(
            HASH.to_string(),
            vec![changed(path, GitChange::Added, 1, 0)],
        )]
        .into(),
        ..Repo::default()
    };
    let input = CollectInput {
        graph: plan_graph(),
        ..Default::default()
    };
    let made = created(&served(repo, &input).0.expect("an answer"));
    let fields = &made[0].1;
    assert_eq!(
        fields["subject"],
        "fix(01-01): colour \u{fffd}[31mred\u{fffd}[0m\u{fffd}x\u{fffd}"
    );
    assert_eq!(fields["files"], json!(["dir/\u{fffd}a\u{fffd}b.js"]));
    assert_eq!(fields["created"], fields["files"]);
}

#[test]
fn a_time_out_of_range_skips_its_commit_and_is_named() {
    let ok = logged(HASH, &[], 20, "feat(01-01): Store module", &[]);
    for at in [253_402_300_800, -1] {
        let bad = logged(&"d".repeat(40), &[], at, "feat(01-01): x", &[]);
        let input = CollectInput {
            graph: plan_graph(),
            ..Default::default()
        };
        let output = collected_over(vec![bad, ok.clone()], input);
        assert_eq!(created(&output).len(), 1, "{at}");
        let title = output.plan.and_then(|p| p.title).unwrap_or_default();
        assert!(
            title.contains("skipped 1") && title.contains("dddddddddddd"),
            "{at}: {title}"
        );
    }
}

#[test]
fn a_task_is_named_as_the_executor_words_it() {
    let commit =
        |c: char, at: i64, subject: &str| logged(&c.to_string().repeat(40), &[], at, subject, &[]);
    let log = vec![
        commit('5', 50, "feat(01-01): implement store module (#12)"),
        commit('4', 40, "test(01-01): add failing test for store module"),
        commit('3', 30, "feat(01-01): scaffold the thing"),
        commit('2', 20, "Revert \"feat(01-01): Store module\""),
    ];
    let input = CollectInput {
        graph: plan_graph(),
        ..Default::default()
    };
    let made = created(&collected_over(log, input));
    let by = |name: &str| {
        made.iter()
            .find(|(id, _)| id == name)
            .expect("recorded")
            .1
            .clone()
    };
    assert_eq!(by("555555555555")["task"], "v1.0/01/01-01/T1");
    assert_eq!(by("444444444444")["task"], "v1.0/01/01-01/T1");
    assert!(!by("333333333333").contains_key("task"));
    let revert = by("222222222222");
    assert_eq!(revert["type"], "revert");
    assert!(!revert.contains_key("task"));
}

#[test]
fn git_collector_writes_line_counts_as_numbers() {
    let commit = Commit {
        hash: "a".repeat(40),
        at: "2026-10-01T00:00:00Z".into(),
        subject: "feat(01-01): x".into(),
        insertions: 12,
        deletions: 3,
        ..Commit::default()
    };
    let plan = node("v1.0/01/01-01", "plan", "Store");
    let linked = Linked {
        plan: &plan,
        task: None,
        kind: Some("feat".into()),
        unread: Trailer::None,
    };
    let written = fields(&commit, &linked, &[]);
    assert_eq!(written["insertions"], serde_json::json!(12));
    assert_eq!(written["deletions"], serde_json::json!(3));
}

/// A commit of the log: the hash of `c`, the parents' hashes, the time, no trailer, the subject.
fn line(c: char, parents: &[char], at: i64, subject: &str) -> GitLogCommit {
    let parents: Vec<String> = parents.iter().map(|p| p.to_string().repeat(40)).collect();
    logged(&c.to_string().repeat(40), &parents, at, subject, &[])
}

fn name(c: char) -> String {
    c.to_string().repeat(12)
}

#[test]
fn a_commit_descends_from_the_recorded_commits_of_its_plan_through_merges() {
    // b ─ o ─ r ──────╮                   (r: RED on a side branch, o: another plan's commit)
    //  ╲              m ─ g ─ e           (m merges the side branch; g: GREEN, made with an earlier clock)
    //   ╰──────────────╯
    //  ╲ s                                 (s: a RED on a sibling branch nothing merged)
    let log = vec![
        line('e', &['d'], 60, "feat(01-01): implement another"),
        line('a', &['b'], 50, "test(01-01): add failing test for another"),
        line('d', &['c'], 20, "feat(01-01): implement store module"),
        line('c', &['b', 'f'], 40, "Merge branch 'side'"),
        line(
            'f',
            &['9'],
            30,
            "test(01-01): add failing test for store module",
        ),
        line('9', &['b'], 25, "feat(01-02): other work"),
        line('b', &[], 10, "docs(01-01): plan"),
    ];
    let mut graph = plan_graph().expect("a graph");
    graph.nodes.push(node("v1.0/01/01-02", "plan", "Other"));
    let input = CollectInput {
        graph: Some(graph),
        ..Default::default()
    };
    let made = created(&collected_over(log, input));
    let by = |c: char| {
        made.iter()
            .find(|(id, _)| *id == name(c))
            .unwrap_or_else(|| panic!("{c} recorded: {made:?}"))
            .1["descends_from"]
            .clone()
    };
    assert!(
        made.iter().all(|(id, _)| *id != name('c')),
        "a merge is not recorded"
    );
    // GREEN reaches the side branch's RED through the merge, whatever the clock says; another plan's commit is never
    // listed, though it is an ancestor.
    assert_eq!(by('d'), serde_json::json!([name('b'), name('f')]));
    assert_eq!(
        by('e'),
        serde_json::json!([name('b'), name('f'), name('d')])
    );
    // The sibling branch's RED is no one's ancestor; a root commit has none, and says so.
    assert_eq!(by('a'), serde_json::json!([name('b')]));
    assert_eq!(by('b'), serde_json::json!([]));
    assert_eq!(by('9'), serde_json::json!([]));
}

#[test]
fn a_commit_already_recorded_is_an_ancestor_but_is_not_written_again() {
    let log = vec![
        line('d', &['b'], 20, "feat(01-01): implement store module"),
        line(
            'b',
            &[],
            10,
            "test(01-01): add failing test for store module",
        ),
    ];
    let mut graph = plan_graph().expect("a graph");
    graph.nodes.push(node(&name('b'), "commit", "test"));
    let input = CollectInput {
        graph: Some(graph),
        ..Default::default()
    };
    let made = created(&collected_over(log, input));
    assert_eq!(made.len(), 1, "{made:?}");
    assert_eq!(made[0].1["descends_from"], serde_json::json!([name('b')]));
}

/// The plan graph with a `project` entity whose `project_code` is `code`.
fn coded_graph(code: Option<&str>) -> Option<GraphWire> {
    let mut graph = plan_graph()?;
    let mut project = GraphNode {
        id: "project".into(),
        kind: "project".into(),
        ..Default::default()
    };
    if let Some(code) = code {
        project.fields.insert("project_code".into(), json!(code));
    }
    graph.nodes.push(project);
    Some(graph)
}

fn recorded_plans(code: Option<&str>, subject: &str) -> Vec<Value> {
    let input = CollectInput {
        graph: coded_graph(code),
        ..Default::default()
    };
    created(&collected_over(vec![line('a', &[], 10, subject)], input))
        .into_iter()
        .map(|(_, fields)| fields["plan"].clone())
        .collect()
}

#[test]
fn a_scope_with_the_configured_project_code_resolves_its_plan() {
    let plan = [json!("v1.0/01/01-01")];
    assert_eq!(recorded_plans(Some("PRJ"), "feat(PRJ-01-01): x"), plan);
    assert_eq!(recorded_plans(Some("PRJ"), "feat(01-01): x"), plan);
    // Only the configured code is stripped, case-sensitively: an id that only looks coded is matched literally.
    assert!(recorded_plans(Some("PRJ"), "feat(AUTH-01-01): x").is_empty());
    assert!(recorded_plans(Some("PRJ"), "feat(prj-01-01): x").is_empty());
    assert!(recorded_plans(None, "feat(PRJ-01-01): x").is_empty());
}

/// A 40-hex hash for commit `i` of a synthetic log.
fn hash_of(i: usize) -> String {
    format!("{:012x}{:028x}", i + 1, 0)
}

/// A synthetic log, newest first, of `parents.len()` commits: commit `i` has the parents `parents[i]` and the scope
/// `scopes[i]` (`None`: no plan).
fn synthetic(parents: &[Vec<usize>], scopes: &[Option<String>]) -> Vec<GitLogCommit> {
    (0..parents.len())
        .rev()
        .map(|i| {
            let ps: Vec<String> = parents[i].iter().map(|&p| hash_of(p)).collect();
            let subject = scopes[i].as_ref().map_or(format!("chore: step {i}"), |s| {
                format!("feat({s}): step {i}")
            });
            logged(&hash_of(i), &ps, 1000 + i as i64, &subject, &[])
        })
        .collect()
}

/// Every recorded commit's `descends_from`, by 12-hex name.
fn ancestry_of(log: Vec<GitLogCommit>, graph: GraphWire) -> BTreeMap<String, Value> {
    let input = CollectInput {
        graph: Some(graph),
        ..Default::default()
    };
    created(&collected_over(log, input))
        .into_iter()
        .map(|(id, fields)| (id, fields["descends_from"].clone()))
        .collect()
}

#[test]
fn the_memoised_ancestry_equals_a_full_walk_through_merges() {
    // A branching history with merges, three plans and unplanned commits, from a fixed seed.
    let mut seed: u64 = 0x5eed;
    let mut next = |n: usize| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(seed >> 33).expect("31 bits") % n
    };
    let count = 300;
    let (mut parents, mut scopes) = (vec![Vec::new()], vec![Some("01-01".to_string())]);
    for i in 1..count {
        let mut ps = vec![i - 1 - next(i.min(6))];
        if next(5) == 0 {
            let other = next(i);
            if other != ps[0] {
                ps.push(other);
            }
        }
        parents.push(ps);
        scopes.push(match next(4) {
            0 => None,
            k => Some(format!("01-0{k}")),
        });
    }
    let mut graph = plan_graph().expect("a graph");
    graph.nodes.push(node("v1.0/01/01-02", "plan", "Two"));
    graph.nodes.push(node("v1.0/01/01-03", "plan", "Three"));
    let got = ancestry_of(synthetic(&parents, &scopes), graph);
    // The naive answer: every ancestor of `i`, through every parent, with the same scope, oldest first; merges are not
    // recorded.
    let recorded = |i: usize| scopes[i].is_some() && parents[i].len() < 2;
    let mut want = BTreeMap::new();
    for i in (0..count).filter(|&i| recorded(i)) {
        let mut seen = BTreeSet::new();
        let mut stack = parents[i].clone();
        while let Some(at) = stack.pop() {
            if seen.insert(at) {
                stack.extend(&parents[at]);
            }
        }
        let names: Vec<String> = seen
            .into_iter()
            .filter(|&a| recorded(a) && scopes[a] == scopes[i])
            .map(|a| hash_of(a)[..12].to_string())
            .collect();
        want.insert(hash_of(i)[..12].to_string(), json!(names));
    }
    assert!(
        want.values()
            .any(|v| v.as_array().is_some_and(|a| a.len() > 20))
    );
    assert_eq!(got, want);
}

#[test]
fn a_large_log_resolves_its_ancestry_quickly() {
    // 20 000 commits, every 10th of one of 100 plans, a merge every 50: the walk must stay near-linear to fit the
    // host's 30 s guest limit (a walk to the root per record is about 2 000 × 20 000 steps).
    let count = 20_000;
    let plan = |i: usize| format!("{:02}-{:02}", i / 10 % 100 / 10 + 1, i / 10 % 10 + 1);
    let parents: Vec<Vec<usize>> = (0..count)
        .map(|i| match i {
            0 => vec![],
            i if i % 50 == 0 && i > 7 => vec![i - 1, i - 7],
            i => vec![i - 1],
        })
        .collect();
    let scopes: Vec<Option<String>> = (0..count).map(|i| (i % 10 == 5).then(|| plan(i))).collect();
    let mut nodes: Vec<GraphNode> = (0..100)
        .map(|p| plan(p * 10))
        .map(|id| node(&format!("v1.0/{}/{id}", &id[..2]), "plan", &id))
        .collect();
    nodes.dedup_by(|a, b| a.id == b.id);
    let plans = nodes.len();
    WALKED.with(|n| n.set(0));
    let got = ancestry_of(
        synthetic(&parents, &scopes),
        GraphWire {
            nodes,
            edges: vec![],
        },
    );
    let walked = WALKED.with(std::cell::Cell::get);
    // Work, not time (a 5 s clock bound flaked under the full gate). A plan's first commit walks at most the whole log;
    // every other record stops near its plan's previous one, `gap` commits back on each of a merge's two sides. The
    // walk to the root per record steps on 20 000 000 commits; this one on 5 563 800 (the history is fixed).
    let (records, gap) = (count / 10, 10 * plans);
    let bound = records * 2 * gap + plans * count;
    assert!(
        walked <= bound,
        "walked {walked} commits (bound {bound}) of {count} for {plans} plans"
    );
    assert_eq!(got.len(), 2_000);
    assert_eq!(
        got[&hash_of(19_995)[..12]].as_array().map(Vec::len),
        Some(19)
    );
}
