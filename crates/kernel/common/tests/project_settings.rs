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
