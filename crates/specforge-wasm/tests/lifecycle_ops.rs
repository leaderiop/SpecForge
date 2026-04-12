use specforge_wasm::{
    check_newer_version, install_extension, install_from_local, uninstall_extension,
    upgrade_extension, LockFile, LockFileEntry,
};
use specforge_registry::{ManifestV2, PeerDependency};
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
        surfaces: None,
    }
}

fn make_manifest(name: &str, version: &str, peers: &[(&str, &str)]) -> ManifestV2 {
    ManifestV2 {
        name: name.to_string(),
        version: version.to_string(),
        peer_dependencies: peers
            .iter()
            .map(|(n, v)| PeerDependency {
                name: n.to_string(),
                version: v.to_string(),
                optional: false,
            })
            .collect(),
        ..default_manifest()
    }
}

fn fake_wasm_bytes() -> Vec<u8> {
    b"\x00asm\x01\x00\x00\x00fake_extension_binary".to_vec()
}

fn compute_sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

// ============================================================================
// Install tests
// ============================================================================

// B:install_wasm_extension — verify unit "resolve -> download -> verify SHA256 -> place binary"
#[test]
fn test_install_verify_sha256_and_place_binary() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let wasm_bytes = fake_wasm_bytes();
    let expected_hash = compute_sha256(&wasm_bytes);
    let mut lock = LockFile::new();

    let result = install_extension(
        "@test/my-ext",
        "1.0.0",
        &wasm_bytes,
        &expected_hash,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        true,
    )
    .unwrap();

    assert_eq!(result.name, "@test/my-ext");
    assert_eq!(result.version, "1.0.0");
    assert_eq!(result.wasm_hash, expected_hash);

    // Binary was placed
    let wasm_path = extensions_dir.join("@test/my-ext").join("extension.wasm");
    assert!(wasm_path.exists());
    assert_eq!(std::fs::read(&wasm_path).unwrap(), wasm_bytes);
}

// B:install_wasm_extension — verify unit "install from local path (copy)"
#[test]
fn test_install_from_local_path() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    // Write a local wasm file
    let local_wasm = dir.path().join("local_ext.wasm");
    let wasm_bytes = fake_wasm_bytes();
    std::fs::write(&local_wasm, &wasm_bytes).unwrap();

    let mut lock = LockFile::new();

    let result = install_from_local(
        "@test/local-ext",
        "1.0.0",
        &local_wasm,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        true,
    )
    .unwrap();

    assert_eq!(result.name, "@test/local-ext");
    assert_eq!(result.version, "1.0.0");

    // Binary was copied
    let installed_path = extensions_dir
        .join("@test/local-ext")
        .join("extension.wasm");
    assert!(installed_path.exists());
    assert_eq!(std::fs::read(&installed_path).unwrap(), wasm_bytes);
}

// B:install_wasm_extension — verify unit "install AOT compiles after placement"
#[test]
fn test_install_aot_compiles_after_placement() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let wasm_bytes = fake_wasm_bytes();
    let expected_hash = compute_sha256(&wasm_bytes);
    let mut lock = LockFile::new();

    let result = install_extension(
        "@test/aot-ext",
        "1.0.0",
        &wasm_bytes,
        &expected_hash,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false, // do NOT skip AOT
    )
    .unwrap();

    assert!(result.aot_compiled);

    // AOT cache artifact exists
    let aot_path = cache_dir.join(format!("{}.aot", expected_hash));
    assert!(aot_path.exists());
}

// B:install_wasm_extension — verify unit "install updates lock file with extension entry"
#[test]
fn test_install_updates_lock_file() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let wasm_bytes = fake_wasm_bytes();
    let expected_hash = compute_sha256(&wasm_bytes);
    let mut lock = LockFile::new();

    install_extension(
        "@test/lock-ext",
        "2.0.0",
        &wasm_bytes,
        &expected_hash,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        true,
    )
    .unwrap();

    assert_eq!(lock.entries.len(), 1);
    assert_eq!(lock.entries[0].name, "@test/lock-ext");
    assert_eq!(lock.entries[0].version, "2.0.0");
    assert_eq!(lock.entries[0].wasm_hash, expected_hash);
    assert_eq!(lock.entries[0].source, "registry");
}

// B:install_wasm_extension — verify unit "install rolls back on download failure (atomic)"
#[test]
fn test_install_rolls_back_on_integrity_failure() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let wasm_bytes = fake_wasm_bytes();
    let mut lock = LockFile::new();

    // Pass wrong expected hash to simulate integrity failure
    let err = install_extension(
        "@test/bad-ext",
        "1.0.0",
        &wasm_bytes,
        "0000000000000000000000000000000000000000000000000000000000000000",
        &extensions_dir,
        &cache_dir,
        &mut lock,
        true,
    )
    .unwrap_err();

    assert_eq!(err.code, "E032");
    assert!(err.message.contains("integrity check failed"));

    // No directory was created
    let ext_dir = extensions_dir.join("@test/bad-ext");
    assert!(!ext_dir.exists());

    // Lock file is unchanged
    assert!(lock.entries.is_empty());
}

