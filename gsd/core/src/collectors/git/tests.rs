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
