use crate::integrity::hex_sha256;
use specforge_common::{Diagnostic, Severity};
use std::path::{Path, PathBuf};

/// Reason for AOT cache invalidation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationReason {
    RuntimeVersionChange,
    UserClear,
    BinaryContentChanged,
    ExtensionRemoved,
}

/// AOT cache entry metadata.
#[derive(Debug, Clone)]
pub struct AotCacheEntry {
    pub wasm_hash: String,
    pub aot_path: PathBuf,
    pub platform: String,
}

/// Compute the cache path for a given wasm binary hash.
pub fn cache_path_for_hash(cache_dir: &Path, wasm_hash: &str) -> PathBuf {
    cache_dir.join(format!("{}.aot", wasm_hash))
}

/// AOT compile a .wasm binary and cache the result.
/// In this implementation, we simulate AOT by copying the binary into the cache dir
/// with a content-hash filename. Real Extism would produce a native module.
pub fn aot_compile(
    wasm_path: &Path,
    cache_dir: &Path,
) -> Result<AotCacheEntry, Diagnostic> {
    let bytes = std::fs::read(wasm_path).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!("cannot read .wasm binary for AOT compilation: {}", e),
        span: None,
        suggestion: None,
    })?;

    let wasm_hash = hex_sha256(&bytes);
    let aot_path = cache_path_for_hash(cache_dir, &wasm_hash);

    std::fs::create_dir_all(cache_dir).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!("cannot create AOT cache directory: {}", e),
        span: None,
        suggestion: None,
    })?;

    // Write compiled artifact (simulated — in production this would be Extism AOT output)
    std::fs::write(&aot_path, &bytes).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!("cannot write AOT cache artifact: {}", e),
        span: None,
        suggestion: None,
    })?;

    Ok(AotCacheEntry {
        wasm_hash,
        aot_path,
        platform: std::env::consts::ARCH.to_string(),
    })
}

/// Check if a valid cache entry exists for the given wasm binary.
pub fn has_cached_artifact(wasm_path: &Path, cache_dir: &Path) -> Option<AotCacheEntry> {
    let bytes = std::fs::read(wasm_path).ok()?;
    let wasm_hash = hex_sha256(&bytes);
    let aot_path = cache_path_for_hash(cache_dir, &wasm_hash);

    if aot_path.exists() {
        // Verify integrity: re-hash the cached artifact
        let cached_bytes = std::fs::read(&aot_path).ok()?;
        let cached_hash = hex_sha256(&cached_bytes);
        if cached_hash == wasm_hash {
            return Some(AotCacheEntry {
                wasm_hash,
                aot_path,
                platform: std::env::consts::ARCH.to_string(),
            });
        }
        // Corrupted — evict silently
        let _ = std::fs::remove_file(&aot_path);
    }
    None
}

/// Invalidate cache entries. Returns paths of removed artifacts.
pub fn invalidate_cache(cache_dir: &Path, reason: InvalidationReason) -> Vec<PathBuf> {
    let mut removed = Vec::new();

    match reason {
        InvalidationReason::UserClear | InvalidationReason::RuntimeVersionChange => {
            // Remove all .aot files in cache dir
            if let Ok(entries) = std::fs::read_dir(cache_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "aot")
                        && std::fs::remove_file(&path).is_ok()
                    {
                        removed.push(path);
                    }
                }
            }
        }
        InvalidationReason::BinaryContentChanged | InvalidationReason::ExtensionRemoved => {
            // Specific entry invalidation happens via caller providing the hash
            // This is a no-op at the directory level
        }
    }

    removed
}

/// Compute a grammar cache key from content hash and ABI version.
pub fn grammar_cache_key(content_hash: &str, abi_version: u32) -> String {
    format!("{}_abi{}", content_hash, abi_version)
}

/// Cache a grammar artifact to disk.
pub fn cache_grammar_artifact(
    content_hash: &str,
    abi_version: u32,
    grammar_bytes: &[u8],
    cache_dir: &Path,
) -> Result<PathBuf, Diagnostic> {
    let key = grammar_cache_key(content_hash, abi_version);
    let path = cache_dir.join(format!("{}.grammar", key));

    std::fs::create_dir_all(cache_dir).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!("cannot create grammar cache directory: {}", e),
        span: None,
        suggestion: None,
    })?;

    std::fs::write(&path, grammar_bytes).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!("cannot write grammar cache artifact: {}", e),
        span: None,
        suggestion: None,
    })?;

    Ok(path)
}

