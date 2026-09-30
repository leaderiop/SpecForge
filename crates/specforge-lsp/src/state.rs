use crate::DocumentBuffer;
use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_registry::{
    EdgeRegistry, FieldRegistry, KindRegistry, validation_engine::ValidationRulePattern,
};
use specforge_watch::IncrementalPipeline;
use std::collections::HashMap;

/// Shared LSP server state: open documents, the shared incremental pipeline
/// (graph + per-file parses + diagnostics), registries, and the published
/// per-URI diagnostics.
pub struct LspState {
    documents: HashMap<String, DocumentBuffer>,
    diagnostics: HashMap<String, Vec<Diagnostic>>,
    pipeline: IncrementalPipeline,
    kind_registry: KindRegistry,
    field_registry: FieldRegistry,
    edge_registry: EdgeRegistry,
    /// Patterns paired with their originating extension ("" for
    /// host-generated rules); the origin names the module that owns a
    /// custom rule's `wasm_function` export.
    validation_patterns: Vec<(ValidationRulePattern, String)>,
    /// Entity keyword -> extension name, derived from the loaded manifests.
    /// Mirrors the CLI's `known_extension_keywords` so I004 hints agree
    /// across surfaces (WASM-only migration, Phase 4).
    known_extension_keywords: HashMap<String, String>,
    /// The session's Wasm runtime, for custom-rule dispatch.
    runtime: Option<std::sync::Arc<specforge_component::ComponentRuntime>>,
    /// The project's spec root, for file-reference checks.
    spec_root: std::path::PathBuf,
    shutdown: bool,
}

impl Default for LspState {
    fn default() -> Self {
        Self::new()
    }
}

impl LspState {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            diagnostics: HashMap::new(),
            pipeline: IncrementalPipeline::empty(),
            kind_registry: KindRegistry::new(),
            field_registry: FieldRegistry::new(),
            edge_registry: EdgeRegistry::new(),
            validation_patterns: Vec::new(),
            known_extension_keywords: HashMap::new(),
            runtime: None,
            spec_root: std::path::PathBuf::new(),
            shutdown: false,
        }
    }

    pub fn open_document(&mut self, uri: &str, content: &str) {
        if self.shutdown {
            return;
        }
        self.documents.insert(
            uri.to_string(),
            DocumentBuffer::new(uri.to_string(), content.to_string()),
        );
    }

    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
        self.diagnostics.remove(uri);
    }

    pub fn is_open(&self, uri: &str) -> bool {
        self.documents.contains_key(uri)
    }

    pub fn document(&self, uri: &str) -> Option<&DocumentBuffer> {
        self.documents.get(uri)
    }

    pub fn document_mut(&mut self, uri: &str) -> Option<&mut DocumentBuffer> {
        self.documents.get_mut(uri)
    }

    pub fn open_uris(&self) -> Vec<&str> {
        let mut uris: Vec<&str> = self.documents.keys().map(|s| s.as_str()).collect();
        uris.sort();
        uris
    }

    pub fn apply_change(
        &mut self,
        uri: &str,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        new_text: &str,
    ) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.apply_change(start_line, start_col, end_line, end_col, new_text);
        }
    }

    pub fn set_diagnostics(&mut self, uri: &str, diags: Vec<Diagnostic>) {
        self.diagnostics.insert(uri.to_string(), diags);
    }

    pub fn diagnostics(&self, uri: &str) -> &[Diagnostic] {
        self.diagnostics
            .get(uri)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn graph(&self) -> &Graph {
        self.pipeline.graph()
    }

    pub fn graph_mut(&mut self) -> &mut Graph {
        self.pipeline.graph_mut()
    }

    pub fn pipeline(&self) -> &IncrementalPipeline {
        &self.pipeline
    }

    /// Take the pipeline out (for blocking compute off the async runtime).
    /// Until `set_pipeline`, the state keeps a copy of the last complete
    /// graph, so readers never see a half-applied update (an empty graph).
    pub fn take_pipeline(&mut self) -> IncrementalPipeline {
        let mut stand_in = IncrementalPipeline::empty();
        *stand_in.graph_mut() = self.pipeline.graph().clone();
        std::mem::replace(&mut self.pipeline, stand_in)
    }

    /// Put a previously taken pipeline back.
    pub fn set_pipeline(&mut self, pipeline: IncrementalPipeline) {
        self.pipeline = pipeline;
    }

    pub fn pipeline_mut(&mut self) -> &mut IncrementalPipeline {
        &mut self.pipeline
    }

    pub fn kind_registry(&self) -> &KindRegistry {
        &self.kind_registry
    }

    pub fn field_registry(&self) -> &FieldRegistry {
        &self.field_registry
    }

    pub fn edge_registry(&self) -> &EdgeRegistry {
        &self.edge_registry
    }

    pub fn validation_patterns(&self) -> &[(ValidationRulePattern, String)] {
        &self.validation_patterns
    }

    pub fn runtime(&self) -> Option<&std::sync::Arc<specforge_component::ComponentRuntime>> {
        self.runtime.as_ref()
    }

    pub fn spec_root(&self) -> &std::path::Path {
        &self.spec_root
    }

    pub fn set_spec_root(&mut self, spec_root: std::path::PathBuf) {
        self.spec_root = spec_root;
    }

    pub fn set_runtime(&mut self, runtime: specforge_component::ComponentRuntime) {
        self.runtime = Some(std::sync::Arc::new(runtime));
    }

    /// Replace the registries and validation patterns (called after loading extension manifests).
    pub fn set_registries(
        &mut self,
        kind_reg: KindRegistry,
        field_reg: FieldRegistry,
        edge_reg: EdgeRegistry,
    ) {
        self.kind_registry = kind_reg;
        self.field_registry = field_reg;
        self.edge_registry = edge_reg;
    }

    pub fn known_extension_keywords(&self) -> &HashMap<String, String> {
        &self.known_extension_keywords
    }

    pub fn set_known_extension_keywords(&mut self, map: HashMap<String, String>) {
        self.known_extension_keywords = map;
    }

    pub fn set_validation_patterns(&mut self, patterns: Vec<(ValidationRulePattern, String)>) {
        self.validation_patterns = patterns;
    }

    pub fn shutdown(&mut self) {
        self.shutdown = true;
        self.documents.clear();
        self.diagnostics.clear();
        self.pipeline = IncrementalPipeline::empty();
        self.kind_registry = KindRegistry::new();
        self.field_registry = FieldRegistry::new();
        self.edge_registry = EdgeRegistry::new();
        self.validation_patterns = Vec::new();
        self.runtime = None;
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }
}
