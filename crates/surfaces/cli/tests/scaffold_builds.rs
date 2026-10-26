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
