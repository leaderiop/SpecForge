use crate::DocumentBuffer;
use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_registry::{
    validation_engine::ValidationRulePattern,
    EdgeRegistry, FieldRegistry, KindRegistry,
};
use std::collections::HashMap;

/// Shared LSP server state: open documents, in-memory graph, registries, and diagnostics.
pub struct LspState {
    documents: HashMap<String, DocumentBuffer>,
    diagnostics: HashMap<String, Vec<Diagnostic>>,
    graph: Graph,
    kind_registry: KindRegistry,
    field_registry: FieldRegistry,
    edge_registry: EdgeRegistry,
    validation_patterns: Vec<ValidationRulePattern>,
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
            graph: Graph::new(),
            kind_registry: KindRegistry::new(),
            field_registry: FieldRegistry::new(),
            edge_registry: EdgeRegistry::new(),
            validation_patterns: Vec::new(),
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
        self.diagnostics.get(uri).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
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

    pub fn validation_patterns(&self) -> &[ValidationRulePattern] {
        &self.validation_patterns
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

    pub fn set_validation_patterns(&mut self, patterns: Vec<ValidationRulePattern>) {
        self.validation_patterns = patterns;
    }

    pub fn shutdown(&mut self) {
        self.shutdown = true;
        self.documents.clear();
        self.diagnostics.clear();
        self.graph = Graph::new();
        self.kind_registry = KindRegistry::new();
        self.field_registry = FieldRegistry::new();
        self.edge_registry = EdgeRegistry::new();
        self.validation_patterns = Vec::new();
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }
}

