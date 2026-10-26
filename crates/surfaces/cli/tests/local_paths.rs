//! A local install is read from the project root on the command line as over MCP, so the reinstall command a
//! check suggests runs from any directory of the project (ADR 0272 D5).

use crate::support::registry::greet_wasm;
use crate::support::{Cli, TestProject};
use serde_json::json;
use specforge_test_macros::test as specforge_test;

#[cfg(unix)]
#[specforge_test(
    behavior = "run_doctor_check",
    verify = "the reinstall command a local install suggests runs from any directory of the project"
)]
fn a_local_install_is_reinstalled_by_its_suggestion_from_a_subdirectory() {
    let project = TestProject::new()
        .config(|c| c["extensions"] = json!([]))
        .file("ext/greet.wasm", greet_wasm())
        .file("sub/keep", "");
    Cli::on(&project).run(&["add", "./ext/greet.wasm"]).exits(0);
    std::fs::remove_file(
        project
            .root()
            .join(".specforge/extensions/@sdk/greet/extension.wasm"),
    )
    .unwrap();
    let sub = project.root().join("sub");

    let check = Cli::on(&project)
        .in_dir(&sub)
        .bare(&["check", "--format", "json"]);
    let document = check.json();
    let suggestion = document[0]["suggestion"].as_str().unwrap();
    let argument = suggestion
        .strip_prefix("install the extension with: specforge add ")
        .unwrap();

    Cli::on(&project)
        .in_dir(&sub)
        .bare(&["add", argument])
        .exits(0);
    Cli::on(&project).in_dir(&sub).bare(&["check"]).exits(0);
}
