//! Installed (non-builtin) extensions load from the lock file (plan 03,
//! O4.4, ADR 0004 D3-b). Until this, no third-party extension had ever
//! loaded through `check`: the loader keyed modules by file stem while
//! compile looked them up by name, and nothing read the lock.

use crate::fake_registry::{FakeRegistry, Package};
use crate::registry::{greet_wasm, project_on};
use serde_json::{Value, json};
use specforge_test_macros::test as specforge_test;
use std::path::Path;
use tempfile::TempDir;

/// A spec using `greeting`, the kind only `@sdk/greet` defines.
const GREETING: &str = "greeting hello \"Hello\" {\n  style warm\n}\n";

fn specforge() -> assert_cmd::Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

/// A project enabling `@specforge/software` with a spec that uses the
/// `greeting` kind.
fn greeting_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        json!({"name": "p", "version": "0.1.0", "extensions": ["@specforge/software"]}).to_string(),
    )
    .unwrap();
    std::fs::write(dir.path().join("main.spec"), GREETING).unwrap();
    dir
}

fn add_local_greet(root: &Path) {
    let wasm = root.join("greet.wasm");
    std::fs::write(&wasm, greet_wasm()).unwrap();
    specforge()
        .args(["add", "greet.wasm", "--path"])
        .arg(root)
        .current_dir(root)
        .assert()
        .success();
}

/// Every diagnostic a fresh `specforge check` reports, as `(code, message,
/// suggestion)`, with its exit status.
fn check(root: &Path) -> (bool, Vec<(String, String, String)>) {
    let out = specforge()
        .args(["check", "--format", "json"])
        .arg(root)
        .output()
        .unwrap();
    let diagnostics: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let found = diagnostics
        .iter()
        .map(|d| {
            let text = |key: &str| d[key].as_str().unwrap_or_default().to_string();
            (text("code"), text("message"), text("suggestion"))
        })
        .collect();
    (out.status.success(), found)
}

fn codes(found: &[(String, String, String)]) -> Vec<&str> {
    found.iter().map(|(code, _, _)| code.as_str()).collect()
}

fn lock_entry(root: &Path, name: &str) -> Value {
    let lock: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("specforge.lock")).unwrap())
            .unwrap();
    lock["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == name)
        .cloned()
        .unwrap_or_else(|| panic!("no lock entry for {name}: {lock}"))
}

fn enabled(root: &Path) -> Value {
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("specforge.json")).unwrap())
            .unwrap();
    config["extensions"].clone()
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "installed extension manifest is loaded"
)]
fn a_local_install_loads_through_check() {
    let dir = greeting_project();
    let (_, before) = check(dir.path());
    assert!(codes(&before).contains(&"E024"), "{before:?}");

    add_local_greet(dir.path());

    let (ok, after) = check(dir.path());
    assert!(ok, "{after:?}");
    // `greeting` is a known kind now, and nothing failed to load.
    for code in ["E024", "E028", "E033"] {
        assert!(!codes(&after).contains(&code), "{code}: {after:?}");
    }
}

#[specforge_test(
    behavior = "add_extension_to_existing_project",
    verify = "a local .wasm install is locked at its declared version with source local:<path> and enabled by its bare name"
)]
fn a_local_install_is_locked_at_its_declared_version() {
    let dir = greeting_project();

    add_local_greet(dir.path());

    // The handshake's name and version, not the file stem or a placeholder.
    let entry = lock_entry(dir.path(), "@sdk/greet");
    assert_eq!(entry["version"], "0.1.0", "{entry}");
    assert_eq!(entry["source"], "local:greet.wasm", "{entry}");
    assert_eq!(
        enabled(dir.path()),
        json!(["@specforge/software", "@sdk/greet"])
    );
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "an extension installed from a registry loads through check"
)]
fn a_registry_install_loads_through_check() {
    let registry = FakeRegistry::serve(vec![Package::new("@sdk/greet", "0.1.0", greet_wasm())]);
    let dir = project_on(&registry);
    std::fs::write(dir.path().join("main.spec"), GREETING).unwrap();
    let home = TempDir::new().unwrap();

    specforge()
        .args(["add", "@sdk/greet@0.1.0", "--allow-unsigned", "--path"])
        .arg(dir.path())
        .env("HOME", home.path())
        .assert()
        .success();

    assert_eq!(lock_entry(dir.path(), "@sdk/greet")["source"], "registry");
    assert_eq!(
        enabled(dir.path()),
        json!(["@specforge/software", "@sdk/greet"])
    );
    let (ok, found) = check(dir.path());
    assert!(ok, "{found:?}");
    assert!(!codes(&found).contains(&"E024"), "{found:?}");
}

