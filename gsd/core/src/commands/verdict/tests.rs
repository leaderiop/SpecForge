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

fn proposed<'a>(
    code: &str,
    kind: Option<&'a str>,
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
    }
}

#[test]
fn only_a_truths_e501_verdict_must_cite_code() {
    let values = BTreeMap::new();
    let file = |f: &str| Cite::File {
        file: f.to_string(),
    };
    let none: &[Cite] = &[];
    let outside = [file("../outside.js")];
    let entity = [Cite::Entity {
        entity: "v1.0/01".to_string(),
    }];
    for (cited, outcome) in [
        (none, "verified"),
        (&outside, "failed"),
        (&entity, "verified"),
    ] {
        let why = truth_cites(&proposed("E501", Some("truth"), outcome, &values, cited))
            .expect_err("refused");
        assert!(
            why.contains("must cite a file of the project or a test"),
            "{why}"
        );
    }
    let inside = [file("src/tasks.js")];
    let test = [Cite::Test {
        test: "test/tasks.test.js#adds".to_string(),
    }];
    for cited in [&inside[..], &test] {
        assert!(truth_cites(&proposed("E501", Some("truth"), "verified", &values, cited)).is_ok());
    }
    // An artifact and a key link read their own files; another rule binds no citation.
    for (code, kind) in [("E501", "artifact"), ("E501", "key_link"), ("W311", "plan")] {
        assert!(truth_cites(&proposed(code, Some(kind), "verified", &values, none)).is_ok());
    }
}
