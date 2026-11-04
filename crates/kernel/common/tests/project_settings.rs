//! `specforge.json` `settings` is read by the project config (ADR 0288).

use std::path::Path;

use specforge_common::{ConfigProblem, parse_project_config};

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "settings that is not an object in specforge.json is E069 and the rest of the file is kept"
)]
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
