//! `specforge explain` explains a code the project's loaded extensions catalog (ADR 0286).

use crate::support::{Cli, TestProject};
use serde_json::json;
use specforge_test_macros::test as specforge_test;

/// A project loading the SDK's worked example `@sdk/greet`, whose rule E901 checks a greeting's style (its code `sdk.greet/E901`).
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
    verify = "in a project, specforge explain explains a code its loaded extensions catalog: the code, its title, its level and its owner"
)]
fn explain_explains_a_code_the_projects_extension_catalogs() {
    let project = greet_project();
    let cli = Cli::on(&project);
    let run = cli
        .run(&["explain", "SDK.GREET/e901", "--format", "json"])
        .exits(0);
    let entry = run.json();
    assert_eq!(entry["code"], "sdk.greet/E901", "{entry}");
    assert_eq!(entry["owner"], "@sdk/greet", "{entry}");
    assert_eq!(entry["level"], "error", "{entry}");
    assert_eq!(entry["retired"], false, "{entry}");
    assert!(
        entry["title"].as_str().is_some_and(|t| !t.is_empty()),
        "{entry}"
    );
    let text = cli.run(&["explain", "sdk.greet/E901"]).exits(0).stdout;
    for line in ["sdk.greet/E901: ", "Owner: @sdk/greet", "Level: error"] {
        assert!(text.contains(line), "{line} in {text}");
    }
    // A bare code is not the extension's: it is refused, naming the namespaced form.
    let refused = cli.run(&["explain", "E901"]).exits(1).stderr;
    assert!(refused.contains("error[unknown_code]"), "{refused}");
    assert!(
        refused.contains("did you mean sdk.greet/E901?"),
        "{refused}"
    );
}

#[specforge_test(
    behavior = "explain_diagnostic_code",
    verify = "outside a project specforge explain answers from the catalog alone: a code only an extension catalogs is unknown_code"
)]
fn outside_its_project_a_catalogued_code_is_unknown() {
    let loose = TestProject::new().without_config();
    let stderr = Cli::on(&loose)
        .bare(&["explain", "sdk.greet/E901"])
        .exits(1)
        .stderr;
    assert!(stderr.contains("error[unknown_code]"), "{stderr}");
    assert!(
        stderr.contains("inside a project that loads the extension"),
        "{stderr}"
    );
}
