// A lock entry is read whole where the lock is read: a version, hash, key id, peer or source the lock
// cannot record makes the whole lock unreadable (E033), shown escaped (ADR 0243).

use std::path::Path;

use serde_json::{Value, json};
use specforge_common::remedy::printable;
use specforge_installed::Installed;
use tempfile::TempDir;

/// A one-entry lock at `root` whose entry has `field` set to `value`.
fn lock_with(root: &Path, field: &str, value: Value) {
    let mut entry = json!({
        "name": "@sdk/greet",
        "version": "0.1.0",
        "source": "local:greet.wasm",
        "wasm_hash": "0".repeat(64),
    });
    entry[field] = value;
    let lock = json!({"lockfile_version": 1, "entries": [entry]});
    std::fs::write(root.join("specforge.lock"), lock.to_string()).unwrap();
}

/// The lock at `root` is unreadable (E033) and its message names `field`.
fn refused(root: &Path, field: &str, shown: &Value) {
    let state = Installed::at(root).lock().clone();
    let problem = state
        .problem()
        .unwrap_or_else(|| panic!("{field} {shown}: {state:?}"));
    assert_eq!(problem.code, "E033", "{field} {shown}: {problem:?}");
    assert!(problem.message.contains("corrupt lock file"), "{problem:?}");
    assert!(
        problem.message.contains(field),
        "{field} {shown}: {problem:?}"
    );
    assert_eq!(problem.message, printable(&problem.message), "{problem:?}");
    assert!(state.entries().is_empty(), "{field} {shown}");
}

#[specforge_test_macros::test(
    behavior = "read_lock_file",
    verify = "a lock entry whose version is no SemVer version is E033, and the message shows no control character"
)]
#[ignore = "flipped by T4"]
fn a_lock_entry_whose_version_is_no_semver_version_is_unreadable() {
    let dir = TempDir::new().unwrap();
    for version in [
        "0.1.0\u{1b}]0;PWNED\u{7}",
        "1.0",
        "latest",
        "v1.0.0",
        "0.1.x",
        "",
    ] {
        lock_with(dir.path(), "version", version.into());
        refused(dir.path(), "version", &version.into());
    }
}

#[specforge_test_macros::test(
    behavior = "read_lock_file",
    verify = "a lock entry whose hash is no SHA-256 in lowercase hex is E033, an empty one included"
)]
fn a_lock_entry_whose_hash_is_no_lowercase_sha256_is_unreadable() {
    let dir = TempDir::new().unwrap();
    let upper = "ACC6D3776A303907FA229BCB80A8520406800A0DE2AA807B89DBAF5A3BE09F60";
    for hash in [
        "not-a-hash\u{1b}[2J".to_string(),
        "00".into(),
        "aaaaaaaé".into(),
        "a".repeat(63),
        "a".repeat(65),
        upper.into(),
        String::new(),
    ] {
        lock_with(dir.path(), "wasm_hash", hash.clone().into());
        refused(dir.path(), "wasm_hash", &hash.into());
    }
}

#[specforge_test_macros::test(
    behavior = "read_lock_file",
    verify = "a lock entry whose key id is no publisher key id is E033"
)]
#[ignore = "flipped by T5"]
fn a_lock_entry_whose_key_is_no_publisher_key_id_is_unreadable() {
    let dir = TempDir::new().unwrap();
    for key in [
        "\u{1b}]0;PWNED\u{7}abcd".to_string(),
        "k".into(),
        "in-memory".into(),
        "ABCDEF0123456789".into(),
        "a".repeat(15),
        "a".repeat(17),
        String::new(),
    ] {
        lock_with(dir.path(), "key_id", key.clone().into());
        refused(dir.path(), "key_id", &key.into());
    }
}

#[specforge_test_macros::test(
    behavior = "read_lock_file",
    verify = "a lock entry whose recorded peer or local source holds a character a terminal acts on is E033"
)]
#[ignore = "flipped by T6"]
fn a_lock_entry_whose_peer_or_local_source_holds_an_unprintable_character_is_unreadable() {
    let dir = TempDir::new().unwrap();
    for peer in [
        json!({"name": "@x/y\u{1b}[2J", "version": "^1"}),
        json!({"name": "@x/y", "version": "^1\u{7}"}),
        json!({"name": "@x/y\u{2066}", "version": "^1"}),
    ] {
        lock_with(dir.path(), "peer_dependencies", json!([peer]));
        refused(dir.path(), "peer", &peer);
    }
    lock_with(dir.path(), "source", "local:gr\u{202e}eet.wasm".into());
    refused(dir.path(), "source", &"bidi".into());
}
