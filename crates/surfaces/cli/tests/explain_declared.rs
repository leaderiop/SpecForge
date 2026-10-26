//! `specforge explain` explains a code the project's loaded extensions declare (ADR 0278).

use crate::support::{Cli, TestProject};
use serde_json::json;
use specforge_test_macros::test as specforge_test;

/// A project loading the SDK's worked example `@sdk/greet`, whose rule E901 checks a greeting's style.
fn greet_project() -> TestProject {
    let project = TestProject::new().without_config();
    project.write(
        "specforge.json",
        json!({"name": "p", "version": "0.1.0", "spec_root": "spec", "extensions": ["greet.wasm"]})
            .to_string(),
    );
    project.write("greet.wasm", specforge_component::committed::GREET.read());
    project
}

#[specforge_test(
    behavior = "explain_diagnostic_code",
    verify = "in a project, specforge explain explains a code its loaded extensions' rules declare and the catalog does not have: the code, its level and each rule with its extension"
)]
fn explain_explains_a_code_the_projects_extension_declares() {
    let project = greet_project();
    let cli = Cli::on(&project);
    let run = cli.run(&["explain", "e901", "--format", "json"]).exits(0);
    assert_eq!(
        run.json(),
        json!({
            "code": "E901",
            "level": "error",
            "retired": false,
            "rules": [{
                "extension": "@sdk/greet",
                "target_kind": "greeting",
                "check": "field_value_constraint",
                "message": "greeting '{id}' has unknown style",
            }],
        })
    );
    let text = cli.run(&["explain", "E901"]).exits(0).stdout;
    for line in [
        "E901: declared by @sdk/greet",
        "Rule: field_value_constraint on greeting: greeting '{id}' has unknown style",
        "Owner: @sdk/greet",
        "Level: error",
    ] {
        assert!(text.contains(line), "{line} in {text}");
    }
    assert!(!text.contains("Docs:"), "{text}");
}

#[specforge_test(
    behavior = "explain_diagnostic_code",
    verify = "outside a project specforge explain answers from the catalog alone: a code only an extension declares is unknown_code"
)]
fn outside_its_project_a_declared_code_is_unknown() {
    let loose = TestProject::new().without_config();
    let stderr = Cli::on(&loose).bare(&["explain", "E901"]).exits(1).stderr;
    assert!(stderr.contains("error[unknown_code]"), "{stderr}");
    assert!(
        stderr.contains("inside a project that loads the extension"),
        "{stderr}"
    );
}
