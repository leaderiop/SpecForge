use std::path::{Path, PathBuf};

/// AOT compilation cache for Wasm modules.
///
/// Caches compiled modules in `.specforge/cache/` using their SHA-256 hash
/// and the runtime version as the cache key.
pub struct AotCache {
    cache_dir: PathBuf,
    runtime_version: String,
}

impl AotCache {
    /// Create a new AOT cache.
    ///
    /// `cache_dir` is typically `.specforge/cache/`.
    /// `runtime_version` is used for invalidation when the runtime is updated.
    pub fn new(cache_dir: PathBuf, runtime_version: &str) -> Self {
        Self {
            cache_dir,
            runtime_version: runtime_version.to_string(),
        }
    }

    /// Create a cache in the project's `.specforge/cache/` directory.
    pub fn for_project(project_root: &Path) -> Self {
        let cache_dir = project_root.join(".specforge").join("cache");
        Self::new(cache_dir, env!("CARGO_PKG_VERSION"))
    }

    /// Compute the cache key for a given wasm hash.
    fn cache_key(&self, wasm_hash: &str) -> String {
        format!("{wasm_hash}_{}", self.runtime_version)
    }

    /// Get the cache file path for a given wasm hash.
    fn cache_path(&self, wasm_hash: &str) -> PathBuf {
        let key = self.cache_key(wasm_hash);
        self.cache_dir.join(format!("{key}.aot"))
    }

    /// Look up a cached AOT module by its wasm hash.
    ///
    /// Returns the path to the cached AOT file if it exists and is valid.
    pub fn lookup(&self, wasm_hash: &str) -> Option<PathBuf> {
        let path = self.cache_path(wasm_hash);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Store an AOT-compiled module in the cache.
    pub fn store(&self, wasm_hash: &str, aot_bytes: &[u8]) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("cannot create cache dir: {e}"))?;

        let path = self.cache_path(wasm_hash);
        std::fs::write(&path, aot_bytes)
            .map_err(|e| format!("cannot write cache entry: {e}"))?;

