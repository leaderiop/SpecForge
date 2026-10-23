//! A lock entry must record what its binary declares (ADR 0217): `check` refuses one that does not
//! (E070), and the add of its file rewrites it.

use crate::support::registry::greet_wasm;
use crate::support::{Cli, TestProject};
use serde_json::Value;
use specforge_test_macros::test as specforge_test;

fn greeting_project() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&["@specforge/software"])
        .file("main.spec", "greeting hello \"Hello\" {\n  style warm\n}\n")
}

/// The codes `specforge check` reports, with whether it passed.
fn check(project: &TestProject) -> (bool, Vec<(String, String)>) {
    let out = Cli::on(project)
        .command(&["check", "--format", "json"])
        .output()
        .unwrap();
    let diagnostics: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let found = diagnostics
        .iter()
        .map(|d| {
            let text = |key: &str| d[key].as_str().unwrap_or_default().to_string();
            (text("code"), text("message"))
        })
        .collect();
    (out.status.success(), found)
}

fn lock(project: &TestProject) -> Value {
    serde_json::from_str(&std::fs::read_to_string(project.root().join("specforge.lock")).unwrap())
        .unwrap()
}

#[specforge_test(
    behavior = "load_wasm_module",
    verify = "a lock entry whose binary declares another version or other peers is refused with E070 through check, and add of its file rewrites it"
)]
fn a_lock_entry_its_binary_does_not_declare_is_e070_until_an_add_rewrites_it() {
    let project = greeting_project();
    project.write("greet.wasm", greet_wasm());
    Cli::on(&project)
        .command(&["add", "greet.wasm"])
        .assert()
        .success();
    let mut edited = lock(&project);
    edited["entries"][0]["peer_dependencies"] =
        serde_json::json!([{"name": "@specforge/formal", "version": "^99", "optional": false}]);
    std::fs::write(project.root().join("specforge.lock"), edited.to_string()).unwrap();

    let (ok, found) = check(&project);
    assert!(!ok, "{found:?}");
    let (_, message) = found
        .iter()
        .find(|(code, _)| code == "E070")
        .unwrap_or_else(|| panic!("no E070: {found:?}"));
    assert!(message.contains("@sdk/greet"), "{message}");

    Cli::on(&project)
        .command(&["add", "@specforge/formal", "--dry-run"])
        .assert()
        .success();
    Cli::on(&project)
        .command(&["add", "greet.wasm"])
        .assert()
        .success();
    assert!(
        lock(&project)["entries"][0]
            .get("peer_dependencies")
            .is_none(),
        "{}",
        lock(&project)
    );
    let (ok, found) = check(&project);
    assert!(ok, "{found:?}");
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "doctor never calls the installed binaries healthy beside an E070 of a binary its lock entry does not record"
)]
fn doctor_calls_no_binary_healthy_beside_an_e070() {
    let project = greeting_project();
    project.write("greet.wasm", greet_wasm());
    Cli::on(&project)
        .command(&["add", "greet.wasm"])
        .assert()
        .success();
    let mut edited = lock(&project);
    edited["entries"][0]["version"] = "0.2.0".into();
    std::fs::write(project.root().join("specforge.lock"), edited.to_string()).unwrap();

    let run = Cli::on(&project).run(&["doctor"]);
    assert_eq!(run.code, 1, "{}", run.stdout);
    let from = &run.stdout[run
        .stdout
        .find("Installed binaries")
        .expect("binaries section")..];
    let binaries = &from[..from.find("\n\n").unwrap_or(from.len())];
    assert!(!binaries.contains("healthy"), "{binaries}");
    assert!(
        run.stdout
            .contains("declares version 0.1.0, but its lock entry records 0.2.0"),
        "{}",
        run.stdout
    );
}
