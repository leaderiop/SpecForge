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
fn unix_seconds_become_utc_rfc3339() {
    assert_eq!(utc(0), "1970-01-01T00:00:00Z");
    assert_eq!(utc(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(utc(1_709_164_799), "2024-02-28T23:59:59Z");
    assert_eq!(utc(1_709_164_800), "2024-02-29T00:00:00Z");
    assert_eq!(utc(1_709_251_200), "2024-03-01T00:00:00Z");
    assert_eq!(utc(1_761_732_000), "2025-10-29T10:00:00Z");
    assert_eq!(utc(2_147_483_648), "2038-01-19T03:14:08Z");
    assert_eq!(utc(4_107_542_400), "2100-03-01T00:00:00Z");
    assert_eq!(utc(-1), "1969-12-31T23:59:59Z");
}

#[test]
fn a_binary_numstat_counts_zero() {
    let log = "\u{1e}aaaa\u{1f}0\u{1f}feat(01-01): x\n\n3\t1\tsrc/a.js\n-\t-\timg/logo.png\n create mode 100644 src/a.js\n create mode 100644 img/logo.png\n";
    let [commit] = &parse(log)[..] else {
        panic!("one commit");
    };
    assert_eq!((commit.insertions, commit.deletions), (3, 1));
    assert_eq!(commit.files, ["src/a.js", "img/logo.png"]);
    assert_eq!(commit.created, ["src/a.js", "img/logo.png"]);
}

#[test]
fn a_created_path_may_hold_spaces() {
    let log =
        "\u{1e}bbbb\u{1f}60\u{1f}docs: x\n\n1\t0\tmy notes.md\n create mode 100644 my notes.md\n";
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
        "\u{1e}2222222222222222\u{1f}20\u{1f}feat(01-01): Store module\n\n1\t0\ta.js\n",
        "\u{1e}1111111111111111\u{1f}10\u{1f}docs(pause): wip\n",
        "\u{1e}3333333333333333\u{1f}30\u{1f}docs(01-01): complete store plan\n",
        "\u{1e}4444444444444444\u{1f}40\u{1f}feat(gsd): x\n",
        "\u{1e}5555555555555555\u{1f}50\u{1f}feat(01-01): STORE   module\n",
        "\u{1e}6666666666666666\u{1f}60\u{1f}feat(02-01): unknown plan\n",
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
        stdout: Some("\u{1e}2222222222222222\u{1f}20\u{1f}feat(01-01): x\n".to_string()),
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
        stdout: Some("\u{1e}2222222222222222\u{1f}20\u{1f}feat(01-01): Same\n".to_string()),
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
