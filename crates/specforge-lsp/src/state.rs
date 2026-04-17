use crate::DocumentBuffer;
use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_registry::{populate_registries, EdgeRegistry, FieldRegistry, KindRegistry};
use specforge_wasm::protocol::{
    load_protocol_extension, protocol_extension_to_manifest, ProtocolHost,
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
    shutdown: bool,
}

impl Default for LspState {
    fn default() -> Self {
        Self::new()
    }
}

impl LspState {
    pub fn new() -> Self {
        // Eagerly load default registries from built-in extensions so that
        // keyword completion, semantic tokens, and code actions work from
        // the first request — no hardcoded fallback constants needed.
        let (kind_registry, field_registry, edge_registry) = load_default_registries();

        Self {
            documents: HashMap::new(),
            diagnostics: HashMap::new(),
            graph: Graph::new(),
            kind_registry,
            field_registry,
            edge_registry,
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

    /// Replace the registries (called after loading extension manifests).
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

    pub fn shutdown(&mut self) {
        self.shutdown = true;
        self.documents.clear();
        self.diagnostics.clear();
        self.graph = Graph::new();
        self.kind_registry = KindRegistry::new();
        self.field_registry = FieldRegistry::new();
        self.edge_registry = EdgeRegistry::new();
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }
}

/// Load default registries from all built-in extensions via the protocol pipeline.
/// Returns (KindRegistry, FieldRegistry, EdgeRegistry) pre-populated with all
/// built-in entity kinds, fields, and edge types.
fn load_default_registries() -> (KindRegistry, FieldRegistry, EdgeRegistry) {
    let runtime = specforge_emitter::builtins::default_runtime();
    let host = ProtocolHost::new(&runtime);

    let builtins = [
        "@specforge/product",
        "@specforge/software",
        "@specforge/governance",
        "@specforge/formal",
    ];

    let mut manifests = Vec::new();
    for name in &builtins {
        if let Ok(proto_ext) = load_protocol_extension(&host, name) {
            manifests.push(protocol_extension_to_manifest(&proto_ext));
        }
    }

    let (kind_reg, field_reg, edge_reg, _diags) = populate_registries(&manifests);
    (kind_reg, field_reg, edge_reg)
}
