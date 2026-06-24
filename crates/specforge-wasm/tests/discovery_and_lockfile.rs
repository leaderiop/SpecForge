// Slice 18: Discovery, Lock File & Query Extensions Integration Tests
//
// Tests extension discovery, lock file operations, doctor checks,
// lock file refresh, query extension validation, and query file composition.

use specforge_registry::ManifestV2;
use specforge_wasm::{
    compose_query_files, discover_extensions, parse_extension_specifier, read_lock_file,
    refresh_lock_file, run_doctor_check, validate_query_extensions, write_lock_file, DoctorStatus,
    ExtensionSpecifier, LockFile, LockFileEntry, QueryExtension, QueryFileKind,
    RawQueryExtension, ResolvedExtension,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn default_manifest() -> ManifestV2 {
    ManifestV2 {
        name: String::new(),
        version: String::new(),
        manifest_version: 2,
        wasm_path: String::new(),
        contributes: Default::default(),
        entity_kinds: vec![],
        edge_types: vec![],
        fields: vec![],
        validation_rules: vec![],
        verify_kinds: vec![],
        reserved_keywords: vec![],
        peer_dependencies: vec![],
        sandbox_policy: None,
        incremental: None,
        migration_hook: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        grammar_contributions: vec![],
        body_parser_contributions: vec![],
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        analyzer_contributions: vec![],
        surfaces: None,
    }
}

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
fn parse_invalid_specifier_e032() {
    let err = parse_extension_specifier("").unwrap_err();
    assert_eq!(err.code, "E032");

    let err = parse_extension_specifier("just-a-name").unwrap_err();
    assert_eq!(err.code, "E032");
}

// ============================================================
// B:discover_extensions
// ============================================================

// B:discover_extensions — verify integration "directory with valid manifest.json discovered"
#[test]
fn discover_valid_manifest() {
    let dir = TempDir::new().unwrap();
    let ext_dir = dir.path().join("my-ext");
    std::fs::create_dir(&ext_dir).unwrap();
    let manifest = r#"{
        "name": "@test/my-ext",
        "version": "1.0.0",
        "manifestVersion": 2,
        "wasmPath": "ext.wasm"
    }"#;
    std::fs::write(ext_dir.join("manifest.json"), manifest).unwrap();

    let (resolved, diags) = discover_extensions(dir.path());
    assert!(diags.is_empty());
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].manifest.name, "@test/my-ext");
}

// B:discover_extensions — verify integration "invalid manifest produces warning, valid ones still loaded"
#[test]
fn discover_invalid_manifest_warning() {
    let dir = TempDir::new().unwrap();

    // Valid
    let valid_dir = dir.path().join("valid");
    std::fs::create_dir(&valid_dir).unwrap();
    std::fs::write(
        valid_dir.join("manifest.json"),
        r#"{"name":"@t/v","version":"1.0.0","manifestVersion":2,"wasmPath":"e.wasm"}"#,
    )
    .unwrap();

    // Invalid
    let invalid_dir = dir.path().join("invalid");
    std::fs::create_dir(&invalid_dir).unwrap();
    std::fs::write(invalid_dir.join("manifest.json"), "garbage").unwrap();

    let (resolved, diags) = discover_extensions(dir.path());
    assert_eq!(resolved.len(), 1);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "W029");
}

// B:discover_extensions — verify integration "non-existent directory produces warning"
#[test]
fn discover_nonexistent_dir_warning() {
    let (resolved, diags) = discover_extensions(Path::new("/nonexistent/extensions/dir"));
    assert!(resolved.is_empty());
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "W029");
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
            },
            LockFileEntry {
                name: "@specforge/governance".to_string(),
                version: "2.0.0".to_string(),
                source: "local:./ext".to_string(),
                wasm_hash: "def456".to_string(),
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
        }],
    };
    let results = run_doctor_check(&lock, dir.path(), |_| None, &HashMap::new());
    assert!(results
        .iter()
        .any(|r| matches!(r, DoctorStatus::MissingBinary { name } if name == "missing-ext")));
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
        }],
    };
    let results = run_doctor_check(
        &lock,
        dir.path(),
        |_| Some("different_hash".to_string()),
        &HashMap::new(),
    );
    assert!(results
        .iter()
        .any(|r| matches!(r, DoctorStatus::StaleHash { .. })));
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
        }],
    };
    let installed: HashMap<String, String> =
        [("good-ext".to_string(), "1.0.0".to_string())]
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

// ============================================================
// B:refresh_lock_file
// ============================================================

// B:refresh_lock_file — verify integration "new extension added to lock"
#[test]
fn refresh_adds_new_extension() {
    let mut lock = LockFile::new();
    let mut manifest = default_manifest();
    manifest.name = "@ext/new".to_string();
    manifest.version = "1.0.0".to_string();
    manifest.wasm_path = "extension.wasm".to_string();

    let resolved = vec![ResolvedExtension {
        manifest,
        source: ExtensionSpecifier::Registry {
            name: "@ext/new".to_string(),
            version: "1.0.0".to_string(),
        },
        manifest_path: PathBuf::from("/ext/manifest.json"),
    }];

    let diags = refresh_lock_file(&mut lock, &resolved, |_| Some("hash1".to_string()));
    assert!(diags.is_empty());
    assert_eq!(lock.entries.len(), 1);
    assert_eq!(lock.entries[0].name, "@ext/new");
}

