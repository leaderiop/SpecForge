// The separate install, uninstall and load steps, through the public API:
// - B:load_wasm_module (with the lock file's hash pin)
// - B:install_wasm_extension

use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
use specforge_installed::legacy::{install_extension, load_wasm_module, uninstall_extension};
use specforge_installed::{Installed, LockFile, LockState, hex_sha256};
use specforge_wasm::testing::InProcessRuntime;
use tempfile::TempDir;

/// A runtime serving the extensions these tests load, so loading their
/// binaries under those names succeeds.
fn runtime() -> InProcessRuntime {
    let serve =
        |name: &'static str| move || ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    InProcessRuntime::new()
        .with(serve("@test/ext"))
        .with(serve("@test/legacy"))
        .with(serve("@test/local"))
}

fn create_fake_wasm(dir: &TempDir, name: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, b"\x00asm\x01\x00\x00\x00fake").unwrap();
    path
}

// B:load_wasm_module — verify integration "load valid module bytes → Ok"
#[test]
fn test_load_valid_module_loads() {
    let dir = TempDir::new().unwrap();
    let wasm_path = create_fake_wasm(&dir, "ext.wasm");
    let runtime = runtime();

    load_wasm_module("@test/ext", &wasm_path, &runtime, None).unwrap();
}

// B:load_wasm_module — verify integration "load corrupted bytes → Err with E028"
#[test]
fn test_load_missing_wasm_returns_e028() {
    let runtime = runtime();
    let missing = std::path::Path::new("/nonexistent/path/ext.wasm");

    let err = load_wasm_module("@test/missing", missing, &runtime, None).unwrap_err();
    assert_eq!(err.severity, specforge_common::Severity::Error);
    assert!(err.message.contains("not found"));
}

// B:load_wasm_module — verify contract "requires valid bytes, ensures a load or a diagnostic"
#[test]
fn test_load_wasm_module_contract() {
    let dir = TempDir::new().unwrap();
    let wasm_path = create_fake_wasm(&dir, "ext.wasm");
    let runtime = runtime();

    // ensures: success path loads it
    load_wasm_module("@test/ext", &wasm_path, &runtime, None).unwrap();

    // ensures: failure path returns E028 diagnostic
    let err =
        load_wasm_module("bad", std::path::Path::new("/no/such.wasm"), &runtime, None).unwrap_err();
    assert_eq!(err.code, "E028");
    assert_eq!(err.severity, specforge_common::Severity::Error);
}

// B:load_wasm_module — verify unit "lockfile hash pin refuses tampered binary"
#[test]
fn load_refuses_binary_that_differs_from_lockfile_hash() {
    let dir = TempDir::new().unwrap();
    let installed = Installed::with_lock(dir.path(), LockState::Absent);

    let wasm_bytes = b"\0asm-original";
    let mut lock = LockFile::new();
    let name = specforge_protocol_types::PackageName::parse("@test/ext").unwrap();
    install_extension(
        &name,
        "1.0.0",
        wasm_bytes,
        &hex_sha256(wasm_bytes),
        &installed,
        &mut lock,
        None,
        Vec::new(),
    )
    .unwrap();

    let wasm_path = installed.module_path(&name);

    // Load with the recorded hash: succeeds.
    let runtime = runtime();
    load_wasm_module(
        "@test/ext",
        &wasm_path,
        &runtime,
        Some(lock.entries[0].wasm_hash.as_str()),
    )
    .unwrap();

    // Tamper with the installed binary, then load: refused with E033.
    std::fs::write(&wasm_path, b"\0asm-swapped-after-install").unwrap();
    let err = load_wasm_module(
        "@test/ext",
        &wasm_path,
        &runtime,
        Some(lock.entries[0].wasm_hash.as_str()),
    )
    .unwrap_err();
    assert_eq!(err.code, "E033");
    assert!(err.message.contains("integrity mismatch"));
    assert!(err.suggestion.unwrap_or_default().contains("re-install"));
}

// B:load_wasm_module — verify unit "legacy entries without hash load unchanged"
#[test]
fn load_with_empty_or_absent_hash_does_not_fail() {
    let dir = TempDir::new().unwrap();
    let wasm_path = dir.path().join("extension.wasm");
    std::fs::write(&wasm_path, b"\0asm-legacy").unwrap();
    let runtime = runtime();

    // Legacy lockfile entry: empty hash string — warn-and-load, not fail.
    load_wasm_module("@test/legacy", &wasm_path, &runtime, Some("")).unwrap();

    // No hash context at all (local dev load): unchanged behavior.
    load_wasm_module("@test/local", &wasm_path, &runtime, None).unwrap();
}

// B:install_wasm_extension — verify unit "an extension is installed under the extensions directory of its project, by its package name"
#[test]
fn relative_path_is_what_install_joins() {
    use specforge_protocol_types::PackageName;

    let dir = TempDir::new().unwrap();
    let installed = Installed::with_lock(dir.path(), LockState::Absent);
    let name = PackageName::parse("@acme/tool").unwrap();
    let wasm = b"\0asm-tool";
    let mut lock = LockFile::new();

    install_extension(
        &name,
        "1.0.0",
        wasm,
        &hex_sha256(wasm),
        &installed,
        &mut lock,
        None,
        Vec::new(),
    )
    .unwrap();

    let module = dir
        .path()
        .join(".specforge")
        .join("extensions")
        .join("@acme")
        .join("tool")
        .join("extension.wasm");
    assert_eq!(installed.module_path(&name), module);
    assert!(module.is_file());
    // Nothing is written outside the project's `.specforge` directory.
    let outside: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert_eq!(outside, [".specforge"]);

    uninstall_extension(&name, &installed, &mut lock).unwrap();
    assert!(!module.exists());
    assert!(lock.entries.is_empty());
}