#[specforge_test(
    invariant = "wasm_compile_cache_integrity",
    verify = "tampered installed binary refused via lockfile hash pin (E033)"
)]
fn a_tampered_installed_binary_is_refused_with_e033() {
    let dir = greeting_project();
    add_local_greet(dir.path());
    let installed = dir
        .path()
        .join(".specforge/extensions/@sdk/greet/extension.wasm");
    let mut bytes = std::fs::read(&installed).unwrap();
    bytes.extend_from_slice(b"tampered");
    std::fs::write(&installed, bytes).unwrap();

    let (ok, found) = check(dir.path());

    assert!(!ok, "{found:?}");
    let (_, message, _) = found
        .iter()
        .find(|(code, _, _)| code == "E033")
        .unwrap_or_else(|| panic!("no E033: {found:?}"));
    assert!(message.contains("@sdk/greet"), "{message}");
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "an enabled extension with no installed binary produces E028 naming the command that installs it"
)]
fn an_enabled_extension_that_is_not_installed_is_e028_with_a_remedy() {
    let dir = greeting_project();
    add_local_greet(dir.path());
    // A fresh clone: `.specforge/` is gitignored, the config and lock are not.
    std::fs::remove_dir_all(dir.path().join(".specforge")).unwrap();

    let (ok, found) = check(dir.path());

    assert!(!ok, "{found:?}");
    let (_, message, suggestion) = found
        .iter()
        .find(|(code, _, _)| code == "E028")
        .unwrap_or_else(|| panic!("no E028: {found:?}"));
    assert!(message.contains("@sdk/greet"), "{message}");
    assert!(suggestion.contains("specforge add"), "{suggestion}");
}

#[specforge_test(
    invariant = "init_config_validity",
    verify = "specforge init followed by specforge check produces zero config errors"
)]
fn init_installs_a_local_extension_check_loads() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("greet.wasm"), greet_wasm()).unwrap();

    // D3-e: init installs a local file through the add operation.
    specforge()
        .args(["init", "--name", "hello", "--extensions", "greet.wasm"])
        .current_dir(dir.path())
        .assert()
        .success();
    std::fs::write(dir.path().join("spec/greet.spec"), GREETING).unwrap();

    assert_eq!(enabled(dir.path()), json!(["@sdk/greet"]));
    assert_eq!(
        lock_entry(dir.path(), "@sdk/greet")["source"],
        "local:greet.wasm"
    );
    let (ok, found) = check(dir.path());
    assert!(ok, "{found:?}");
    assert!(!codes(&found).contains(&"E024"), "{found:?}");
}

#[test]
fn a_legacy_versioned_entry_still_loads() {
    let dir = greeting_project();
    add_local_greet(dir.path());
    // What `add` wrote before D3-b.
    std::fs::write(
        dir.path().join("specforge.json"),
        json!({"name": "p", "version": "0.1.0",
               "extensions": ["@specforge/software", "@sdk/greet@0.1.0"]})
        .to_string(),
    )
    .unwrap();

    let (ok, found) = check(dir.path());
    assert!(ok, "{found:?}");
    assert!(!codes(&found).contains(&"E024"), "{found:?}");
}

#[specforge_test(
    behavior = "update_all_extensions",
    verify = "update never replaces a locally installed extension from a registry"
)]
fn update_never_replaces_a_local_install() {
    // The registry publishes a newer package under the same name.
    let registry = FakeRegistry::serve(vec![Package::new("@sdk/greet", "9.9.9", greet_wasm())]);
    let dir = project_on(&registry);
    add_local_greet(dir.path());
    let lock_before = std::fs::read(dir.path().join("specforge.lock")).unwrap();
    let home = TempDir::new().unwrap();

    specforge()
        .args(["update", "--allow-unsigned", "--path"])
        .arg(dir.path())
        .env("HOME", home.path())
        .assert()
        .success();

    assert_eq!(
        std::fs::read(dir.path().join("specforge.lock")).unwrap(),
        lock_before
    );
    assert_eq!(
        registry.hits(),
        0,
        "update asked the registry about a local build"
    );
}

