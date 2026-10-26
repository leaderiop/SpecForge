//! A scaffold builds where it is written: inside another cargo workspace, and
//! with a clear refusal when its SDK does not resolve (ADR 0277).

use crate::support::{Cli, TestProject};
use specforge_test_macros::test as specforge_test;
use std::fs;

#[specforge_test(
    behavior = "scaffold_wasm_extension_project",
    verify = "a scaffold inside a cargo workspace is a workspace of its own"
)]
fn a_scaffold_inside_a_cargo_workspace_is_a_workspace_of_its_own() {
    let dir = TestProject::new().without_config();
    fs::write(dir.root().join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    let init = Cli::on(&dir)
        .bare_command(&[
            "extension",
            "init",
            "--name",
            "@x/inner",
            "--format",
            "json",
            "--dir",
        ])
        .arg(dir.root())
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stdout)
    );
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    // locate-project loads manifests only: no network, no wasm target, no SDK build.
    let located = std::process::Command::new(cargo)
        .args([
            "locate-project",
            "--workspace",
            "--offline",
            "--message-format",
            "plain",
            "--manifest-path",
        ])
        .arg(dir.root().join("inner/Cargo.toml"))
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(
        located.status.success(),
        "{}",
        String::from_utf8_lossy(&located.stderr)
    );
    assert!(
        String::from_utf8_lossy(&located.stdout)
            .trim_end()
            .ends_with("inner/Cargo.toml")
    );
}

#[specforge_test(
    behavior = "build_wasm_extension",
    verify = "a build whose SDK does not resolve says so and names how to build against an unreleased SDK"
)]
fn a_build_whose_sdk_does_not_resolve_names_the_remedy() {
    let dir = TestProject::new().without_config();
    let krate = dir.root().join("needs-sdk");
    fs::create_dir_all(krate.join("src")).unwrap();
    // A version no registry will have, whatever crates.io holds.
    fs::write(
        krate.join("Cargo.toml"),
        "[package]\nname = \"needs-sdk\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n\n\
         [lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\nspecforge-extension-sdk = \"999.0.0\"\n",
    )
    .unwrap();
    fs::write(krate.join("src/lib.rs"), "").unwrap();
    let output = Cli::on(&dir)
        .with_toolchain()
        .env("CARGO_NET_OFFLINE", "true")
        .bare_command(&["extension", "build", "--format", "json", "--dir"])
        .arg(&krate)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&output.stdout)));
    assert_eq!(json["code"], "E040", "{json}");
    let error = json["error"].as_str().unwrap();
    assert!(
        error.starts_with("specforge-extension-sdk does not resolve in "),
        "{error}"
    );
    let suggestion = json["suggestion"].as_str().unwrap_or_default();
    assert!(suggestion.contains("[patch.crates-io]"), "{json}");
    assert!(
        suggestion.contains("Building against an unreleased SDK"),
        "{json}"
    );
    let docs =
        fs::read_to_string(specforge_repo_root::repo_root().join("docs/extension-sdk.md")).unwrap();
    assert!(
        docs.lines()
            .any(|l| l == "### Building against an unreleased SDK")
    );
}
