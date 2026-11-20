//! `specforge.json` `settings` is read by the project config (ADR 0288), and so is `facts.retain` (ADR 0376 D4).

use std::path::Path;

use specforge_common::{ConfigProblem, FactsConfig, parse_project_config};

#[specforge_test_macros::test(requirement = "FEATURE-028")]
fn a_settings_value_that_is_not_an_object_is_e069() {
    let read = parse_project_config(
        Path::new("specforge.json"),
        r#"{"name": "p", "settings": [], "extensions": ["@a/b"]}"#,
    );
    assert_eq!(
        read.problems,
        [ConfigProblem::WrongType {
            path: "specforge.json".into(),
            key: "settings",
            expected: "an object",
        }]
    );
    assert_eq!(read.config.name.as_deref(), Some("p"));
    assert_eq!(read.config.extensions, ["@a/b"]);
    assert!(!read.problems[0].loads_nothing());
    assert!(
        read.problems[0]
            .diagnostic()
            .message
            .contains("\"settings\"")
    );
    let ok = parse_project_config(Path::new("specforge.json"), r#"{"settings": {}}"#);
    assert!(ok.problems.is_empty());
    assert!(ok.config.text.as_deref().unwrap().contains("settings"));
}

#[test]
fn effects_or_roots_that_is_not_an_object_is_e069() {
    let read = parse_project_config(
        Path::new("specforge.json"),
        r#"{"effects": "off", "roots": [], "extensions": ["@a/b"]}"#,
    );
    let keys: Vec<&str> = read
        .problems
        .iter()
        .map(|p| match p {
            ConfigProblem::WrongType { key, .. } => *key,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(keys, ["effects", "roots"]);
    assert_eq!(read.config.extensions, ["@a/b"]);
}

#[test]
fn a_checks_key_that_is_not_an_object_is_e069() {
    let wrong = |text: &str| -> Vec<&'static str> {
        parse_project_config(Path::new("specforge.json"), text)
            .problems
            .iter()
            .map(|p| match p {
                ConfigProblem::WrongType { key, .. } => *key,
                other => panic!("{other:?}"),
            })
            .collect()
    };
    assert_eq!(wrong(r#"{"checks": []}"#), ["checks"]);
    assert_eq!(
        wrong(r#"{"checks": {"publish": "yes"}}"#),
        ["checks.publish"]
    );
    assert_eq!(wrong(r#"{"checks": {"fields": []}}"#), ["checks.fields"]);
    assert_eq!(
        wrong(r#"{"checks": {"fields": {"a.b": 1}}}"#),
        ["checks.fields"]
    );
    assert_eq!(
        wrong(r#"{"checks": {"fields": {"a.b": ["x", 1]}}}"#),
        ["checks.fields"]
    );

    let read = parse_project_config(
        Path::new("specforge.json"),
        r#"{"checks": {"publish": true, "fields": {
            "service.docs": [{"checker": "file", "args": {"root": "spec"}}, "anchor"],
            "service.health": "url"
        }}}"#,
    );
    assert!(read.problems.is_empty(), "{:?}", read.problems);
    assert!(read.config.checks.publish);
    let fields = &read.config.checks.fields;
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].0, "service.docs");
    assert_eq!(fields[0].1.len(), 2);
    assert_eq!(fields[1].1.len(), 1);
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn facts_retain_reads_an_integer_from_1_to_100_and_defaults_to_8() {
    let retain = |text: &str| {
        let read = parse_project_config(Path::new("specforge.json"), text);
        assert!(read.problems.is_empty(), "{text}: {:?}", read.problems);
        read.config.facts.retain
    };
    assert_eq!(FactsConfig::default().retain, 8);
    assert_eq!(FactsConfig::DEFAULT_RETAIN, 8);
    assert_eq!(FactsConfig::MAX_RETAIN, 100);
    assert_eq!(retain("{}"), 8);
    assert_eq!(retain(r#"{"facts": {}}"#), 8);
    assert_eq!(retain(r#"{"facts": {"retain": 1}}"#), 1);
    assert_eq!(retain(r#"{"facts": {"retain": 100}}"#), 100);
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn a_facts_retain_out_of_range_or_not_an_integer_is_e069_and_the_default() {
    for written in [
        "0",
        "101",
        "-1",
        "2.5",
        r#""8""#,
        "null",
        "1e3",
        "18446744073709551616",
    ] {
        let text = format!(r#"{{"name": "p", "facts": {{"retain": {written}}}}}"#);
        let read = parse_project_config(Path::new("specforge.json"), &text);
        assert_eq!(
            read.problems,
            [ConfigProblem::WrongType {
                path: "specforge.json".into(),
                key: "facts.retain",
                expected: "an integer from 1 to 100",
            }],
            "{written}"
        );
        assert_eq!(read.config.facts.retain, 8, "{written}");
        assert_eq!(
            read.config.name.as_deref(),
            Some("p"),
            "the other keys are kept"
        );
        assert!(!read.problems[0].loads_nothing());
        assert_eq!(read.problems[0].diagnostic().code, "E069");
    }
    for written in ["[]", "8", r#""x""#] {
        let text = format!(r#"{{"facts": {written}}}"#);
        let read = parse_project_config(Path::new("specforge.json"), &text);
        assert_eq!(
            read.problems,
            [ConfigProblem::WrongType {
                path: "specforge.json".into(),
                key: "facts",
                expected: "an object",
            }],
            "{written}"
        );
        assert_eq!(read.config.facts.retain, 8, "{written}");
    }
}
