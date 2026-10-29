use specforge_extension_sdk::GraphWire;

use super::xml::{Event, decode, events};
use super::*;

#[test]
fn xml_entities_and_self_closing_testcases() {
    assert_eq!(
        decode("a &amp; b &lt;c&gt; &quot;d&quot; &apos;e&apos; &#65;&#x42; &unknown; &"),
        "a & b <c> \"d\" 'e' AB &unknown; &"
    );
    let report = r#"<?xml version="1.0"?>
<!-- a comment with <testcase name="no"> in it -->
<testsuites>
  <testsuite name="s">
    <testcase name="adds &amp; lists" classname="test.tasks" time="0.0123" file="/p/test/t.js"/>
    <testcase name='single "quoted"' time="1"><failure message="a > b">text</failure><system-out><![CDATA[<testcase name="cdata"/>]]></system-out></testcase>
    <testcase name="skips"><skipped/></testcase>
    <testcase name="errors"><error type="x"/></testcase>
  </testsuite>
</testsuites>"#;
    let found = cases(report).expect("a report");
    let summary: Vec<(&str, TestStatus)> =
        found.iter().map(|c| (c.name.as_str(), c.status)).collect();
    assert_eq!(
        summary,
        [
            ("adds & lists", TestStatus::Passed),
            ("single \"quoted\"", TestStatus::Failed),
            ("skips", TestStatus::Skipped),
            ("errors", TestStatus::Failed),
        ]
    );
    assert_eq!(found[0].classname, "test.tasks");
    assert_eq!(found[0].file, "/p/test/t.js");
    assert_eq!(found[0].seconds, Some(0.0123));
    assert_eq!(found[1].seconds, Some(1.0));
    let tags = events("<a><b x=\"1\"/></a>").expect("tags");
    assert_eq!(tags.len(), 3);
    assert!(matches!(&tags[1], Event::Open { name, empty: true, .. } if name == "b"));
}

#[test]
fn a_report_that_does_not_close_is_an_error() {
    for truncated in [
        "<testsuite><testcase name=\"a",
        "<testsuite><testcase name=\"a\"",
        "<testsuite><!-- never closed",
        "<testsuite><![CDATA[ never closed",
        "<testsuite></testsuite",
        "<testcase name=a/>",
        "<testcase name/>",
    ] {
        assert!(cases(truncated).is_err(), "{truncated:?}");
    }
    assert_eq!(cases("").expect("an empty report"), Vec::new());
}

fn node(id: &str, kind: &str) -> GraphNode {
    GraphNode {
        id: id.into(),
        kind: kind.into(),
        ..Default::default()
    }
}

fn deliverable(id: &str, refs: &[&str]) -> GraphNode {
    let mut n = node(id, "deliverable");
    n.fields.insert(
        "checks".into(),
        serde_json::json!(
            refs.iter()
                .map(|r| serde_json::json!({"kind": "unit", "ref": r}))
                .collect::<Vec<_>>()
        ),
    );
    n
}

fn run(report: &str, nodes: Vec<GraphNode>) -> CollectOutput {
    collect(&CollectInput {
        reports: vec![CollectReportFile {
            path: "junit.xml".into(),
            content: report.into(),
        }],
        cwd: Some("/p".into()),
        graph: Some(GraphWire {
            nodes,
            edges: vec![],
        }),
        ..Default::default()
    })
    .expect("an answer")
}

