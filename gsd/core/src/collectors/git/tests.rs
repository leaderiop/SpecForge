use super::subject::{Header, parse as header};
use specforge_extension_sdk::{GraphWire, PlanOp};

use super::*;

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

#[test]
fn a_quoted_path_is_unquoted() {
    assert_eq!(unquote("src/a.js"), "src/a.js");
    assert_eq!(unquote("\"a\\tb.txt\""), "a\tb.txt");
    assert_eq!(unquote("\"say \\\"hi\\\".txt\""), "say \"hi\".txt");
    assert_eq!(unquote("\"back\\\\slash\""), "back\\slash");
    assert_eq!(unquote("\"caf\\303\\251.txt\""), "café.txt");
}

#[test]
fn a_binary_numstat_counts_zero() {
    let log = "\u{0}aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\u{1f}\u{1f}0\u{1f}feat(01-01): x\n\n3\t1\tsrc/a.js\n-\t-\timg/logo.png\n create mode 100644 src/a.js\n create mode 100644 img/logo.png\n";
    let [commit] = &parse(log)[..] else {
        panic!("one commit");
    };
    assert_eq!((commit.insertions, commit.deletions), (3, 1));
    assert_eq!(commit.files, ["src/a.js", "img/logo.png"]);
    assert_eq!(commit.created, ["src/a.js", "img/logo.png"]);
}

#[test]
fn a_created_path_may_hold_spaces() {
    let log = "\u{0}bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\u{1f}\u{1f}60\u{1f}docs: x\n\n1\t0\tmy notes.md\n create mode 100644 my notes.md\n";
    assert_eq!(parse(log)[0].created, ["my notes.md"]);
    assert_eq!(parse(log)[0].at, "1970-01-01T00:01:00Z");
}

#[test]
fn an_empty_log_is_no_plan() {
    assert!(parse("").is_empty());
    for stdout in ["", "\n", "garbage without a record separator"] {
        let input = CollectInput {
            stdout: Some(stdout.to_string()),
            graph: Some(Default::default()),
            ..Default::default()
        };
        let output = collect(&input).expect("an answer");
        assert!(
            output.plan.is_none() && output.entity_results.is_empty(),
            "{stdout:?}"
        );
    }
}

fn node(id: &str, kind: &str, title: &str) -> GraphNode {
    GraphNode {
        id: id.into(),
        kind: kind.into(),
        title: Some(title.into()),
        file: Some("spec/plans.spec".into()),
        ..Default::default()
    }
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
    let log = [
        "\u{0}2222222222222222222222222222222222222222\u{1f}\u{1f}20\u{1f}feat(01-01): Store module\n\n1\t0\ta.js\n",
        "\u{0}1111111111111111111111111111111111111111\u{1f}\u{1f}10\u{1f}docs(pause): wip\n",
        "\u{0}3333333333333333333333333333333333333333\u{1f}\u{1f}30\u{1f}docs(01-01): complete store plan\n",
        "\u{0}4444444444444444444444444444444444444444\u{1f}\u{1f}40\u{1f}feat(gsd): x\n",
        "\u{0}5555555555555555555555555555555555555555\u{1f}\u{1f}50\u{1f}feat(01-01): STORE   module\n",
        "\u{0}6666666666666666666666666666666666666666\u{1f}\u{1f}60\u{1f}feat(02-01): unknown plan\n",
    ]
    .concat();
    let input = CollectInput {
        stdout: Some(log),
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
    let made = created(&collect(&input).expect("an answer"));
    // Oldest first: git prints the newest first, the log above is in that order reversed by `rev`.
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
        stdout: Some(
            "\u{0}2222222222222222222222222222222222222222\u{1f}\u{1f}20\u{1f}feat(01-01): x\n"
                .to_string(),
        ),
        graph: Some(GraphWire {
            nodes: vec![
                node("v1.0/01/01-01", "plan", "Store"),
                node("222222222222", "commit", "x"),
            ],
            edges: vec![],
        }),
        ..Default::default()
    };
    assert!(collect(&input).expect("an answer").plan.is_none());
}

