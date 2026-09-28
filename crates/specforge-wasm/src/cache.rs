use specforge_common::{Diagnostic, Severity};
use std::path::{Path, PathBuf};

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
    if path.exists() { Some(path) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::hex_sha256;
    use tempfile::TempDir;

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