/// Check if a cached grammar artifact exists.
pub fn has_cached_grammar(
    content_hash: &str,
    abi_version: u32,
    cache_dir: &Path,
) -> Option<PathBuf> {
    let key = grammar_cache_key(content_hash, abi_version);
    let path = cache_dir.join(format!("{}.grammar", key));
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Invalidate a specific cache entry by wasm hash.
pub fn invalidate_entry(cache_dir: &Path, wasm_hash: &str) -> bool {
    let path = cache_path_for_hash(cache_dir, wasm_hash);
    std::fs::remove_file(&path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_fake_wasm(dir: &TempDir, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    // -- aot_compile_wasm_module --

    // B:aot_compile_wasm_module — verify unit "first load triggers AOT compilation"
    #[test]
    fn test_first_load_triggers_aot_compilation() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", b"\x00asm_data");

        let entry = aot_compile(&wasm_path, &cache_dir).unwrap();
        assert!(entry.aot_path.exists());
        assert!(!entry.wasm_hash.is_empty());
    }

    // B:aot_compile_wasm_module — verify unit "compiled artifact cached with content-hash filename"
    #[test]
    fn test_compiled_artifact_cached_with_content_hash_filename() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let content = b"wasm binary data";
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", content);
        let expected_hash = hex_sha256(content);

        let entry = aot_compile(&wasm_path, &cache_dir).unwrap();
        assert_eq!(entry.wasm_hash, expected_hash);
        assert_eq!(entry.aot_path.file_name().unwrap().to_str().unwrap(), format!("{}.aot", expected_hash));
    }

    // B:aot_compile_wasm_module — verify unit "subsequent load uses cached artifact"
    #[test]
    fn test_subsequent_load_uses_cached_artifact() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", b"module_bytes");

        // First compile
        aot_compile(&wasm_path, &cache_dir).unwrap();

        // Subsequent check finds cached
        let cached = has_cached_artifact(&wasm_path, &cache_dir);
        assert!(cached.is_some());
    }

    // B:aot_compile_wasm_module — verify contract "requires/ensures consistency for AOT Wasm compilation"
    #[test]
    fn test_aot_compile_contract() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", b"binary");

        // ensures: wasm_aot_compiled — compilation succeeds
        let entry = aot_compile(&wasm_path, &cache_dir).unwrap();
        assert!(entry.aot_path.exists());

        // ensures: artifact_cached — content-hash filename
        let expected_hash = hex_sha256(b"binary");
        assert_eq!(entry.wasm_hash, expected_hash);

        // ensures: subsequent_loads_fast — cache hit
        assert!(has_cached_artifact(&wasm_path, &cache_dir).is_some());
    }

    // -- cache_aot_artifacts --

    // B:cache_aot_artifacts — verify unit "cache entries use content-addressed filenames"
    #[test]
    fn test_cache_entries_use_content_addressed_filenames() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let content_a = b"module_a";
        let content_b = b"module_b";
        let wasm_a = create_fake_wasm(&dir, "a.wasm", content_a);
        let wasm_b = create_fake_wasm(&dir, "b.wasm", content_b);

        let entry_a = aot_compile(&wasm_a, &cache_dir).unwrap();
        let entry_b = aot_compile(&wasm_b, &cache_dir).unwrap();

        // Different content -> different filenames
        assert_ne!(entry_a.aot_path, entry_b.aot_path);
        assert_ne!(entry_a.wasm_hash, entry_b.wasm_hash);
    }

    // B:cache_aot_artifacts — verify unit "corrupted cache entry is evicted and recompiled"
    #[test]
    fn test_corrupted_cache_entry_is_evicted() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", b"good_data");

        // Compile first
        let entry = aot_compile(&wasm_path, &cache_dir).unwrap();

        // Corrupt the cache
        std::fs::write(&entry.aot_path, b"corrupted_data").unwrap();

        // Cache check should evict corrupted entry
        let cached = has_cached_artifact(&wasm_path, &cache_dir);
        assert!(cached.is_none());
        assert!(!entry.aot_path.exists()); // Evicted
    }

    // B:cache_aot_artifacts — verify contract "requires/ensures consistency for AOT artifact caching"
    #[test]
    fn test_cache_aot_artifacts_contract() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", b"data");

        // ensures: content_addressed
        let entry = aot_compile(&wasm_path, &cache_dir).unwrap();
        assert!(entry.aot_path.to_str().unwrap().contains(&entry.wasm_hash));

        // ensures: corruption_detected + corruption_recovered
        std::fs::write(&entry.aot_path, b"bad").unwrap();
        assert!(has_cached_artifact(&wasm_path, &cache_dir).is_none());
    }

    // -- invalidate_aot_cache --

    // B:invalidate_aot_cache — verify unit "invalidates on runtime version change"
    #[test]
    fn test_invalidates_on_runtime_version_change() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", b"data");
        aot_compile(&wasm_path, &cache_dir).unwrap();

        let removed = invalidate_cache(&cache_dir, InvalidationReason::RuntimeVersionChange);
        assert_eq!(removed.len(), 1);
    }

    // B:invalidate_aot_cache — verify unit "invalidates on specforge cache clear"
    #[test]
    fn test_invalidates_on_user_clear() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_a = create_fake_wasm(&dir, "a.wasm", b"a");
        let wasm_b = create_fake_wasm(&dir, "b.wasm", b"b");
        aot_compile(&wasm_a, &cache_dir).unwrap();
        aot_compile(&wasm_b, &cache_dir).unwrap();

        let removed = invalidate_cache(&cache_dir, InvalidationReason::UserClear);
        assert_eq!(removed.len(), 2);
    }

    // B:invalidate_aot_cache — verify unit "invalidates when .wasm binary changes"
    #[test]
    fn test_invalidates_when_binary_changes() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", b"v1_data");
        let entry = aot_compile(&wasm_path, &cache_dir).unwrap();
        let old_hash = entry.wasm_hash.clone();

        // Invalidate old entry
        assert!(invalidate_entry(&cache_dir, &old_hash));
        assert!(!entry.aot_path.exists());
    }

    // B:invalidate_aot_cache — verify unit "removes stale AOT artifacts"
    #[test]
    fn test_removes_stale_aot_artifacts() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", b"data");
        let entry = aot_compile(&wasm_path, &cache_dir).unwrap();
        assert!(entry.aot_path.exists());

        invalidate_cache(&cache_dir, InvalidationReason::UserClear);
        assert!(!entry.aot_path.exists());
    }

    // B:invalidate_aot_cache — verify contract "requires/ensures consistency for AOT cache invalidation"
    #[test]
    fn test_invalidate_cache_contract() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("cache");
        let wasm_path = create_fake_wasm(&dir, "ext.wasm", b"module");
        aot_compile(&wasm_path, &cache_dir).unwrap();

        // ensures: aot_cache_invalidated — stale removed
        let removed = invalidate_cache(&cache_dir, InvalidationReason::RuntimeVersionChange);
        assert!(!removed.is_empty());

        // ensures: extension_marked_for_recompilation — no cache available
        assert!(has_cached_artifact(&wasm_path, &cache_dir).is_none());

        // Recompile works after invalidation
        let new_entry = aot_compile(&wasm_path, &cache_dir).unwrap();
        assert!(new_entry.aot_path.exists());
    }

    // -- grammar cache --

    // B:cache_grammar_artifacts — verify unit "cache key combines content hash and ABI version"
    #[test]
    fn test_grammar_cache_key_composite() {
        let key = grammar_cache_key("abc123", 14);
        assert_eq!(key, "abc123_abi14");
        // Different ABI = different key
        let key2 = grammar_cache_key("abc123", 15);
        assert_ne!(key, key2);
        // Different hash = different key
        let key3 = grammar_cache_key("def456", 14);
        assert_ne!(key, key3);
    }

    // B:cache_grammar_artifacts — verify unit "cache hit skips grammar loading"
    #[test]
    fn test_grammar_cache_hit() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("grammar_cache");
        let bytes = b"grammar bytes";
        let hash = hex_sha256(bytes);

        // Cache it
        let path = cache_grammar_artifact(&hash, 14, bytes, &cache_dir).unwrap();
        assert!(path.exists());

        // Hit
        let cached = has_cached_grammar(&hash, 14, &cache_dir);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), path);
    }

    // B:cache_grammar_artifacts — verify unit "content hash change invalidates cache"
    #[test]
    fn test_grammar_cache_content_change_invalidates() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("grammar_cache");
        let bytes = b"grammar v1";
        let hash = hex_sha256(bytes);

        cache_grammar_artifact(&hash, 14, bytes, &cache_dir).unwrap();

        // Different content hash = cache miss
        let new_hash = hex_sha256(b"grammar v2");
        assert!(has_cached_grammar(&new_hash, 14, &cache_dir).is_none());
    }

    // B:cache_grammar_artifacts — verify unit "ABI version change invalidates cache"
    #[test]
    fn test_grammar_cache_abi_change_invalidates() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("grammar_cache");
        let bytes = b"grammar bytes";
        let hash = hex_sha256(bytes);

        cache_grammar_artifact(&hash, 14, bytes, &cache_dir).unwrap();

        // Same hash, different ABI = miss
        assert!(has_cached_grammar(&hash, 15, &cache_dir).is_none());
        // Same hash, same ABI = hit
        assert!(has_cached_grammar(&hash, 14, &cache_dir).is_some());
    }

    // B:cache_grammar_artifacts — verify contract
    #[test]
    fn test_cache_grammar_artifact_contract() {
        let dir = TempDir::new().unwrap();
        let cache_dir = dir.path().join("grammar_cache");
        let bytes = b"test grammar";
        let hash = hex_sha256(bytes);

        // ensures: caching writes to disk
        let path = cache_grammar_artifact(&hash, 14, bytes, &cache_dir).unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);

        // ensures: cache key is composite
        let key = grammar_cache_key(&hash, 14);
        assert!(path.to_str().unwrap().contains(&key));

        // ensures: cache hit returns path
        assert_eq!(has_cached_grammar(&hash, 14, &cache_dir).unwrap(), path);

        // ensures: cache miss for different params
        assert!(has_cached_grammar(&hash, 99, &cache_dir).is_none());
        assert!(has_cached_grammar("other_hash", 14, &cache_dir).is_none());
    }
}
