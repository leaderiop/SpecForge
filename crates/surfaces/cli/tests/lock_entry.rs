//! A lock entry is read whole where the lock is read, so no command writes a character a terminal acts on
//! that a cloned `specforge.lock` holds, and none crashes on it (ADR 0243). Process tests.

use crate::support::registry::{Package, greet_wasm, project_on, serve};
use crate::support::{Cli, TestProject};
use serde_json::{Value, json};
use specforge_test_macros::test as specforge_test;

const HASH: &str = "acc6d3776a303907fa229bcb80a8520406800a0de2aa807b89dbaf5a3be09f60";
const ESC: &str = "\u{1b}]0;PWNED\u{7}\u{1b}[2J";

/// A project with greet installed and a lock whose one entry has `field` set to `value`.
fn locked(field: &str, value: Value) -> TestProject {
    let project = TestProject::new().enabling(&["@sdk/greet"]).file(
        ".specforge/extensions/@sdk/greet/extension.wasm",
        greet_wasm(),
    );
    let mut entry = json!({
        "name": "@sdk/greet", "version": "0.1.0", "source": "local:greet.wasm", "wasm_hash": HASH,
    });
    entry[field] = value;
    project.write(
        "specforge.lock",
        json!({"lockfile_version": 1, "entries": [entry]}).to_string(),
    );
    project
}

/// Whether `text` holds a character a terminal acts on, its own line ends aside.
fn acts(text: &str) -> bool {
    let text = text.replace('\n', "");
    text != specforge_common::remedy::printable(&text)
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "doctor reports a lock entry whose hash is not ASCII as an unreadable lock, without crashing"
)]
fn doctor_reports_a_lock_hash_that_is_not_ascii_as_unreadable() {
    let project = locked("wasm_hash", json!("aaaaaaaé"));

    let run = Cli::on(&project).run(&["doctor"]);
    let document = Cli::on(&project).document(&["doctor"]);

    assert_ne!(run.code, 101, "{}", run.stderr);
    assert!(!run.stderr.contains("panicked"), "{}", run.stderr);
    assert!(
        document.json.to_string().contains("E033"),
        "{}",
        document.json
    );
}

#[specforge_test(
    behavior = "install_wasm_extension",
    verify = "an add over a lock entry whose key id holds a control character is refused with E033, written escaped"
)]
fn add_names_no_control_character_a_lock_key_holds() {
    let registry = serve(vec![Package::new(
        "@sdk/greet",
        "0.2.0",
        b"\0asm greet 0.2.0".to_vec(),
    )]);
    let project = project_on(&registry);
    let entry = json!({
        "name": "@sdk/greet", "version": "0.1.0", "source": "registry", "wasm_hash": HASH,
        "key_id": format!("{ESC}abcd"),
    });
    project.write(
        "specforge.lock",
        json!({"lockfile_version": 1, "entries": [entry]}).to_string(),
    );

    let run = Cli::on(&project).run(&["add", "@sdk/greet@0.2.0", "--allow-unsigned"]);
    let document = Cli::on(&project).document(&["add", "@sdk/greet@0.2.0", "--allow-unsigned"]);

    assert!(!acts(&run.stdout) && !acts(&run.stderr), "{run:?}");
    assert_eq!(document.json["code"], "E033", "{}", document.json);
    assert!(
        registry
            .requests()
            .iter()
            .all(|r| !r.ends_with("/download")),
        "{:?}",
        registry.requests()
    );
}

#[specforge_test(
    behavior = "read_lock_file",
    verify = "check, extensions and doctor write no control character a lock entry's version, hash or peers hold"
)]
#[ignore = "flipped by T6"]
fn check_extensions_and_doctor_write_no_control_character_a_lock_entry_holds() {
    for (field, value) in [
        ("version", json!(format!("0.1.0{ESC}"))),
        ("wasm_hash", json!(format!("not-a-hash{ESC}"))),
        (
            "peer_dependencies",
            json!([{"name": format!("@x/y{ESC}"), "version": format!("^1{ESC}")}]),
        ),
        ("source", json!("local:gr\u{202e}eet.wasm")),
    ] {
        let project = locked(field, value);
        for args in [&["check"][..], &["extensions"], &["doctor"]] {
            let run = Cli::on(&project).run(args);
            assert!(
                !acts(&run.stdout) && !acts(&run.stderr),
                "{field} {args:?}: {run:?}"
            );
        }
        let document = Cli::on(&project).document(&["check"]);
        assert!(
            document.json.to_string().contains("E033"),
            "{field}: {}",
            document.json
        );
    }
}
