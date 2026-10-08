// The separate install and uninstall steps, through the public API:
// - B:install_wasm_extension

use specforge_installed::legacy::{install_extension, uninstall_extension};
use specforge_installed::{Installed, LockFile, LockState, hex_sha256};
use tempfile::TempDir;

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
