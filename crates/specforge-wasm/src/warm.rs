use std::collections::HashMap;

use crate::error::WasmError;
use crate::manifest::PackageManifest;
use crate::runtime::WasmRuntime;

/// Pool of warm Wasm package instances for LSP/long-running processes.
///
/// Keeps loaded packages alive across incremental rebuilds to avoid
/// cold-start compilation costs.
pub struct WarmInstancePool {
    runtime: Option<WasmRuntime>,
    /// Wasm hashes of currently loaded plugins, keyed by package name.
    loaded_hashes: HashMap<String, String>,
}

impl WarmInstancePool {
    pub fn new() -> Self {
        Self {
            runtime: None,
            loaded_hashes: HashMap::new(),
        }
    }

    /// Get the current runtime, if any plugins are loaded.
    pub fn runtime(&self) -> Option<&WasmRuntime> {
        self.runtime.as_ref()
    }

    /// Get a mutable reference to the runtime.
    pub fn runtime_mut(&mut self) -> Option<&mut WasmRuntime> {
        self.runtime.as_mut()
    }

    /// Load or reload plugins based on the current manifests.
    ///
    /// Only reloads plugins whose wasm hash has changed. Returns load errors.
    pub fn reconcile(
        &mut self,
        manifests: Vec<PackageManifest>,
    ) -> Vec<WasmError> {
        // Check if anything changed
        let new_hashes: HashMap<String, String> = manifests
            .iter()
            .filter_map(|m| {
                let wasm_bytes = std::fs::read(&m.wasm_path).ok()?;
                Some((m.package.clone(), crate::loader::compute_sha256(&wasm_bytes)))
            })
            .collect();

        if new_hashes == self.loaded_hashes {
            // Nothing changed, keep existing runtime
            return Vec::new();
        }

        // Rebuild runtime
        let mut runtime = WasmRuntime::new();
        let errors = runtime.load_packages(manifests);

        if errors.is_empty() {
            let _init = runtime.initialize_all();
        }

        self.loaded_hashes = new_hashes;
        self.runtime = Some(runtime);
        errors
    }

    /// Unload all plugins.
    pub fn shutdown(&mut self) {
        self.runtime = None;
        self.loaded_hashes.clear();
    }

    /// Check if any plugins are loaded.
    pub fn is_loaded(&self) -> bool {
        self.runtime.is_some()
    }
}

impl Default for WarmInstancePool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_new_empty() {
        let pool = WarmInstancePool::new();
        assert!(!pool.is_loaded());
        assert!(pool.runtime().is_none());
    }

    #[test]
    fn pool_reconcile_empty() {
        let mut pool = WarmInstancePool::new();
        let errors = pool.reconcile(vec![]);
        assert!(errors.is_empty());
    }

    #[test]
    fn pool_shutdown() {
        let mut pool = WarmInstancePool::new();
        pool.shutdown();
        assert!(!pool.is_loaded());
    }
}
