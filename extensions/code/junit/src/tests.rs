use super::xml::{decode, events, Event};
use super::*;

/// `node --test --test-reporter=junit` (node 26.12) on one file: a top-level test, `describe("board")` holding
/// `adds` and `describe("inner")` holding a failing `adds`, a skipped test and a `todo`; paths under `/project`.
const NODE26_NESTED: &str = include_str!("../fixtures/node26-nested.xml");
/// The taskboard twin's recorded report (`gsd/parity/goldens/taskboard/junit.xml`): three top-level tests, files
/// relative to the project.
const TASKBOARD: &str = include_str!("../fixtures/taskboard.xml");

/// Each case's `(suites > name, status)`.
fn summary(found: &[Case]) -> Vec<(String, TestStatus)> {
    found
        .iter()
        .map(|c| {
            let mut path = c.suites.clone();
            path.push(c.name.clone());
            (path.join(" > "), c.status)
        })
        .collect()
}

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
    assert_eq!(
        summary(&found),
        [
            ("s > adds & lists".to_string(), TestStatus::Passed),
            ("s > single \"quoted\"".to_string(), TestStatus::Failed),
            ("s > skips".to_string(), TestStatus::Skipped),
            ("s > errors".to_string(), TestStatus::Failed),
        ]
    );
    assert_eq!(found[0].classname, "test.tasks");
    assert_eq!(found[0].file.as_deref(), Some("/p/test/t.js"));
    assert_eq!(found[1].file, None);
    assert_eq!(found[0].seconds, Some(0.0123));
    assert_eq!(found[1].seconds, Some(1.0));
    let tags = events("<a><b x=\"1\"/></a>").expect("tags");
    assert_eq!(tags.len(), 3);
    assert!(matches!(&tags[1], Event::Open { name, empty: true, .. } if name == "b"));
}

#[test]
fn a_truncated_report_is_an_error() {
    for truncated in [
        "<testsuite><testcase name=\"a",
        "<testsuite><testcase name=\"a\"",
        "<testsuite><!-- never closed",
        "<testsuite><![CDATA[ never closed",
        "<testsuite></testsuite",
        "<testcase name=a/>",
        "<testcase name/>",
        "<testsuites><testcase name=\"last\"><failure/>",
    ] {
        assert!(cases(truncated).is_err(), "{truncated:?}");
    }
    assert_eq!(cases("").expect("an empty report"), Vec::new());
    assert!(cases("<results><testcase name=\"a\"/></results>")
        .unwrap_err()
        .contains("not a JUnit report"));
}

#[test]
fn nested_suites_make_the_path() {
    let found = cases(NODE26_NESTED).expect("node's report");
    assert_eq!(
        summary(&found),
        [
            ("top".to_string(), TestStatus::Passed),
            ("board > adds".to_string(), TestStatus::Passed),
            ("board > inner > adds".to_string(), TestStatus::Failed),
            ("skipped".to_string(), TestStatus::Skipped),
            ("todo one".to_string(), TestStatus::Skipped),
        ]
    );
    let ids: Vec<String> = found
        .iter()
        .map(|c| TestId::js("probe", "test/a.test.js", c).to_string())
        .collect();
    assert_eq!(
        ids,
        [
            "js:probe/test/a.test.js#top",
            "js:probe/test/a.test.js#board > adds",
            "js:probe/test/a.test.js#board > inner > adds",
            "js:probe/test/a.test.js#skipped",
            "js:probe/test/a.test.js#todo one",
        ]
    );
    assert_eq!(
        relative(found[2].file.as_deref().unwrap(), "/project").as_deref(),
        Some("test/a.test.js")
    );
}

#[test]
fn todo_is_skipped() {
    let report = r#"<testsuites><testcase name="later"><skipped type="todo" message="true"/></testcase></testsuites>"#;
    assert_eq!(cases(report).unwrap()[0].status, TestStatus::Skipped);
}