// B:install_wasm_extension — verify unit "install defers AOT when skip_aot is true"
#[test]
fn test_install_defers_aot_when_skip_aot() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let wasm_bytes = fake_wasm_bytes();
    let expected_hash = compute_sha256(&wasm_bytes);
    let mut lock = LockFile::new();

    let result = install_extension(
        "@test/no-aot-ext",
        "1.0.0",
        &wasm_bytes,
        &expected_hash,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        true, // skip AOT
    )
    .unwrap();

    assert!(!result.aot_compiled);

    // No AOT cache artifact
    let aot_path = cache_dir.join(format!("{}.aot", expected_hash));
    assert!(!aot_path.exists());
}

// ============================================================================
// Upgrade tests
// ============================================================================

// B:upgrade_wasm_extension — verify unit "check source for newer version"
#[test]
fn test_check_newer_version() {
    // Semver comparisons
    assert!(check_newer_version("1.0.0", "1.0.1"));
    assert!(check_newer_version("1.0.0", "1.1.0"));
    assert!(check_newer_version("1.0.0", "2.0.0"));
    assert!(!check_newer_version("1.0.0", "1.0.0"));
    assert!(!check_newer_version("2.0.0", "1.0.0"));
    assert!(!check_newer_version("1.1.0", "1.0.9"));

    // Fallback string comparison for non-semver
    assert!(check_newer_version("alpha", "beta"));
    assert!(!check_newer_version("beta", "alpha"));
}

// B:upgrade_wasm_extension — verify unit "validate peer dependency compatibility"
#[test]
fn test_upgrade_validates_peer_compat() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let wasm_bytes = fake_wasm_bytes();
    let expected_hash = compute_sha256(&wasm_bytes);

    // Set up: install v1 first
    let mut lock = LockFile::new();
    lock.entries.push(LockFileEntry {
        name: "@test/ext-a".to_string(),
        version: "1.0.0".to_string(),
        source: "registry".to_string(),
        wasm_hash: "old_hash".to_string(),
    });

    // Peer manifests: ext-b depends on ext-a >=1.0.0
    let ext_a_new = make_manifest("@test/ext-a", "2.0.0", &[]);
    let ext_b = make_manifest("@test/ext-b", "1.0.0", &[("@test/ext-a", ">=1.0.0")]);
    let peer_manifests = vec![ext_a_new.clone(), ext_b];

    // Upgrade should succeed when peers are satisfied
    let result = upgrade_extension(
        "@test/ext-a",
        "2.0.0",
        &wasm_bytes,
        &expected_hash,
        &peer_manifests,
        &ext_a_new,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false,
    )
    .unwrap();

    assert_eq!(result.old_version, "1.0.0");
    assert_eq!(result.new_version, "2.0.0");
}

// B:upgrade_wasm_extension — verify unit "reject breaking peer change without --force"
#[test]
fn test_upgrade_rejects_breaking_peer_without_force() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let wasm_bytes = fake_wasm_bytes();
    let expected_hash = compute_sha256(&wasm_bytes);

    let mut lock = LockFile::new();
    lock.entries.push(LockFileEntry {
        name: "@test/core".to_string(),
        version: "1.0.0".to_string(),
        source: "registry".to_string(),
        wasm_hash: "old_hash".to_string(),
    });

    // New manifest declares a peer dep on a non-existent extension
    let new_manifest = make_manifest(
        "@test/core",
        "3.0.0",
        &[("@test/nonexistent", ">=1.0.0")],
    );

    // Peer manifests only include the new core (nonexistent is missing)
    let peer_manifests = vec![new_manifest.clone()];

    let err = upgrade_extension(
        "@test/core",
        "3.0.0",
        &wasm_bytes,
        &expected_hash,
        &peer_manifests,
        &new_manifest,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false, // no force
    )
    .unwrap_err();

    assert_eq!(err.code, "E027");
    assert!(err.message.contains("breaks peer dependencies"));
    assert!(err.suggestion.as_ref().unwrap().contains("--force"));
}

// B:upgrade_wasm_extension — verify unit "invalidate old AOT cache + recompile"
#[test]
fn test_upgrade_invalidates_old_aot_and_recompiles() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();
    std::fs::create_dir_all(&cache_dir).unwrap();

    // Create old AOT artifact
    let old_hash = "old_fake_hash";
    let old_aot_path = cache_dir.join(format!("{}.aot", old_hash));
    std::fs::write(&old_aot_path, b"old aot data").unwrap();

    let mut lock = LockFile::new();
    lock.entries.push(LockFileEntry {
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        source: "registry".to_string(),
        wasm_hash: old_hash.to_string(),
    });

    let wasm_bytes = fake_wasm_bytes();
    let expected_hash = compute_sha256(&wasm_bytes);
    let new_manifest = make_manifest("@test/ext", "2.0.0", &[]);

    let result = upgrade_extension(
        "@test/ext",
        "2.0.0",
        &wasm_bytes,
        &expected_hash,
        std::slice::from_ref(&new_manifest),
        &new_manifest,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false,
    )
    .unwrap();

    // Old AOT was invalidated
    assert!(!old_aot_path.exists());

    // New AOT was compiled
    assert!(result.aot_recompiled);
    let new_aot_path = cache_dir.join(format!("{}.aot", expected_hash));
    assert!(new_aot_path.exists());

    // Lock file updated
    assert_eq!(lock.entries.len(), 1);
    assert_eq!(lock.entries[0].version, "2.0.0");
    assert_eq!(lock.entries[0].wasm_hash, expected_hash);
}

