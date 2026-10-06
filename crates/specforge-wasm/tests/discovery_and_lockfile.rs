// Extension specifiers, the lock file and doctor checks.

use specforge_wasm::{
    DoctorStatus, ExtensionSpecifier, LockFile, LockFileEntry, parse_extension_specifier,
    read_lock_file, run_doctor_check, write_lock_file,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ============================================================
// B:parse_extension_specifier
// ============================================================

// B:parse_extension_specifier — verify integration "parses @scope/name@version registry specifier"
#[test]
fn parse_registry_specifier() {
    let spec = parse_extension_specifier("@specforge/software@1.0.0").unwrap();
    assert_eq!(
        spec,
        ExtensionSpecifier::Registry {
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
        }
    );
}

// B:parse_extension_specifier — verify integration "parses ./local/path specifier"
#[test]
fn parse_local_specifier() {
    let spec = parse_extension_specifier("./local/path").unwrap();
    assert_eq!(
        spec,
        ExtensionSpecifier::Local {
            path: PathBuf::from("./local/path"),
        }
    );
}

// B:parse_extension_specifier — verify integration "parses git+https with optional #rev"
#[test]
fn parse_git_specifier_with_rev() {
    let spec = parse_extension_specifier("git+https://github.com/org/ext#v2.0.0").unwrap();
    assert_eq!(
        spec,
        ExtensionSpecifier::Git {
            url: "https://github.com/org/ext".to_string(),
            rev: Some("v2.0.0".to_string()),
        }
    );

    let no_rev = parse_extension_specifier("git+https://github.com/org/ext").unwrap();
    assert_eq!(
        no_rev,
        ExtensionSpecifier::Git {
            url: "https://github.com/org/ext".to_string(),
            rev: None,
        }
    );
}

// B:parse_extension_specifier — verify integration "empty/invalid input produces E032"
#[test]
fn parse_invalid_specifier_e054() {
    let err = parse_extension_specifier("").unwrap_err();
    assert_eq!(err.code, "E054");

    let err = parse_extension_specifier("just-a-name").unwrap_err();
    assert_eq!(err.code, "E054");
}

// ============================================================
// B:write_lock_file + B:read_lock_file
// ============================================================

// B:write_lock_file, B:read_lock_file — verify integration "roundtrip write+read produces identical LockFile"
#[test]
fn lock_file_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("specforge.lock");

    let lock = LockFile {
        lockfile_version: 1,
        entries: vec![
            LockFileEntry {
                name: "@specforge/software".to_string(),
                version: "1.0.0".to_string(),
                source: "registry".to_string(),
                wasm_hash: "abc123".to_string(),
                key_id: None,
                peer_dependencies: Vec::new(),
            },
            LockFileEntry {
                name: "@specforge/governance".to_string(),
                version: "2.0.0".to_string(),
                source: "local:./ext".to_string(),
                wasm_hash: "def456".to_string(),
                key_id: None,
                peer_dependencies: Vec::new(),
            },
        ],
    };

    write_lock_file(&lock, &path).unwrap();
    let read_back = read_lock_file(&path).unwrap();
    assert_eq!(lock, read_back);
}

// B:read_lock_file — verify integration "corrupt file produces E033"
#[test]
fn lock_file_corrupt_e033() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("specforge.lock");
    std::fs::write(&path, "not json {{{").unwrap();

    let err = read_lock_file(&path).unwrap_err();
    assert_eq!(err.code, "E033");
    assert!(err.message.contains("corrupt"));
}

// B:read_lock_file — verify integration "missing file produces E033"
#[test]
fn lock_file_missing_e033() {
    let err = read_lock_file(Path::new("/nonexistent/specforge.lock")).unwrap_err();
    assert_eq!(err.code, "E033");
}

// ============================================================
// B:run_doctor_check
// ============================================================

// B:run_doctor_check — verify integration "missing binary detected"
#[test]
fn doctor_missing_binary() {
    let dir = TempDir::new().unwrap();
    let lock = LockFile {
        lockfile_version: 1,
        entries: vec![LockFileEntry {
            name: "missing-ext".to_string(),
            version: "1.0.0".to_string(),
            source: "registry".to_string(),
            wasm_hash: "abc".to_string(),
            key_id: None,
            peer_dependencies: Vec::new(),
        }],
    };
    let results = run_doctor_check(&lock, dir.path(), |_| None, &HashMap::new());
    assert!(
        results
            .iter()
            .any(|r| matches!(r, DoctorStatus::MissingBinary { name } if name == "missing-ext"))
    );
}

// B:run_doctor_check — verify integration "stale hash detected"
#[test]
fn doctor_stale_hash() {
    let dir = TempDir::new().unwrap();
    let ext_dir = dir.path().join("my-ext");
    std::fs::create_dir(&ext_dir).unwrap();
    std::fs::write(ext_dir.join("extension.wasm"), b"content").unwrap();

    let lock = LockFile {
        lockfile_version: 1,
        entries: vec![LockFileEntry {
            name: "my-ext".to_string(),
            version: "1.0.0".to_string(),
            source: "registry".to_string(),
            wasm_hash: "expected_hash".to_string(),
            key_id: None,
            peer_dependencies: Vec::new(),
        }],
    };
    let results = run_doctor_check(
        &lock,
        dir.path(),
        |_| Some("different_hash".to_string()),
        &HashMap::new(),
    );
    assert!(
        results
            .iter()
            .any(|r| matches!(r, DoctorStatus::StaleHash { .. }))
    );
}

// B:run_doctor_check — verify integration "all healthy returns empty"
#[test]
fn doctor_all_healthy() {
    let dir = TempDir::new().unwrap();
    let ext_dir = dir.path().join("good-ext");
    std::fs::create_dir(&ext_dir).unwrap();
    std::fs::write(ext_dir.join("extension.wasm"), b"wasm").unwrap();

    let lock = LockFile {
        lockfile_version: 1,
        entries: vec![LockFileEntry {
            name: "good-ext".to_string(),
            version: "1.0.0".to_string(),
            source: "registry".to_string(),
            wasm_hash: "correct".to_string(),
            key_id: None,
            peer_dependencies: Vec::new(),
        }],
    };
    let installed: HashMap<String, String> = [("good-ext".to_string(), "1.0.0".to_string())]
        .into_iter()
        .collect();
    let results = run_doctor_check(
        &lock,
        dir.path(),
        |_| Some("correct".to_string()),
        &installed,
    );
    assert!(results.is_empty(), "expected healthy, got: {:?}", results);
}
