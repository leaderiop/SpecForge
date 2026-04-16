use std::collections::VecDeque;
use std::sync::Mutex;

/// Configuration for the warm engine instance pool.
#[derive(Debug, Clone)]
pub struct WarmEngineConfig {
    pub max_instances: u32,
    pub max_memory_mb: u32,
}

impl Default for WarmEngineConfig {
    fn default() -> Self {
        Self {
            max_instances: 16,
            max_memory_mb: 512,
        }
    }
}

/// A warm engine instance entry.
#[derive(Debug, Clone)]
pub struct WarmInstance {
    pub extension_name: String,
    pub memory_mb: u32,
}

/// LRU-based pool of warm Wasm engine instances.
///
/// Thread-safe: wraps the instance queue in a `Mutex` so the pool
/// can be shared via `Arc<EnginePool>` across concurrent request handlers
/// (e.g., MCP server). All mutating methods take `&self`.
#[derive(Debug)]
pub struct EnginePool {
    instances: Mutex<VecDeque<WarmInstance>>,
    config: WarmEngineConfig,
}

impl EnginePool {
    pub fn new(config: WarmEngineConfig) -> Self {
        Self {
            instances: Mutex::new(VecDeque::new()),
            config,
        }
    }

    /// Warm (add or refresh) an engine instance.
    /// Returns the evicted instance name if eviction was needed.
    pub fn warm(&self, extension_name: &str, memory_mb: u32) -> Option<String> {
        let mut instances = self.instances.lock().expect("EnginePool lock poisoned");

        // Remove existing entry for this extension (refresh to front)
        instances.retain(|i| i.extension_name != extension_name);

        let mut evicted = None;

        // Check capacity
        if instances.len() as u32 >= self.config.max_instances {
            evicted = instances.pop_front().map(|i| i.extension_name);
        }

        // Check memory ceiling
        let total_mem: u32 = instances.iter().map(|i| i.memory_mb).sum();
        let mut current_total = total_mem;
        while current_total + memory_mb > self.config.max_memory_mb
            && !instances.is_empty()
        {
            if let Some(removed) = instances.pop_front() {
                current_total -= removed.memory_mb;
                evicted = Some(removed.extension_name);
            }
        }

        instances.push_back(WarmInstance {
            extension_name: extension_name.to_string(),
            memory_mb,
        });

        evicted
    }

    /// Evict and return the least-recently-used instance.
    pub fn evict_lru(&self) -> Option<String> {
        let mut instances = self.instances.lock().expect("EnginePool lock poisoned");
        instances.pop_front().map(|i| i.extension_name)
    }

    /// Remove a specific extension from the pool.
    pub fn remove(&self, extension_name: &str) -> bool {
        let mut instances = self.instances.lock().expect("EnginePool lock poisoned");
        let before = instances.len();
        instances.retain(|i| i.extension_name != extension_name);
        instances.len() < before
    }

    pub fn len(&self) -> usize {
        let instances = self.instances.lock().expect("EnginePool lock poisoned");
        instances.len()
    }

    pub fn is_empty(&self) -> bool {
        let instances = self.instances.lock().expect("EnginePool lock poisoned");
        instances.is_empty()
    }

    pub fn total_memory(&self) -> u32 {
        let instances = self.instances.lock().expect("EnginePool lock poisoned");
        instances.iter().map(|i| i.memory_mb).sum()
    }

