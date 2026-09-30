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
    pub graph: Graph,
    pub diagnostics: Vec<Diagnostic>,
    pub project_root: Option<PathBuf>,
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
    pub project_config: ProjectConfig,
    /// When the current graph was compiled. Compared against the watch
    /// snapshot marker mtime to detect staleness (C9-07).
    pub loaded_at: Option<std::time::SystemTime>,
}

impl McpState {
    /// Path of the watch snapshot marker for this project, if configured.
    pub fn snapshot_marker(&self) -> Option<std::path::PathBuf> {
        self.project_root
            .as_ref()
            .map(|root| root.join(".specforge").join("graph.json"))
    }

    /// Rebuild the graph when watch has written a newer snapshot (C9-07).
    /// No-op without a project root, without a snapshot, or when fresh.
    pub fn refresh_if_stale(&mut self) {
        use std::time::SystemTime;
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
            let previous_graph = self.graph.clone();
            let previous_diagnostics = self.diagnostics.clone();
            let compiled = crate::compile::compile_project(&root);
            self.graph = compiled.graph;
            self.diagnostics = compiled.diagnostics;
            self.kind_registry = compiled.kind_registry;
            self.field_registry = compiled.field_registry;
            self.edge_registry = compiled.edge_registry;
            self.extension_info = compiled.extension_info;
            self.manifests = compiled.manifests;
            self.loaded_at = Some(SystemTime::now());
            // Subscribed clients learn what changed (C9-01).
            crate::notifications::enqueue_compile_notifications(
                self,
                &previous_graph,
                &previous_diagnostics,
            );
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
            graph: Graph::new(),
            diagnostics: Vec::new(),
            project_root: None,
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
            project_config: ProjectConfig::default(),
            loaded_at: None,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.phase == ServerPhase::Initialized
    }

    pub fn push_event(&mut self, name: impl Into<String>, params: serde_json::Value) {
        self.events.push(McpEvent {
            name: name.into(),
            params,
        });
    }

    /// Compile `root` afresh into this state: graph, diagnostics, registries
    /// and extension surfaces. Subscribed clients learn what changed.
    pub fn recompile(&mut self, root: &std::path::Path) {
        let previous_graph = self.graph.clone();
        let previous_diagnostics = self.diagnostics.clone();
        let result = crate::compile::compile_project(root);
        self.graph = result.graph;
        self.diagnostics = result.diagnostics;
        self.kind_registry = result.kind_registry;
        self.field_registry = result.field_registry;
        self.edge_registry = result.edge_registry;
        self.extension_info = result.extension_info;
        self.surface_entries = result.surface_entries;
        self.manifests = result.manifests;
        self.loaded_at = Some(std::time::SystemTime::now());

        // Re-register extension surfaces (remove old extension tools/resources first)
        self.tool_registry
            .retain(|t| t.category.as_deref() != Some("extension"));
        self.resource_registry.retain(|r| {
            // Keep core resources, remove extension-added ones
            r.uri.starts_with("specforge://") && !r.uri.starts_with("specforge://ext/")
        });
        crate::registry::register_extension_surfaces(self, &result.manifest_surfaces);
        crate::notifications::enqueue_compile_notifications(
            self,
            &previous_graph,
            &previous_diagnostics,
        );
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
