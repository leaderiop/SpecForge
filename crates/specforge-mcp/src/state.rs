use specforge_common::{Diagnostic, ProjectConfig};
use specforge_graph::Graph;
use specforge_registry::{EdgeRegistry, FieldRegistry, KindRegistry, SurfaceRegistryEntry};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::{McpEvent, McpPromptDescriptor, McpResourceDescriptor, McpToolDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerPhase {
    Uninitialized,
    Initialized,
    ShuttingDown,
}

pub struct McpState {
    pub phase: ServerPhase,
    /// The protocol revision `initialize` negotiated (the latest one the
    /// server speaks until then).
    pub protocol_version: &'static str,
    pub graph: Graph,
    pub diagnostics: Vec<Diagnostic>,
    pub project_root: Option<PathBuf>,
    /// Where the compiled project's `.spec` files live: spans are relative
    /// to it (the project root unless `spec_root` is configured).
    pub spec_root: Option<PathBuf>,
    /// Project compiled when the client's `initialize` names no `projectRoot`
    /// (the `specforge mcp <path>` argument).
    pub default_project_root: Option<PathBuf>,
    pub subscriptions: HashMap<String, Vec<Subscription>>,
    pub previous_diagnostics: Vec<Diagnostic>,
    pub tool_registry: Vec<McpToolDescriptor>,
    pub resource_registry: Vec<McpResourceDescriptor>,
    pub prompt_registry: Vec<McpPromptDescriptor>,
    pub events: Vec<McpEvent>,
    /// Server→client notifications queued for subscribed channels (C9-01),
    /// drained by the host loop via `pending_notifications`.
    pub notification_outbox: Vec<serde_json::Value>,
    pub kind_registry: KindRegistry,
    pub field_registry: FieldRegistry,
    pub edge_registry: EdgeRegistry,
    pub extension_info: Vec<(String, String)>,
    pub surface_entries: Vec<SurfaceRegistryEntry>,
    pub manifests: Vec<specforge_registry::ManifestV2>,
    /// The extensions' validation rules, with the extension declaring each.
    pub rules: Vec<(
        specforge_registry::validation_engine::ValidationRulePattern,
        String,
    )>,
    pub project_config: ProjectConfig,
    /// When the current graph was compiled. Compared against the watch
    /// snapshot marker mtime to detect staleness (C9-07).
    pub loaded_at: Option<std::time::SystemTime>,
    /// The Wasm runtime extensions run in, when the host supplies one; by
    /// default each compile builds the project's runtime
    /// (`specforge_component::project_runtime`).
    pub extension_runtime: Option<std::sync::Arc<dyn specforge_wasm::WasmRuntime>>,
    /// The served project's runtime for extension calls (tools, resources,
    /// passes, hooks): built on the first call that needs it and kept until
    /// the project is compiled again, so its modules match the compile.
    served_runtime: std::sync::Mutex<Option<std::sync::Arc<dyn specforge_wasm::WasmRuntime>>>,
}

impl McpState {
    /// Path of the watch snapshot marker for this project, if configured.
    pub fn snapshot_marker(&self) -> Option<std::path::PathBuf> {
        self.project_root
            .as_ref()
            .map(|root| root.join(".specforge").join("graph.json"))
    }

    /// Recompile the served project when watch has written a newer
    /// snapshot (C9-07). No-op without a project root, without a snapshot,
    /// or when fresh.
    pub fn refresh_if_stale(&mut self) {
        let Some(marker) = self.snapshot_marker() else {
            return;
        };
        let Ok(meta) = std::fs::metadata(&marker) else {
            return;
        };
        let mtime = meta.modified().ok();
        let stale = match (self.loaded_at, mtime) {
            (Some(loaded), Some(m)) => m > loaded,
            (None, _) => true, // never compiled against a snapshot
            _ => false,
        };
        if !stale {
            return;
        }
        if let Some(root) = self.project_root.clone() {
            self.recompile(&root);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub client_id: String,
    pub channel: String,
}

impl Default for McpState {
    fn default() -> Self {
        Self::new()
    }
}

impl McpState {
    pub fn new() -> Self {
        Self {
            phase: ServerPhase::Uninitialized,
            protocol_version: crate::lifecycle::LATEST_PROTOCOL_VERSION,
            graph: Graph::new(),
            diagnostics: Vec::new(),
            project_root: None,
            spec_root: None,
            default_project_root: None,
            subscriptions: HashMap::new(),
            previous_diagnostics: Vec::new(),
            tool_registry: Vec::new(),
            resource_registry: Vec::new(),
            prompt_registry: Vec::new(),
            events: Vec::new(),
            kind_registry: KindRegistry::new(),
            notification_outbox: Vec::new(),
            field_registry: FieldRegistry::new(),
            edge_registry: EdgeRegistry::new(),
            extension_info: Vec::new(),
            surface_entries: Vec::new(),
            manifests: Vec::new(),
            rules: Vec::new(),
            project_config: ProjectConfig::default(),
            loaded_at: None,
            extension_runtime: None,
            served_runtime: std::sync::Mutex::new(None),
        }
    }

    /// The runtime extensions of the project at `root` run in: the host's,
    /// or the project's own. The served project's is built once per
    /// compile, on the first call that needs it; another project's is built
    /// for the call.
    pub fn wasm_runtime(
        &self,
        root: &std::path::Path,
    ) -> std::sync::Arc<dyn specforge_wasm::WasmRuntime> {
        if self.extension_runtime.is_some() || self.project_root.as_deref() != Some(root) {
            return self.fresh_runtime(root);
        }
        let mut served = self
            .served_runtime
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::sync::Arc::clone(served.get_or_insert_with(|| self.fresh_runtime(root)))
    }

    /// Whether the served project's runtime has been built since it was
    /// last compiled.
    pub fn has_loaded_runtime(&self) -> bool {
        self.served_runtime
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }

    /// A runtime for `root` built now: the host's, or a new one of the
    /// project's own.
    fn fresh_runtime(
        &self,
        root: &std::path::Path,
    ) -> std::sync::Arc<dyn specforge_wasm::WasmRuntime> {
        match &self.extension_runtime {
            Some(runtime) => std::sync::Arc::clone(runtime),
            None => std::sync::Arc::new(specforge_component::project_runtime(root)),
        }
    }

    /// Compile the project at `root` with its extensions in [`Self::wasm_runtime`].
    pub fn compile(&self, root: &std::path::Path) -> specforge_project::CompilationContext {
        self.compile_project(root).into_context()
    }

    pub fn is_initialized(&self) -> bool {
        self.phase == ServerPhase::Initialized
    }

    /// Whether the session accepts JSON-RPC batches: only a 2025-03-26
    /// session does, since later revisions removed batching.
    pub fn accepts_batches(&self) -> bool {
        self.is_initialized()
            && self.protocol_version == crate::lifecycle::BATCHING_PROTOCOL_VERSION
    }

    /// Whether tool results carry `structuredContent` (2025-06-18 on).
    pub fn sends_structured_content(&self) -> bool {
        self.protocol_version >= crate::lifecycle::STRUCTURED_CONTENT_PROTOCOL_VERSION
    }

    /// Record an event. Object payloads without a `timestamp` get one (RFC
    /// 3339, UTC) — except `mcp_initialized`, whose spec payload has none.
    pub fn push_event(&mut self, name: impl Into<String>, mut params: serde_json::Value) {
        let name = name.into();
        if name != "mcp_initialized"
            && let Some(object) = params.as_object_mut()
            && !object.contains_key("timestamp")
        {
            let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            object.insert("timestamp".into(), serde_json::Value::String(now));
        }
        self.events.push(McpEvent { name, params });
    }

    /// Compile the project at `root` with its extensions in a runtime built
    /// for it, without serving it: the modules are what is on disk now.
    pub fn compile_project(&self, root: &std::path::Path) -> specforge_project::CompiledProject {
        let runtime = self.fresh_runtime(root);
        specforge_project::CompiledProject::compile(root, Some(runtime.as_ref()))
    }

    /// Whether `root` names a project other than the one this server
    /// serves. With no project served yet, no path is another's.
    pub fn serves_other_than(&self, root: &std::path::Path) -> bool {
        let canonical =
            |p: &std::path::Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        self.project_root
            .as_deref()
            .is_some_and(|served| canonical(served) != canonical(root))
    }

    /// Serve `project`, compiled from `root`. Its graph, diagnostics,
    /// registries, config, and extension tools and resources replace the
    /// previous project's all at once: the tools and resources listed are
    /// the defaults plus what this project's extensions contribute, so
    /// nothing a previous compile contributed survives, and nothing is
    /// listed twice. Subscribed clients learn what changed. The one place
    /// a compile becomes the served project (initialize, a stale refresh,
    /// validate, analyze, and the mutation tools all come through here).
    pub fn install(&mut self, root: &std::path::Path, project: specforge_project::CompiledProject) {
        let diagnostics = project.diagnostics();
        let specforge_project::CompiledProject { env, graph, .. } = project;
        let specforge_project::Environment {
            config,
            spec_root,
            registries,
            ..
        } = env;
        let previous_graph = std::mem::replace(&mut self.graph, graph);
        let previous_diagnostics = std::mem::replace(&mut self.diagnostics, diagnostics);
        self.kind_registry = registries.kinds;
        self.field_registry = registries.fields;
        self.edge_registry = registries.edges;
        self.extension_info = registries.extension_info;
        self.manifests = registries.manifests;
        self.rules = registries.rules;
        self.surface_entries = registries.surfaces;
        self.project_config = config;
        self.spec_root = Some(spec_root);
        self.project_root = Some(root.to_path_buf());
        self.loaded_at = Some(std::time::SystemTime::now());
        // The next extension call loads the modules this compile used.
        *self
            .served_runtime
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;

        crate::registry::register_defaults(self);
        crate::registry::register_extension_surfaces(self, &registries.manifest_surfaces);
        crate::notifications::enqueue_compile_notifications(
            self,
            &previous_graph,
            &previous_diagnostics,
        );
    }

    /// Compile `root` afresh and serve it ([`Self::install`]).
    pub fn recompile(&mut self, root: &std::path::Path) {
        let project = self.compile_project(root);
        self.install(root, project);
    }

    pub fn shutdown(&mut self) {
        self.phase = ServerPhase::ShuttingDown;
        let clients: std::collections::BTreeSet<String> = self
            .subscriptions
            .values()
            .flatten()
            .map(|s| s.client_id.clone())
            .collect();
        for client in clients {
            crate::subscriptions::unsubscribe_all(self, &client);
        }
        self.subscriptions.clear();
        // The outbox stays: the host drains it after the shutdown response.
        self.previous_diagnostics = std::mem::take(&mut self.diagnostics);
        self.graph = Graph::new();
        self.project_root = None;
        self.kind_registry = KindRegistry::new();
        self.field_registry = FieldRegistry::new();
        self.edge_registry = EdgeRegistry::new();
        self.extension_info.clear();
        self.surface_entries.clear();
        self.manifests.clear();
    }
}