#[test]
fn a_windows_file_under_a_verbatim_cwd_is_relative() {
    let file = r"C:\work\app\test\a.test.js";
    assert_eq!(
        relative(file, r"\\?\C:\work\app").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(
        relative(file, r"\\?\c:\work\app\").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(
        relative(r"\\server\share\app\t.js", r"\\?\UNC\server\share\app").as_deref(),
        Some("t.js")
    );
    assert_eq!(relative(file, r"\\?\D:\work\app"), None);
}

#[test]
fn a_private_var_alias_is_relative() {
    let file = "/var/folders/x/T/p/test/a.test.js";
    assert_eq!(
        relative(file, "/private/var/folders/x/T/p").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(
        relative("/private/tmp/p/test/a.test.js", "/tmp/p").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(relative("/variable/p/a.js", "/private/variable/p"), None);
}

#[test]
fn a_file_outside_cwd_is_none() {
    assert_eq!(relative("/elsewhere/test/a.test.js", "/project"), None);
    assert_eq!(relative("/project-two/a.test.js", "/project"), None);
    assert_eq!(relative("/project", "/project"), None);
    assert_eq!(
        relative("./test/a.test.js", "/project").as_deref(),
        Some("test/a.test.js")
    );
    assert_eq!(
        relative("/project/a.js", "/").as_deref(),
        Some("project/a.js")
    );
}

#[test]
fn a_file_that_climbs_out_through_dot_dot_is_outside() {
    for escaping in [
        "../x.test.js",
        "./test/../../x.test.js",
        r"..\x.test.js",
        "/project/../x.test.js",
        "/project/test/../../elsewhere/x.test.js",
    ] {
        assert_eq!(relative(escaping, "/project"), None, "{escaping}");
    }
    for (inside, expected) in [
        ("test/../src/a.test.js", "src/a.test.js"),
        ("/project/test/./unit/../a.test.js", "test/a.test.js"),
    ] {
        assert_eq!(
            relative(inside, "/project").as_deref(),
            Some(expected),
            "{inside}"
        );
    }
}

#[test]
fn a_duplicate_id_keeps_the_worst_status() {
    let case = |status, seconds| Case {
        suites: vec!["board".to_string()],
        name: "adds".to_string(),
        classname: "board".to_string(),
        file: Some("test/a.test.js".to_string()),
        seconds,
        status,
    };
    let id = |c: &Case| TestId::js("@acme/app", "test/a.test.js", c);
    let a = case(TestStatus::Passed, Some(0.002));
    let b = case(TestStatus::Failed, Some(0.003));
    let c = case(TestStatus::Skipped, None);
    let facts = facts([(id(&a), a), (id(&b), b), (id(&c), c)]);

    let wanted = [
        Fact::new("test", "js:@acme/app/test/a.test.js#board > adds")
            .title("board > adds")
            .field("package", "@acme/app")
            .field("suite", vec!["board"])
            .field("name", "adds")
            .field("status", "failed")
            .field("file", "test/a.test.js"),
        Fact::new("test_run", "run:js:@acme/app/test/a.test.js#board > adds")
            .field("test", "js:@acme/app/test/a.test.js#board > adds")
            .field("status", "failed")
            .field("duration_ms", 5),
    ];
    assert_eq!(facts, wanted);
}

#[test]
fn the_taskboard_test_id_is_the_exit_string() {
    let found = cases(TASKBOARD).expect("the twin's report");
    let minted = facts(found.into_iter().map(|case| {
        let file = relative(case.file.as_deref().unwrap(), "/anywhere").unwrap();
        (TestId::js("taskboard", &file, &case), case)
    }));
    let tests: Vec<(&str, Option<&str>)> = minted
        .iter()
        .filter(|f| f.kind == "test")
        .map(|f| (f.id.as_str(), f.fields["status"].as_str()))
        .collect();
    assert_eq!(
        tests,
        [
            (
                "js:taskboard/test/tasks.test.js#a task is added with the next id",
                Some("passed")
            ),
            (
                "js:taskboard/test/tasks.test.js#a task is completed by its id",
                Some("passed")
            ),
            (
                "js:taskboard/test/tasks.test.js#completing an unknown id throws",
                Some("passed")
            ),
        ]
    );
    assert_eq!(minted.len(), 6, "a test and a run per case");
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

#[test]
fn a_tests_file_is_its_ids_whatever_the_report_wrote() {
    let report = r#"<testsuites><testsuite name="board"><testcase name="adds" file="C:\work\app\test\a.test.js"/></testsuite></testsuites>"#;
    let case = cases(report).expect("a report").remove(0);
    let file = relative(case.file.as_deref().unwrap(), r"\\?\C:\work\app").unwrap();
    let minted = facts([(TestId::js("app", &file, &case), case)]);
    assert_eq!(minted[0].id, "js:app/test/a.test.js#board > adds");
    assert_eq!(minted[0].fields["file"].as_str(), Some("test/a.test.js"));
}
