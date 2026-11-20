use super::*;

#[test]
fn a_project_path_is_relative_and_stays_inside() {
    for path in [
        "src/tasks.js",
        "test/tasks.test.js",
        "./src/a.js",
        "a..b/c.js",
    ] {
        assert!(in_project(path), "{path}");
    }
    for path in [
        "",
        "/etc/passwd",
        "\\share\\x",
        "C:/x.js",
        "c:x.js",
        "../outside.js",
        "src/../../x.js",
        "src\\..\\x.js",
    ] {
        assert!(!in_project(path), "{path}");
    }
}

/// The id of the one test fact [`collected`] holds.
const FACT: &str = "js:taskboard/test/tasks.test.js#adds";

/// A command graph holding one collected test fact, `adds` of `test/tasks.test.js`.
fn collected() -> CommandGraph {
    let fact: GraphNode = serde_json::from_value(serde_json::json!({
        "id": FACT, "kind": "test",
        "fields": { "name": "adds", "file": "test/tasks.test.js", "status": "passed" }
    }))
    .expect("a test fact");
    CommandGraph::new(vec![fact], Vec::new())
}

static COLLECTED: std::sync::LazyLock<CommandGraph> = std::sync::LazyLock::new(collected);

fn proposed<'a>(
    code: &str,
    kind: &'a str,
    outcome: &'a str,
    values: &'a BTreeMap<String, Value>,
    cited: &'a [Cite],
) -> Proposed<'a> {
    Proposed {
        entity: "v1.0/01/01-01/TR1",
        kind,
        rule: crate::judged::get(code).expect("a judged rule"),
        outcome,
        values,
        cited,
        graph: &COLLECTED,
    }
}

fn test(name: &str) -> Cite {
    Cite::Test {
        test: name.to_string(),
    }
}

/// A cited test names a collected test fact by its id, as `file#name` or by its name; one that names none is refused,
/// naming it and how to cite, on any rule; with no test fact collected, the refusal says so.
#[test]
fn a_cited_test_must_name_a_collected_test_fact() {
    let values = BTreeMap::new();
    for name in [FACT, "test/tasks.test.js#adds", "#adds", "adds"] {
        let cited = [test(name)];
        for (code, kind) in [("E501", "truth"), ("W311", "plan")] {
            assert!(
                cited_tests(&proposed(code, kind, "verified", &values, &cited)).is_ok(),
                "{name}"
            );
        }
    }
    let cited = [test("test/other.test.js#adds"), test("removes")];
    let why =
        cited_tests(&proposed("W311", "plan", "passed", &values, &cited)).expect_err("refused");
    assert_eq!(
        why,
        "cited: 'test/other.test.js#adds', 'removes' name no collected test fact; cite a test fact by its id, as \
         `specforge trace --format json` prints it, or as `file#name`"
    );
    let empty = CommandGraph::default();
    let cited = [test("adds")];
    let why = cited_tests(&Proposed {
        graph: &empty,
        ..proposed("E501", "truth", "verified", &values, &cited)
    })
    .expect_err("refused");
    assert_eq!(
        why,
        "cited: 'adds' names no collected test fact; none is collected: collect the tests (`specforge collect`) or \
         cite the files"
    );
}

#[test]
fn a_truths_and_a_prohibitions_verdict_must_cite_code() {
    let values = BTreeMap::new();
    let file = |f: &str| Cite::File {
        file: f.to_string(),
    };
    let none: &[Cite] = &[];
    let outside = [file("../outside.js")];
    let entity = [Cite::Entity {
        entity: "v1.0/01".to_string(),
    }];
    let inside = [file("src/tasks.js")];
    let test = [test("test/tasks.test.js#adds")];
    for (code, kind, outcomes) in [
        ("E501", "truth", ["verified", "failed"]),
        ("W525", "prohibition", ["held", "violated"]),
    ] {
        // Refused whatever the outcome.
        for outcome in outcomes {
            for cited in [none, &outside, &entity] {
                let why = cites_code(&proposed(code, kind, outcome, &values, cited))
                    .expect_err("refused");
                assert_eq!(
                    why,
                    format!(
                        "cited: a verdict on a {kind} must cite a file of the project or a test, or it can \
                         never go stale on a code edit"
                    )
                );
            }
            for cited in [&inside[..], &test] {
                assert!(cites_code(&proposed(code, kind, outcome, &values, cited)).is_ok());
            }
        }
    }
    // An artifact and a key link read their own files; another rule binds no citation.
    for (code, kind) in [("E501", "artifact"), ("E501", "key_link"), ("W311", "plan")] {
        assert!(cites_code(&proposed(code, kind, "verified", &values, none)).is_ok());
    }
}