#[test]
fn two_tasks_with_one_title_name_the_plan_only() {
    let input = CollectInput {
        stdout: Some(
            "\u{0}2222222222222222222222222222222222222222\u{1f}\u{1f}20\u{1f}feat(01-01): Same\n"
                .to_string(),
        ),
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
    let made = created(&collect(&input).expect("an answer"));
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
    let log = format!(
        "\u{0}{HASH}\u{1f}\u{1f}20\u{1f}fix(01-01): colour \u{1b}[31mred\u{1b}[0m\u{2028}x\u{7f}\n\n1\t0\tdir/\u{1b}a\u{2028}b.js\n create mode 100644 dir/\u{1b}a\u{2028}b.js\n"
    );
    let [commit] = &parse(&log)[..] else {
        panic!("one commit");
    };
    assert_eq!(
        commit.subject,
        "fix(01-01): colour \u{fffd}[31mred\u{fffd}[0m\u{fffd}x\u{fffd}"
    );
    assert_eq!(commit.files, ["dir/\u{fffd}a\u{fffd}b.js"]);
    assert_eq!(commit.created, commit.files);
    assert_eq!(unquote("\"a\\ab\\vc\""), "a\u{fffd}b\u{fffd}c");
}

#[test]
fn a_subject_cannot_forge_a_record() {
    // The separators the collector used to split on are plain text in a subject now.
    let log = format!(
        "\u{0}{HASH}\u{1f}\u{1f}20\u{1f}feat(01-01): forged\u{1e}deadbeef\u{1f}0\u{1f}feat(01-01): Store module\n"
    );
    let (commits, skipped) = parse_all(&log);
    assert_eq!(commits.len(), 1);
    assert!(skipped.is_empty());
    assert!(commits[0].subject.starts_with("feat(01-01): forged"));
}

#[test]
fn a_bad_header_skips_its_commit_and_is_named() {
    let ok = format!("\u{0}{HASH}\u{1f}\u{1f}20\u{1f}feat(01-01): Store module\n");
    for bad in [
        "\u{0}deadbeef\u{1f}\u{1f}20\u{1f}feat(01-01): x\n".to_string(),
        format!(
            "\u{0}{}\u{1f}\u{1f}20\u{1f}feat(01-01): x\n",
            "g".repeat(40)
        ),
        format!(
            "\u{0}{}\u{1f}\u{1f}20\u{1f}feat(01-01): x\n",
            "A".repeat(40)
        ),
        format!("\u{0}{HASH}\u{1f}\u{1f}999999999999999\u{1f}feat(01-01): x\n"),
        format!("\u{0}{HASH}\u{1f}\u{1f}-1\u{1f}feat(01-01): x\n"),
        format!("\u{0}{HASH}\u{1f}deadbeef\u{1f}20\u{1f}feat(01-01): x\n"),
    ] {
        let input = CollectInput {
            stdout: Some(format!("{bad}{ok}")),
            graph: plan_graph(),
            ..Default::default()
        };
        let output = collect(&input).expect("an answer");
        assert_eq!(created(&output).len(), 1, "{bad:?}");
        let title = output.plan.and_then(|p| p.title).unwrap_or_default();
        assert!(title.contains("skipped 1"), "{bad:?}: {title}");
    }
}

#[test]
fn a_task_is_named_as_the_executor_words_it() {
    let log = [
        format!(
            "\u{0}{}\u{1f}\u{1f}50\u{1f}feat(01-01): implement store module (#12)\n",
            "5".repeat(40)
        ),
        format!(
            "\u{0}{}\u{1f}\u{1f}40\u{1f}test(01-01): add failing test for store module\n",
            "4".repeat(40)
        ),
        format!(
            "\u{0}{}\u{1f}\u{1f}30\u{1f}feat(01-01): scaffold the thing\n",
            "3".repeat(40)
        ),
        format!(
            "\u{0}{}\u{1f}\u{1f}20\u{1f}Revert \"feat(01-01): Store module\"\n",
            "2".repeat(40)
        ),
    ]
    .concat();
    let input = CollectInput {
        stdout: Some(log),
        graph: plan_graph(),
        ..Default::default()
    };
    let made = created(&collect(&input).expect("an answer"));
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
        parents: Vec::new(),
        files: Vec::new(),
        created: Vec::new(),
        insertions: 12,
        deletions: 3,
    };
    let written = fields(&commit, "feat", "v1.0/01/01-01", None, &[]);
    assert_eq!(written["insertions"], serde_json::json!(12));
    assert_eq!(written["deletions"], serde_json::json!(3));
}

/// A record of `git log --format=%x00%H%x1f%P%x1f%at%x1f%s`: the hash of `c`, the parents' hashes, the time, the subject.
fn line(c: char, parents: &[char], at: u32, subject: &str) -> String {
    let parents: Vec<String> = parents.iter().map(|p| p.to_string().repeat(40)).collect();
    format!(
        "\u{0}{}\u{1f}{}\u{1f}{at}\u{1f}{subject}\n",
        c.to_string().repeat(40),
        parents.join(" ")
    )
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
    let log = [
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
    ]
    .concat();
    let mut graph = plan_graph().expect("a graph");
    graph.nodes.push(node("v1.0/01/01-02", "plan", "Other"));
    let input = CollectInput {
        stdout: Some(log),
        graph: Some(graph),
        ..Default::default()
    };
    let made = created(&collect(&input).expect("an answer"));
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
    let log = [
        line('d', &['b'], 20, "feat(01-01): implement store module"),
        line(
            'b',
            &[],
            10,
            "test(01-01): add failing test for store module",
        ),
    ]
    .concat();
    let mut graph = plan_graph().expect("a graph");
    graph.nodes.push(node(&name('b'), "commit", "test"));
    let input = CollectInput {
        stdout: Some(log),
        graph: Some(graph),
        ..Default::default()
    };
    let made = created(&collect(&input).expect("an answer"));
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
        stdout: Some(line('a', &[], 10, subject)),
        graph: coded_graph(code),
        ..Default::default()
    };
    created(&collect(&input).expect("an answer"))
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
fn synthetic(parents: &[Vec<usize>], scopes: &[Option<String>]) -> String {
    (0..parents.len())
        .rev()
        .map(|i| {
            let ps: Vec<String> = parents[i].iter().map(|&p| hash_of(p)).collect();
            let subject = scopes[i].as_ref().map_or(format!("chore: step {i}"), |s| {
                format!("feat({s}): step {i}")
            });
            format!(
                "\u{0}{}\u{1f}{}\u{1f}{}\u{1f}{subject}\n",
                hash_of(i),
                ps.join(" "),
                1000 + i
            )
        })
        .collect()
}

/// Every recorded commit's `descends_from`, by 12-hex name.
fn ancestry_of(log: String, graph: GraphWire) -> BTreeMap<String, Value> {
    let input = CollectInput {
        stdout: Some(log),
        graph: Some(graph),
        ..Default::default()
    };
    created(&collect(&input).expect("an answer"))
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
