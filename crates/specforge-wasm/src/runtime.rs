use specforge_common::Diagnostic;
use specforge_emitter::GeneratedFile;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::error::{WasmError, WasmTrapInfo};
use crate::host_functions::{self, HostContext, RegisteredEdge, RegisteredEntity};
use crate::loader::{self, LoadedPackage};
use crate::manifest::{PluginLifecycleState, PackageManifest};

/// Orchestrates loading, initializing, and calling Wasm packages.
pub struct WasmRuntime {
    plugins: Vec<PackageEntry>,
}

struct PackageEntry {
    loaded: LoadedPackage,
    ctx: Arc<Mutex<HostContext>>,
}

/// Results from initializing all plugins.
pub struct InitializeResult {
    pub entities: Vec<RegisteredEntity>,
    pub edges: Vec<RegisteredEdge>,
    pub diagnostics: Vec<Diagnostic>,
    pub errors: Vec<WasmError>,
}

/// Results from running validation across all plugins.
pub struct ValidateResult {
    pub diagnostics: Vec<Diagnostic>,
    pub errors: Vec<WasmError>,
}

/// Results from running a generator plugin.
pub struct GenerateResult {
    pub files: Vec<GeneratedFile>,
    pub diagnostics: Vec<Diagnostic>,
}

impl WasmRuntime {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Load plugins from their manifests. Returns errors for any that failed to load.
    pub fn load_packages(&mut self, manifests: Vec<PackageManifest>) -> Vec<WasmError> {
        let mut errors = Vec::new();

        for manifest in manifests {
            let (functions, ctx) =
                host_functions::build_host_functions(&manifest.package, manifest.sandbox.clone());

            match loader::load_wasm_module(manifest, functions) {
                Ok(loaded) => {
                    self.plugins.push(PackageEntry { loaded, ctx });
                }
                Err(e) => errors.push(e),
            }
        }

        errors
    }

    /// Initialize all loaded plugins by calling their `initialize` export.
    ///
    /// During initialization, plugins may call `register_entity` and `register_edge`.
    pub fn initialize_all(&mut self) -> InitializeResult {
        let mut all_entities = Vec::new();
        let mut all_edges = Vec::new();
        let mut all_diagnostics = Vec::new();
        let mut errors = Vec::new();

        for entry in &mut self.plugins {
            if entry.loaded.state == PluginLifecycleState::Failed {
                continue;
            }

            // Enable registration host functions
            {
                let mut ctx = entry.ctx.lock().unwrap();
                ctx.in_initialize = true;
            }

            // Check if plugin exports 'initialize'
            if entry.loaded.plugin.function_exists("initialize") {
                match entry.loaded.plugin.call::<&str, &str>("initialize", "") {
                    Ok(_) => {
                        entry.loaded.state = PluginLifecycleState::Initialized;
                    }
                    Err(e) => {
                        entry.loaded.state = PluginLifecycleState::Failed;
                        errors.push(WasmError::TrapCaught(WasmTrapInfo {
                            package: entry.loaded.manifest.package.clone(),
                            function: "initialize".to_string(),
                            message: e.to_string(),
                        }));
                        continue;
                    }
                }
            } else {
                entry.loaded.state = PluginLifecycleState::Initialized;
            }

            // Disable registration and collect results
            {
                let mut ctx = entry.ctx.lock().unwrap();
                ctx.in_initialize = false;
                all_entities.extend(ctx.take_registered_entities());
                all_edges.extend(ctx.take_registered_edges());
                all_diagnostics.extend(ctx.take_diagnostics());
            }

            entry.loaded.state = PluginLifecycleState::Ready;
        }

        InitializeResult {
            entities: all_entities,
            edges: all_edges,
            diagnostics: all_diagnostics,
            errors,
        }
    }

    /// Run validation across all ready plugins by calling their `validate` export.
    ///
    /// Each plugin receives the serialized graph JSON and may emit diagnostics.
    pub fn validate_all(&mut self, graph_json: &str) -> ValidateResult {
        let mut all_diagnostics = Vec::new();
        let mut errors = Vec::new();

        for entry in &mut self.plugins {
            if entry.loaded.state != PluginLifecycleState::Ready {
                continue;
            }

            // Set graph JSON in context
            {
                let mut ctx = entry.ctx.lock().unwrap();
                ctx.graph_json = graph_json.to_string();
            }

            if entry.loaded.plugin.function_exists("validate") {
                match entry.loaded.plugin.call::<&str, &str>("validate", "") {
                    Ok(_) => {}
                    Err(e) => {
                        errors.push(WasmError::TrapCaught(WasmTrapInfo {
                            package: entry.loaded.manifest.package.clone(),
                            function: "validate".to_string(),
                            message: e.to_string(),
                        }));
                    }
                }
            }

            // Collect diagnostics
            {
                let mut ctx = entry.ctx.lock().unwrap();
                all_diagnostics.extend(ctx.take_diagnostics());
            }
        }

        ValidateResult {
            diagnostics: all_diagnostics,
            errors,
        }
    }