    pub fn contains(&self, extension_name: &str) -> bool {
        let instances = self.instances.lock().expect("EnginePool lock poisoned");
        instances.iter().any(|i| i.extension_name == extension_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- warm_wasm_engine_instance --

    // B:warm_wasm_engine_instance — verify unit "warm instance reused across compilations"
    #[test]
    fn test_warm_instance_reused_across_compilations() {
        let pool = EnginePool::new(WarmEngineConfig::default());
        pool.warm("ext-a", 32);

        assert!(pool.contains("ext-a"));
        assert_eq!(pool.len(), 1);

        // Warming again refreshes, doesn't duplicate
        pool.warm("ext-a", 32);
        assert_eq!(pool.len(), 1);
    }

    // B:warm_wasm_engine_instance — verify unit "instance unloaded on extension removal"
    #[test]
    fn test_instance_unloaded_on_extension_removal() {
        let pool = EnginePool::new(WarmEngineConfig::default());
        pool.warm("ext-a", 32);
        pool.warm("ext-b", 32);

        assert!(pool.remove("ext-a"));
        assert!(!pool.contains("ext-a"));
        assert!(pool.contains("ext-b"));
    }

    // B:warm_wasm_engine_instance — verify contract "requires/ensures consistency for warm engine instance management"
    #[test]
    fn test_warm_engine_contract() {
        let pool = EnginePool::new(WarmEngineConfig { max_instances: 2, max_memory_mb: 100 });

        // ensures: engine_warmed — instances tracked
        pool.warm("a", 30);
        pool.warm("b", 30);
        assert_eq!(pool.len(), 2);

        // ensures: instance_reused
        assert!(pool.contains("a"));

        // ensures: instance_unloaded_on_removal
        pool.remove("a");
        assert!(!pool.contains("a"));
        assert_eq!(pool.len(), 1);
    }

    // -- evict_warm_engine_instance --

    // B:evict_warm_engine_instance — verify unit "LRU engine evicted when max instances exceeded"
    #[test]
    fn test_lru_engine_evicted_when_max_instances_exceeded() {
        let pool = EnginePool::new(WarmEngineConfig { max_instances: 2, max_memory_mb: 512 });
        pool.warm("first", 10);
        pool.warm("second", 10);

        // Adding a third should evict "first" (LRU)
        let evicted = pool.warm("third", 10);
        assert_eq!(evicted, Some("first".to_string()));
        assert!(!pool.contains("first"));
        assert!(pool.contains("second"));
        assert!(pool.contains("third"));
    }

    // B:evict_warm_engine_instance — verify unit "memory ceiling triggers eviction of least-recent engine"
    #[test]
    fn test_memory_ceiling_triggers_eviction() {
        let pool = EnginePool::new(WarmEngineConfig { max_instances: 10, max_memory_mb: 100 });
        pool.warm("a", 40);
        pool.warm("b", 40);

        // Adding 30MB would exceed 100MB ceiling -> evict LRU ("a")
        let evicted = pool.warm("c", 30);
        assert_eq!(evicted, Some("a".to_string()));
        assert!(!pool.contains("a"));
        assert!(pool.total_memory() <= 100);
    }

    // B:evict_warm_engine_instance — verify contract "requires/ensures consistency for warm engine eviction"
    #[test]
    fn test_evict_engine_contract() {
        let pool = EnginePool::new(WarmEngineConfig { max_instances: 2, max_memory_mb: 512 });
        pool.warm("old", 10);
        pool.warm("new", 10);

        // ensures: engine_evicted — LRU order respected
        let evicted = pool.warm("newest", 10);
        assert_eq!(evicted, Some("old".to_string()));

        // ensures: lru_order_respected — "new" is still present
        assert!(pool.contains("new"));
        assert!(pool.contains("newest"));

        // ensures: memory_ceiling_enforced
        assert!(pool.total_memory() <= 512);
    }

    // M8: EnginePool thread safety — verify that EnginePool methods take &self
    // and the pool can be shared across threads via Arc<EnginePool> (no external Mutex).
    #[test]
    fn test_engine_pool_thread_safe_via_arc() {
        use std::sync::Arc;
        use std::thread;

        // EnginePool has an internal Mutex, so Arc<EnginePool> is Send + Sync.
        let pool = Arc::new(EnginePool::new(WarmEngineConfig {
            max_instances: 16,
            max_memory_mb: 512,
        }));

        let pool1 = Arc::clone(&pool);
        let pool2 = Arc::clone(&pool);

        let t1 = thread::spawn(move || {
            pool1.warm("ext-thread-1", 32);
            pool1.contains("ext-thread-1")
        });

        let t2 = thread::spawn(move || {
            pool2.warm("ext-thread-2", 64);
            pool2.contains("ext-thread-2")
        });

        let r1 = t1.join().expect("thread 1 panicked");
        let r2 = t2.join().expect("thread 2 panicked");

        assert!(r1, "thread 1 should see its own warmed instance");
        assert!(r2, "thread 2 should see its own warmed instance");

        // After both threads complete, the shared pool should contain both
        assert!(pool.contains("ext-thread-1"));
        assert!(pool.contains("ext-thread-2"));
        assert_eq!(pool.len(), 2);
    }
}
