//! A `.wasm` file entry importing a host port this specforge lacks is E088 through `check` and `add`, not E028
//! (ADR 0326 D1a).

use crate::support::{Cli, TestProject};
use serde_json::Value;
use specforge_component::fixtures::importing;
use specforge_protocol_types::PROTOCOL_VERSION;
use specforge_test_macros::test as specforge_test;

const NEWER_PORT: &str = "specforge:host/http@9.9.0";

/// A greeting project whose `newer.wasm` imports a host port this specforge does not provide.
fn newer_port_project() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&["@specforge/software"])
        .file("main.spec", "greeting hello \"Hello\" {\n  style warm\n}\n")
        .file("newer.wasm", importing(&[NEWER_PORT], PROTOCOL_VERSION))
}

#[specforge_test(
    behavior = "load_wasm_module",
    verify = "a component importing a host port this specforge does not provide is refused with E088 before it is instantiated, naming the port"
)]
#[ignore = "plan 00 T4"]
fn a_wasm_file_entry_importing_a_host_port_this_specforge_lacks_is_e088() {
    let project = newer_port_project().enabling(&["@specforge/software", "newer.wasm"]);

    let out = Cli::on(&project)
        .command(&["check", "--format", "json"])
        .output()
        .unwrap();

    let diagnostics: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let e088: Vec<&Value> = diagnostics.iter().filter(|d| d["code"] == "E088").collect();
    assert_eq!(e088.len(), 1, "{diagnostics:?}");
    let message = e088[0]["message"].as_str().unwrap();
    assert!(message.contains("'newer.wasm'"), "{message}");
    assert!(message.contains(NEWER_PORT), "{message}");
    assert!(
        message.contains(&format!("extension protocol {PROTOCOL_VERSION}")),
        "{message}"
    );
    let suggestion = e088[0]["suggestion"].as_str().unwrap();
    assert!(suggestion.starts_with("update specforge"), "{suggestion}");
}

#[specforge_test(
    behavior = "load_wasm_module",
    verify = "a component importing a host port this specforge does not provide is refused with E088 before it is instantiated, naming the port"
)]
#[ignore = "plan 00 T4"]
fn add_refuses_a_component_importing_a_host_port_this_specforge_lacks_with_e088() {
    let project = newer_port_project();
    let config = std::fs::read(project.root().join("specforge.json")).unwrap();
    let lock = std::fs::read(project.root().join("specforge.lock")).ok();

    let out = Cli::on(&project)
        .command(&["add", "./newer.wasm"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error[E088]"), "{stderr}");
    assert_eq!(
        std::fs::read(project.root().join("specforge.json")).unwrap(),
        config
    );
    assert_eq!(
        std::fs::read(project.root().join("specforge.lock")).ok(),
        lock
    );
    assert!(!project.root().join(".specforge/extensions").exists());
}
