use std::collections::VecDeque;

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
#[derive(Debug)]
pub struct EnginePool {
    instances: VecDeque<WarmInstance>,
    config: WarmEngineConfig,
}

impl EnginePool {
    pub fn new(config: WarmEngineConfig) -> Self {
        Self {
            instances: VecDeque::new(),
            config,
        }
    }

    /// Warm (add or refresh) an engine instance.
    /// Returns the evicted instance name if eviction was needed.
    pub fn warm(&mut self, extension_name: &str, memory_mb: u32) -> Option<String> {
        // Remove existing entry for this extension (refresh to front)
        self.instances.retain(|i| i.extension_name != extension_name);

        let mut evicted = None;

        // Check capacity
        if self.instances.len() as u32 >= self.config.max_instances {
            evicted = self.instances.pop_front().map(|i| i.extension_name);
        }

        // Check memory ceiling
        while self.total_memory() + memory_mb > self.config.max_memory_mb
            && !self.instances.is_empty()
        {
            evicted = self.instances.pop_front().map(|i| i.extension_name);
        }

        self.instances.push_back(WarmInstance {
            extension_name: extension_name.to_string(),
            memory_mb,
        });

        evicted
    }

    /// Evict and return the least-recently-used instance.
    pub fn evict_lru(&mut self) -> Option<String> {
        self.instances.pop_front().map(|i| i.extension_name)
    }

    /// Remove a specific extension from the pool.
    pub fn remove(&mut self, extension_name: &str) -> bool {
        let before = self.instances.len();
        self.instances.retain(|i| i.extension_name != extension_name);
        self.instances.len() < before
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub fn total_memory(&self) -> u32 {
        self.instances.iter().map(|i| i.memory_mb).sum()
    }

    pub fn contains(&self, extension_name: &str) -> bool {
        self.instances.iter().any(|i| i.extension_name == extension_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- warm_wasm_engine_instance --

    // B:warm_wasm_engine_instance — verify unit "warm instance reused across compilations"
    #[test]
    fn test_warm_instance_reused_across_compilations() {
        let mut pool = EnginePool::new(WarmEngineConfig::default());
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
        let mut pool = EnginePool::new(WarmEngineConfig::default());
        pool.warm("ext-a", 32);
        pool.warm("ext-b", 32);

        assert!(pool.remove("ext-a"));
        assert!(!pool.contains("ext-a"));
        assert!(pool.contains("ext-b"));
    }

    // B:warm_wasm_engine_instance — verify contract "requires/ensures consistency for warm engine instance management"
    #[test]
    fn test_warm_engine_contract() {
        let mut pool = EnginePool::new(WarmEngineConfig { max_instances: 2, max_memory_mb: 100 });

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
        let mut pool = EnginePool::new(WarmEngineConfig { max_instances: 2, max_memory_mb: 512 });
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
        let mut pool = EnginePool::new(WarmEngineConfig { max_instances: 10, max_memory_mb: 100 });
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
        let mut pool = EnginePool::new(WarmEngineConfig { max_instances: 2, max_memory_mb: 512 });
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
}