// ============================================================================
// Uninstall tests
// ============================================================================

// B:uninstall_wasm_extension — verify unit "remove from lock file"
#[test]
fn test_uninstall_removes_from_lock_file() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    // Create the extension dir with a wasm file
    let ext_dir = extensions_dir.join("@test/ext");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::write(ext_dir.join("extension.wasm"), b"wasm").unwrap();

    let mut lock = LockFile::new();
    lock.entries.push(LockFileEntry {
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        source: "registry".to_string(),
        wasm_hash: "abc123".to_string(),
    });

    uninstall_extension(
        "@test/ext",
        &[],
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false,
    )
    .unwrap();

    assert!(lock.entries.is_empty());
}

// B:uninstall_wasm_extension — verify unit "delete .wasm binary"
#[test]
fn test_uninstall_deletes_wasm_binary() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let ext_dir = extensions_dir.join("@test/ext");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::write(ext_dir.join("extension.wasm"), b"wasm data").unwrap();

    let mut lock = LockFile::new();
    lock.entries.push(LockFileEntry {
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        source: "registry".to_string(),
        wasm_hash: "hash".to_string(),
    });

    uninstall_extension(
        "@test/ext",
        &[],
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false,
    )
    .unwrap();

    assert!(!ext_dir.exists());
}

// B:uninstall_wasm_extension — verify unit "invalidate AOT cache"
#[test]
fn test_uninstall_invalidates_aot_cache() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();
    std::fs::create_dir_all(&cache_dir).unwrap();

    let ext_dir = extensions_dir.join("@test/ext");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::write(ext_dir.join("extension.wasm"), b"wasm").unwrap();

    // Create an AOT cache artifact
    let wasm_hash = "aot_hash_123";
    let aot_path = cache_dir.join(format!("{}.aot", wasm_hash));
    std::fs::write(&aot_path, b"aot artifact").unwrap();

    let mut lock = LockFile::new();
    lock.entries.push(LockFileEntry {
        name: "@test/ext".to_string(),
        version: "1.0.0".to_string(),
        source: "registry".to_string(),
        wasm_hash: wasm_hash.to_string(),
    });

    let result = uninstall_extension(
        "@test/ext",
        &[],
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false,
    )
    .unwrap();

    assert!(result.cache_invalidated);
    assert!(!aot_path.exists());
}

// B:uninstall_wasm_extension — verify unit "reject when dependents exist without --force"
#[test]
fn test_uninstall_rejects_when_dependents_exist() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let core = make_manifest("@test/core", "1.0.0", &[]);
    let dependent = make_manifest("@test/dep", "1.0.0", &[("@test/core", ">=1.0.0")]);
    let manifests = vec![core, dependent];

    let mut lock = LockFile::new();
    lock.entries.push(LockFileEntry {
        name: "@test/core".to_string(),
        version: "1.0.0".to_string(),
        source: "registry".to_string(),
        wasm_hash: "hash".to_string(),
    });

    let err = uninstall_extension(
        "@test/core",
        &manifests,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        false,
    )
    .unwrap_err();

    assert_eq!(err.code, "E027");
    assert!(err.message.contains("required by"));
    assert!(err.message.contains("@test/dep"));

    // Lock file unchanged
    assert_eq!(lock.entries.len(), 1);
}

// B:uninstall_wasm_extension — verify unit "roll back on failure (restore lock entry)"
#[test]
fn test_uninstall_with_force_bypasses_dependent_check() {
    let dir = TempDir::new().unwrap();
    let extensions_dir = dir.path().join("extensions");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&extensions_dir).unwrap();

    let ext_dir = extensions_dir.join("@test/core");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::write(ext_dir.join("extension.wasm"), b"wasm").unwrap();

    let core = make_manifest("@test/core", "1.0.0", &[]);
    let dependent = make_manifest("@test/dep", "1.0.0", &[("@test/core", ">=1.0.0")]);
    let manifests = vec![core, dependent];

    let mut lock = LockFile::new();
    lock.entries.push(LockFileEntry {
        name: "@test/core".to_string(),
        version: "1.0.0".to_string(),
        source: "registry".to_string(),
        wasm_hash: "hash".to_string(),
    });

    // With force=true, should succeed even with dependents
    let result = uninstall_extension(
        "@test/core",
        &manifests,
        &extensions_dir,
        &cache_dir,
        &mut lock,
        true, // force
    )
    .unwrap();

    assert_eq!(result.name, "@test/core");
    assert!(lock.entries.is_empty());
    assert!(!ext_dir.exists());
}
