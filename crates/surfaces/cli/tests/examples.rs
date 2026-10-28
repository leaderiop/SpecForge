//! The shipped examples and a project of every builtin check as they did before protocol 1.11.0 (ADR 0293 to 0296).
//!
//! Each golden under `golden/examples/` was captured with `specforge check --format json` of the binary built at
//! `655208c10`, the commit before kernel round 35, never from this branch: a golden recorded after the change would
//! prove nothing. The builtins embedded here are the 1.10.0 blobs until the round is re-vendored, so this proves the
//! host's half (the new inputs, `values` and the clock leave an extension that declares nothing as it was); the
//! coordinator runs it again after the re-vendor for the guests' half.

use std::path::Path;

use serde_json::{Value, json};

use crate::support::{Cli, TestProject};

fn golden(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/golden/examples/{name}.check.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// What `specforge check --format json` answers on `project`: its exit code and its diagnostics.
fn checked(project: &TestProject) -> Value {
    let run = Cli::on(project).document(&["check"]);
    json!({"exit_code": run.code, "diagnostics": run.json})
}

#[test]
fn every_example_checks_byte_identically() {
    for name in [
        "marketing",
        "product-launch",
        "regulated-process",
        "shop",
        "spec-driven-method",
        "todo-app",
    ] {
        let project =
            TestProject::copy_of(specforge_repo_root::repo_root().join("examples").join(name));
        assert_eq!(
            checked(&project),
            golden(name),
            "specforge check on examples/{name}"
        );
    }
}

#[test]
fn a_project_of_all_nine_builtins_checks_identically() {
    let project = TestProject::new()
        .enabling(&[
            "@specforge/product",
            "@specforge/software",
            "@specforge/governance",
            "@specforge/formal",
            "@specforge/testing",
            "@specforge/cargo-test",
            "@specforge/vitest",
            "@specforge/rust",
            "@specforge/typescript",
        ])
        .file(
            "main.spec",
            "behavior login \"Login\" {\n  contract \"The system MUST sign a user in\"\n  verify unit \"a known user signs in\"\n}\n",
        );

    assert_eq!(checked(&project), golden("all_builtins"));
}