// B:refresh_lock_file — verify integration "updated extension entry updated"
#[test]
fn refresh_updates_existing() {
    let mut lock = LockFile {
        lockfile_version: 1,
        entries: vec![LockFileEntry {
            name: "@ext/a".to_string(),
            version: "1.0.0".to_string(),
            source: "registry".to_string(),
            wasm_hash: "old".to_string(),
        }],
    };

    let mut manifest = default_manifest();
    manifest.name = "@ext/a".to_string();
    manifest.version = "2.0.0".to_string();
    manifest.wasm_path = "extension.wasm".to_string();

    let resolved = vec![ResolvedExtension {
        manifest,
        source: ExtensionSpecifier::Registry {
            name: "@ext/a".to_string(),
            version: "2.0.0".to_string(),
        },
        manifest_path: PathBuf::from("/ext/manifest.json"),
    }];

    refresh_lock_file(&mut lock, &resolved, |_| Some("new_hash".to_string()));
    assert_eq!(lock.entries.len(), 1);
    assert_eq!(lock.entries[0].version, "2.0.0");
    assert_eq!(lock.entries[0].wasm_hash, "new_hash");
}

// B:refresh_lock_file — verify integration "removed extension pruned from lock"
#[test]
fn refresh_prunes_removed() {
    let mut lock = LockFile {
        lockfile_version: 1,
        entries: vec![
            LockFileEntry {
                name: "@ext/keep".to_string(),
                version: "1.0.0".to_string(),
                source: "registry".to_string(),
                wasm_hash: "h1".to_string(),
            },
            LockFileEntry {
                name: "@ext/remove".to_string(),
                version: "1.0.0".to_string(),
                source: "registry".to_string(),
                wasm_hash: "h2".to_string(),
            },
        ],
    };

    let mut manifest = default_manifest();
    manifest.name = "@ext/keep".to_string();
    manifest.version = "1.0.0".to_string();
    manifest.wasm_path = "extension.wasm".to_string();

    let resolved = vec![ResolvedExtension {
        manifest,
        source: ExtensionSpecifier::Registry {
            name: "@ext/keep".to_string(),
            version: "1.0.0".to_string(),
        },
        manifest_path: PathBuf::from("/ext/manifest.json"),
    }];

    refresh_lock_file(&mut lock, &resolved, |_| Some("h1".to_string()));
    assert_eq!(lock.entries.len(), 1);
    assert_eq!(lock.entries[0].name, "@ext/keep");
}

// ============================================================
// B:validate_query_extensions
// ============================================================

// B:validate_query_extensions — verify integration "valid pattern produces QueryExtension"
#[test]
fn validate_query_ext_valid() {
    let raw = vec![RawQueryExtension {
        file_kind: "highlights".to_string(),
        pattern: "(identifier) @variable".to_string(),
    }];
    let (valid, warnings) = validate_query_extensions("@ext/test", &raw);
    assert!(warnings.is_empty());
    assert_eq!(valid.len(), 1);
    assert_eq!(valid[0].file_kind, QueryFileKind::Highlights);
}

// B:validate_query_extensions — verify integration "empty pattern produces W031"
#[test]
fn validate_query_ext_empty_w031() {
    let raw = vec![RawQueryExtension {
        file_kind: "highlights".to_string(),
        pattern: String::new(),
    }];
    let (valid, warnings) = validate_query_extensions("@ext/test", &raw);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "W031");
    assert!(valid.is_empty());
}

// B:validate_query_extensions — verify integration "null bytes produce W031, valid ones still pass"
#[test]
fn validate_query_ext_null_bytes_w031() {
    let raw = vec![
        RawQueryExtension {
            file_kind: "highlights".to_string(),
            pattern: "has\0null".to_string(),
        },
        RawQueryExtension {
            file_kind: "locals".to_string(),
            pattern: "(valid) @ok".to_string(),
        },
    ];
    let (valid, warnings) = validate_query_extensions("@ext/test", &raw);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "W031");
    assert_eq!(valid.len(), 1);
    assert_eq!(valid[0].file_kind, QueryFileKind::Locals);
}

// ============================================================
// B:compose_query_files
// ============================================================

fn make_ext(name: &str, pattern: &str) -> QueryExtension {
    QueryExtension {
        extension_name: name.to_string(),
        file_kind: QueryFileKind::Highlights,
        pattern: pattern.to_string(),
    }
}

// B:compose_query_files — verify integration "base + extensions composed correctly"
#[test]
fn compose_base_plus_extensions() {
    let base = "(identifier) @variable";
    let exts = vec![
        make_ext("@ext/a", "(string) @string"),
        make_ext("@ext/b", "(comment) @comment"),
    ];
    let result = compose_query_files(base, &exts);
    assert!(result.starts_with(base));
    assert!(result.contains("@string"));
    assert!(result.contains("@comment"));
    // Order preserved
    let a_pos = result.find("@string").unwrap();
    let b_pos = result.find("@comment").unwrap();
    assert!(a_pos < b_pos);
}

// B:compose_query_files — verify integration "empty extensions returns base unchanged"
#[test]
fn compose_empty_extensions_base_unchanged() {
    let base = "(identifier) @variable";
    let result = compose_query_files(base, &[]);
    assert_eq!(result, base);
}

// B:compose_query_files — verify integration "deterministic output across runs"
#[test]
fn compose_deterministic() {
    let base = ";; base";
    let exts = vec![make_ext("@a", "ext_a"), make_ext("@b", "ext_b")];
    let r1 = compose_query_files(base, &exts);
    let r2 = compose_query_files(base, &exts);
    assert_eq!(r1, r2);
}