    /// Run a generator plugin by calling its `generate` export.
    ///
    /// Returns emitted files and any diagnostics.
    pub fn generate(
        &mut self,
        plugin_name: &str,
        graph_json: &str,
        output_dir: &PathBuf,
    ) -> Result<GenerateResult, WasmError> {
        let entry = self
            .plugins
            .iter_mut()
            .find(|e| {
                e.loaded
                    .manifest
                    .generator
                    .as_ref()
                    .is_some_and(|g| g.name == plugin_name)
            })
            .ok_or_else(|| WasmError::WasmLoadFailed {
                package: plugin_name.to_string(),
                message: "generator plugin not found".to_string(),
            })?;

        if entry.loaded.state != PluginLifecycleState::Ready {
            return Err(WasmError::WasmLoadFailed {
                package: plugin_name.to_string(),
                message: format!("plugin in state {:?}, expected Ready", entry.loaded.state),
            });
        }

        // Set graph and output dir in context
        {
            let mut ctx = entry.ctx.lock().unwrap();
            ctx.graph_json = graph_json.to_string();
            ctx.output_dir = Some(output_dir.clone());
        }

        if entry.loaded.plugin.function_exists("generate") {
            entry
                .loaded
                .plugin
                .call::<&str, &str>("generate", "")
                .map_err(|e| {
                    WasmError::TrapCaught(WasmTrapInfo {
                        package: entry.loaded.manifest.package.clone(),
                        function: "generate".to_string(),
                        message: e.to_string(),
                    })
                })?;
        }

        let mut ctx = entry.ctx.lock().unwrap();
        let emitted = ctx.take_emitted_files();
        let diagnostics = ctx.take_diagnostics();

        let files = emitted
            .into_iter()
            .map(|ef| GeneratedFile {
                path: ef.path,
                content: ef.content,
            })
            .collect();

        Ok(GenerateResult { files, diagnostics })
    }

    /// Check if any loaded plugin provides a generator with the given name.
    pub fn has_generator(&self, name: &str) -> bool {
        self.plugins.iter().any(|e| {
            e.loaded
                .manifest
                .generator
                .as_ref()
                .is_some_and(|g| g.name == name)
        })
    }

    /// Get information about all loaded plugins.
    pub fn package_infos(&self) -> Vec<PackageInfo> {
        self.plugins
            .iter()
            .map(|e| PackageInfo {
                package: e.loaded.manifest.package.clone(),
                contributes: e.loaded.manifest.contributes.clone(),
                state: e.loaded.state,
                wasm_hash: e.loaded.wasm_hash.clone(),
                version: e.loaded.manifest.version.clone(),
            })
            .collect()
    }

    /// Get the number of loaded plugins.
    pub fn package_count(&self) -> usize {
        self.plugins.len()
    }

    /// Get loaded manifests for peer dependency validation.
    pub fn manifests(&self) -> Vec<&PackageManifest> {
        self.plugins
            .iter()
            .map(|e| &e.loaded.manifest)
            .collect()
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Public info about a loaded plugin.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub package: String,
    pub contributes: crate::manifest::PackageContributions,
    pub state: PluginLifecycleState,
    pub wasm_hash: String,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_new_empty() {
        let runtime = WasmRuntime::new();
        assert_eq!(runtime.package_count(), 0);
        assert!(!runtime.has_generator("test"));
    }

    #[test]
    fn runtime_load_no_manifests() {
        let mut runtime = WasmRuntime::new();
        let errors = runtime.load_packages(vec![]);
        assert!(errors.is_empty());
        assert_eq!(runtime.package_count(), 0);
    }

    #[test]
    fn runtime_initialize_empty() {
        let mut runtime = WasmRuntime::new();
        let result = runtime.initialize_all();
        assert!(result.entities.is_empty());
        assert!(result.edges.is_empty());
        assert!(result.diagnostics.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn runtime_validate_empty() {
        let mut runtime = WasmRuntime::new();
        let result = runtime.validate_all("{}");
        assert!(result.diagnostics.is_empty());
        assert!(result.errors.is_empty());
    }
}