#[test]
fn a_tag_links_by_the_graphs_rules_and_a_bad_one_is_unlinked() {
    let nodes = vec![
        node("v1.0/01/01-01/TR1", "truth"),
        node("v1.0/01/01-02/TR1", "truth"),
        node("v1.0/01/01-02/T1", "task"),
    ];
    let report = r#"<testsuite>
<testcase name="[gsd:01-01/TR1] adds"/>
<testcase name="[gsd:TR1] ambiguous"/>
<testcase name="[gsd:nobody] unknown"/>
<testcase name="plain"/>
<testcase name="[gsd:v1.0/01/01-02/T1] [gsd:/v1.0/01/01-02/TR1] two tags"><failure/></testcase>
</testsuite>"#;
    let output = run(report, nodes);
    let linked: Vec<(&str, Vec<(&str, TestStatus)>)> = output
        .entity_results
        .iter()
        .map(|r| {
            (
                r.entity_id.as_str(),
                r.test_results
                    .iter()
                    .map(|t| (t.name.as_str(), t.status))
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        linked,
        [
            (
                "v1.0/01/01-01/TR1",
                vec![("[gsd:01-01/TR1] adds", TestStatus::Passed)]
            ),
            (
                "v1.0/01/01-02/T1",
                vec![(
                    "[gsd:v1.0/01/01-02/T1] [gsd:/v1.0/01/01-02/TR1] two tags",
                    TestStatus::Failed
                )]
            ),
            (
                "v1.0/01/01-02/TR1",
                vec![(
                    "[gsd:v1.0/01/01-02/T1] [gsd:/v1.0/01/01-02/TR1] two tags",
                    TestStatus::Failed
                )]
            ),
        ]
    );
    let unlinked: Vec<&str> = output.unlinked.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(
        unlinked,
        ["[gsd:TR1] ambiguous", "[gsd:nobody] unknown", "plain"]
    );
}

#[test]
fn a_coverage_ref_links_by_name_and_file() {
    let nodes = vec![deliverable(
        "v1.0/01/01-01/01-01-SUMMARY/D1",
        &[
            "test/tasks.test.js#adds a task",
            "#any file",
            "other.test.js#adds a task",
        ],
    )];
    let report = r#"<testsuite>
<testcase name="adds a task" file="/p/test/tasks.test.js" time="0.5"/>
<testcase name="adds a task" file="/q/elsewhere/tasks.test.js"/>
<testcase name="any file" file="/q/x.js"/>
</testsuite>"#;
    let output = run(report, nodes);
    let [one] = &output.entity_results[..] else {
        panic!("one entity: {:?}", output.entity_results);
    };
    assert_eq!(one.entity_id, "v1.0/01/01-01/01-01-SUMMARY/D1");
    let names: Vec<&str> = one.test_results.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["test/tasks.test.js#adds a task", "#any file"]);
    assert_eq!(one.test_results[0].duration_ms, Some(500.0));
    let unlinked: Vec<&str> = output.unlinked.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(
        unlinked,
        ["adds a task"],
        "the test of another file proves no check"
    );
}

#[test]
fn the_relative_path_is_the_reports_or_the_roots() {
    assert_eq!(relative("/p/test/a.js", "/p"), "test/a.js");
    assert_eq!(relative("test/a.js", "/p"), "test/a.js");
    assert_eq!(relative("./test/a.js", "/p"), "test/a.js");
    assert_eq!(relative("/q/test/a.js", "/p"), "/q/test/a.js");
    // A sibling directory that starts with the root's name is not under it.
    assert_eq!(relative("/pp/test/a.js", "/p"), "/pp/test/a.js");
}

#[test]
fn an_unlinked_test_keeps_its_classname_segments() {
    let output = run(
        r#"<testsuite><testcase name="t" classname="acme.Add" /></testsuite>"#,
        vec![],
    );
    assert_eq!(output.unlinked[0].path, ["acme", "Add", "t"]);
}

#[test]
fn the_collector_never_answers_a_plan() {
    for report in [
        "",
        "<testsuite><testcase name=\"[gsd:x] a\"/></testsuite>",
        "<testsuite/>",
    ] {
        for nodes in [vec![], vec![node("x", "truth")]] {
            assert!(run(report, nodes).plan.is_none(), "{report:?}");
        }
    }
}

fn collected(report: &str) -> Result<CollectOutput, String> {
    collect(&CollectInput {
        reports: vec![CollectReportFile {
            path: "junit.xml".into(),
            content: report.into(),
        }],
        cwd: Some("/p".into()),
        graph: Some(GraphWire {
            nodes: vec![node("v1.0/01/01-01/TR1", "truth")],
            edges: vec![],
        }),
        ..Default::default()
    })
}

#[test]
fn a_report_cut_inside_a_test_or_no_xml_is_refused() {
    // The last test failed and the report stopped: it must not read as an empty report.
    let cut = "<testsuite><testcase name=\"[gsd:01-01/TR1] a\"><failure/>";
    assert!(collected(cut).is_err());
    assert!(collected("hello world").is_err());
    assert!(collected("<testsuite/>").is_ok());
}

#[test]
fn two_tags_of_one_entity_are_one_result_and_a_bad_tag_is_listed() {
    let output = collected(
        "<testsuite><testcase name=\"[gsd:01-01/TR1] [gsd:v1.0/01/01-01/TR1] [gsd:nobody] a\"/></testsuite>",
    )
    .expect("an answer");
    assert_eq!(output.entity_results.len(), 1);
    assert_eq!(output.entity_results[0].test_results.len(), 1);
    assert_eq!(output.unlinked.len(), 1, "the unresolved tag is reported");
}

#[test]
fn a_time_is_a_finite_non_negative_number() {
    for (time, seconds) in [
        ("1.5", Some(1.5)),
        ("-5", None),
        ("NaN", None),
        ("inf", None),
    ] {
        let found = cases(&format!(
            "<testsuite><testcase name=\"a\" time=\"{time}\"/></testsuite>"
        ))
        .expect("a report");
        assert_eq!(found[0].seconds, seconds, "{time}");
    }
}

#[test]
fn a_malformed_entity_costs_no_more_than_its_length() {
    let value = "&amp".repeat(100_000);
    let started = std::time::Instant::now();
    assert_eq!(decode(&value).len(), value.len());
    assert!(started.elapsed().as_secs() < 2);
}