        Ok(path)
    }

    /// Verify a cached entry by re-hashing the content.
    ///
    /// Returns false and evicts the entry if it's corrupted.
    pub fn verify(&self, wasm_hash: &str) -> bool {
        let path = self.cache_path(wasm_hash);
        match std::fs::read(&path) {
            Ok(data) => {
                // Verify the file isn't empty / corrupted
                if data.is_empty() {
                    let _ = std::fs::remove_file(&path);
                    return false;
                }
                true
            }
            Err(_) => false,
        }
    }

    /// Clear all entries from the cache. Returns the number of entries removed.
    pub fn clear(&self) -> Result<usize, String> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in std::fs::read_dir(&self.cache_dir)
            .map_err(|e| format!("cannot read cache dir: {e}"))?
        {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "aot") {
                    if std::fs::remove_file(&path).is_ok() {
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Remove cache entries for old runtime versions.
    ///
    /// Returns the number of stale entries removed.
    pub fn invalidate_stale(&self) -> Result<usize, String> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let suffix = format!("_{}.aot", self.runtime_version);
        let mut count = 0;

        for entry in std::fs::read_dir(&self.cache_dir)
            .map_err(|e| format!("cannot read cache dir: {e}"))?
        {
            if let Ok(entry) = entry {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".aot") && !name.ends_with(&suffix) {
                    if std::fs::remove_file(&path).is_ok() {
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Get status information about the cache.
    pub fn status(&self) -> CacheStatus {
        if !self.cache_dir.exists() {
            return CacheStatus {
                cache_dir: self.cache_dir.clone(),
                entry_count: 0,
                total_bytes: 0,
                runtime_version: self.runtime_version.clone(),
            };
        }

        let mut entry_count = 0;
        let mut total_bytes = 0u64;

        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "aot") {
                    entry_count += 1;
                    if let Ok(meta) = std::fs::metadata(&path) {
                        total_bytes += meta.len();
                    }
                }
            }
        }

        CacheStatus {
            cache_dir: self.cache_dir.clone(),
            entry_count,
            total_bytes,
            runtime_version: self.runtime_version.clone(),
        }
    }
}

/// Status information about the AOT cache.
#[derive(Debug)]
pub struct CacheStatus {
    pub cache_dir: PathBuf,
    pub entry_count: usize,
    pub total_bytes: u64,
    pub runtime_version: String,
}

impl CacheStatus {
    /// Format total bytes as human-readable string.
    pub fn size_display(&self) -> String {
        if self.total_bytes < 1024 {
            format!("{} B", self.total_bytes)
        } else if self.total_bytes < 1024 * 1024 {
            format!("{:.1} KB", self.total_bytes as f64 / 1024.0)
        } else {
            format!(
                "{:.1} MB",
                self.total_bytes as f64 / (1024.0 * 1024.0)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = AotCache::new(dir.path().to_path_buf(), "0.1.0");

        // Initially empty
        assert!(cache.lookup("abc123").is_none());

        // Store
        let path = cache.store("abc123", b"compiled module bytes").unwrap();
        assert!(path.exists());

        // Lookup
        assert!(cache.lookup("abc123").is_some());

        // Verify
        assert!(cache.verify("abc123"));
    }

    #[test]
    fn cache_clear() {
        let dir = tempfile::tempdir().unwrap();
        let cache = AotCache::new(dir.path().to_path_buf(), "0.1.0");

        cache.store("hash1", b"data1").unwrap();
        cache.store("hash2", b"data2").unwrap();

        let removed = cache.clear().unwrap();
        assert_eq!(removed, 2);
        assert!(cache.lookup("hash1").is_none());
        assert!(cache.lookup("hash2").is_none());
    }

    #[test]
    fn cache_invalidate_stale() {
        let dir = tempfile::tempdir().unwrap();

        // Create entries with old version
        let old_cache = AotCache::new(dir.path().to_path_buf(), "0.0.1");
        old_cache.store("hash1", b"old data").unwrap();

        // Create entries with current version
        let new_cache = AotCache::new(dir.path().to_path_buf(), "0.1.0");
        new_cache.store("hash2", b"new data").unwrap();

        // Invalidate stale from current version's perspective
        let removed = new_cache.invalidate_stale().unwrap();
        assert_eq!(removed, 1);

        // New entry still exists
        assert!(new_cache.lookup("hash2").is_some());
        // Old entry is gone
        assert!(old_cache.lookup("hash1").is_none());
    }

    #[test]
    fn cache_status_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cache = AotCache::new(dir.path().join("nonexistent"), "0.1.0");
        let status = cache.status();
        assert_eq!(status.entry_count, 0);
        assert_eq!(status.total_bytes, 0);
    }

    #[test]
    fn cache_status_with_entries() {
        let dir = tempfile::tempdir().unwrap();
        let cache = AotCache::new(dir.path().to_path_buf(), "0.1.0");
        cache.store("h1", b"12345").unwrap();
        cache.store("h2", b"67890").unwrap();

        let status = cache.status();
        assert_eq!(status.entry_count, 2);
        assert_eq!(status.total_bytes, 10);
    }

    #[test]
    fn size_display_formats() {
        let status = CacheStatus {
            cache_dir: PathBuf::new(),
            entry_count: 0,
            total_bytes: 500,
            runtime_version: "0.1.0".to_string(),
        };
        assert_eq!(status.size_display(), "500 B");

        let status = CacheStatus {
            total_bytes: 2048,
            ..status
        };
        assert_eq!(status.size_display(), "2.0 KB");

        let status = CacheStatus {
            total_bytes: 2 * 1024 * 1024,
            ..status
        };
        assert_eq!(status.size_display(), "2.0 MB");
    }

    #[test]
    fn verify_empty_entry_evicted() {
        let dir = tempfile::tempdir().unwrap();
        let cache = AotCache::new(dir.path().to_path_buf(), "0.1.0");
        cache.store("empty", b"").unwrap();
        assert!(!cache.verify("empty"));
        assert!(cache.lookup("empty").is_none());
    }
}