/// `specforge doctor --format json` on `root`: whether it passed, and the
/// report.
fn doctor(root: &Path) -> (bool, Value) {
    let home = TempDir::new().unwrap();
    let out = specforge()
        .args(["doctor", "--format", "json", "--path"])
        .arg(root)
        .env("HOME", home.path())
        .output()
        .unwrap();
    let report = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("doctor is not JSON ({e}): {out:?}"));
    (out.status.success(), report)
}

fn finding<'a>(report: &'a Value, code: &str) -> &'a Value {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == code)
        .unwrap_or_else(|| panic!("no {code} finding: {report}"))
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "doctor reports an extension that fails to load (E028, E033) as an error"
)]
fn doctor_reports_an_extension_that_fails_to_load() {
    // A binary that no longer matches its lock entry: check refuses it (E033).
    let tampered = greeting_project();
    add_local_greet(tampered.path());
    let installed = tampered
        .path()
        .join(".specforge/extensions/@sdk/greet/extension.wasm");
    let mut bytes = std::fs::read(&installed).unwrap();
    bytes.extend_from_slice(b"tampered");
    std::fs::write(&installed, bytes).unwrap();

    let (ok, report) = doctor(tampered.path());
    assert!(!ok, "{report}");
    let e033 = finding(&report, "E033");
    assert_eq!(e033["status"], "error", "{e033}");
    assert!(e033["check"].as_str().unwrap().contains("@sdk/greet"));

    // Enabled, but neither locked nor installed: only the load knows (E028).
    let missing = greeting_project();
    add_local_greet(missing.path());
    std::fs::remove_file(missing.path().join("specforge.lock")).unwrap();
    std::fs::remove_dir_all(missing.path().join(".specforge")).unwrap();

    let (ok, report) = doctor(missing.path());
    assert!(!ok, "{report}");
    assert_eq!(report["status"], "issues_found", "{report}");
    let e028 = finding(&report, "E028");
    assert_eq!(e028["status"], "error", "{e028}");
    assert!(
        e028["remediation"]
            .as_str()
            .unwrap()
            .contains("specforge add @sdk/greet"),
        "{e028}"
    );
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "a peer whose installed version doctor cannot compare is remedied with a runnable command"
)]
fn a_peer_recorded_at_a_non_semver_version_is_remedied_by_reinstalling_it() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        json!({"name": "p", "version": "0.1.0", "extensions": []}).to_string(),
    )
    .unwrap();
    // A lock from before installs recorded their declared version: the
    // local `@sdk/greet` is at "local", which no range can be checked against.
    let wasm = b"module";
    for name in ["@acme/uses-greet", "@sdk/greet"] {
        let installed = dir.path().join(".specforge/extensions").join(name);
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::write(installed.join("extension.wasm"), wasm).unwrap();
    }
    let hash = specforge_wasm::hex_sha256(wasm);
    let lock = json!({
        "lockfile_version": 1,
        "entries": [
            {"name": "@acme/uses-greet", "version": "1.0.0", "source": "registry",
             "wasm_hash": hash,
             "peer_dependencies": [{"name": "@sdk/greet", "version": "^0.1.0"}]},
            {"name": "@sdk/greet", "version": "local", "source": "registry", "wasm_hash": hash},
        ],
    });
    std::fs::write(dir.path().join("specforge.lock"), lock.to_string()).unwrap();

    let (ok, report) = doctor(dir.path());

    assert!(!ok, "{report}");
    let peer = finding(&report, "peer_mismatch");
    let remediation = peer["remediation"].as_str().unwrap();
    assert_eq!(
        remediation,
        "run `specforge add @sdk/greet` to reinstall it"
    );
    let check = peer["check"].as_str().unwrap();
    assert!(check.contains("'local'"), "{check}");
    assert!(check.contains("^0.1.0"), "{check}");
}
