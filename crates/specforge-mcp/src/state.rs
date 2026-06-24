use specforge_common::{Diagnostic, ProjectConfig};
use specforge_graph::Graph;
use specforge_registry::{KindRegistry, FieldRegistry, EdgeRegistry, SurfaceRegistryEntry};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::{McpEvent, McpToolDescriptor, McpResourceDescriptor, McpPromptDescriptor};

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
    pub subscriptions: HashMap<String, Vec<Subscription>>,
    pub previous_diagnostics: Vec<Diagnostic>,
    pub tool_registry: Vec<McpToolDescriptor>,
    pub resource_registry: Vec<McpResourceDescriptor>,
    pub prompt_registry: Vec<McpPromptDescriptor>,
    pub events: Vec<McpEvent>,
    pub kind_registry: KindRegistry,
    pub field_registry: FieldRegistry,
    pub edge_registry: EdgeRegistry,
    pub extension_info: Vec<(String, String)>,
    pub surface_entries: Vec<SurfaceRegistryEntry>,
    pub manifests: Vec<specforge_registry::ManifestV2>,
    pub project_config: ProjectConfig,
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
            subscriptions: HashMap::new(),
            previous_diagnostics: Vec::new(),
            tool_registry: Vec::new(),
            resource_registry: Vec::new(),
            prompt_registry: Vec::new(),
            events: Vec::new(),
            kind_registry: KindRegistry::new(),
            field_registry: FieldRegistry::new(),
            edge_registry: EdgeRegistry::new(),
            extension_info: Vec::new(),
            surface_entries: Vec::new(),
            manifests: Vec::new(),
            project_config: ProjectConfig::default(),
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

    pub fn shutdown(&mut self) {
        self.phase = ServerPhase::ShuttingDown;
        self.subscriptions.clear();
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
