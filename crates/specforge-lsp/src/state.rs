use crate::DocumentBuffer;
use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_registry::{
    EdgeRegistry, FieldRegistry, KindRegistry, RegistryBuild,
    validation_engine::ValidationRulePattern,
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
    /// The registries, rules and derived inputs built from the loaded
    /// extensions: the same `build_registries` result `specforge check`
    /// uses.
    registries: RegistryBuild,
    /// What loading the extensions reported (E028, manifest checks), ahead
    /// of the registry build's own diagnostics.
    load_diagnostics: Vec<Diagnostic>,
    /// The session's Wasm runtime, for custom-rule dispatch.
    runtime: Option<std::sync::Arc<specforge_component::ComponentRuntime>>,
    /// The project's spec root, for file-reference checks.
    spec_root: std::path::PathBuf,
    /// [`LspState::token_signature`] as of the last recompile, so a
    /// recompile can tell whether the client's semantic tokens went stale.
    last_token_signature: u64,
    shutdown: bool,
}

impl Default for LspState {
    fn default() -> Self {
        Self::new()
    }
}

impl LspState {
    pub fn new() -> Self {
        let mut state = Self {
            documents: HashMap::new(),
            diagnostics: HashMap::new(),
            pipeline: IncrementalPipeline::empty(),
            registries: RegistryBuild::default(),
            load_diagnostics: Vec::new(),
            runtime: None,
            spec_root: std::path::PathBuf::new(),
            last_token_signature: 0,
            shutdown: false,
        };
        state.last_token_signature = state.token_signature();
        state
    }

    /// A digest of everything in the graph and registries that semantic
    /// tokens depend on beyond a document's own text: each entity's ID,
    /// kind and title, and each kind's `semantic_token` classification.
    /// Spans are left out, so an edit that only moves text (whitespace)
    /// keeps the signature.
    pub fn token_signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for node in self.graph().nodes() {
            node.id.raw.as_str().hash(&mut hasher);
            node.kind.raw.as_str().hash(&mut hasher);
            node.title.hash(&mut hasher);
        }
        let mut kinds: Vec<(&String, Option<&String>)> = self
            .registries
            .kinds
            .iter()
            .map(|(keyword, entry)| (keyword, entry.semantic_token.as_ref()))
            .collect();
        kinds.sort();
        kinds.hash(&mut hasher);
        hasher.finish()
    }

    /// Record the current [`LspState::token_signature`]; true when it
    /// differs from the one recorded at the previous recompile.
    pub fn record_token_signature(&mut self) -> bool {
        let signature = self.token_signature();
        let changed = signature != self.last_token_signature;
        self.last_token_signature = signature;
        changed
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
        &self.registries.kinds
    }

    pub fn field_registry(&self) -> &FieldRegistry {
        &self.registries.fields
    }

    pub fn edge_registry(&self) -> &EdgeRegistry {
        &self.registries.edges
    }

    /// Patterns paired with their originating extension ("" for
    /// host-generated rules); the origin names the module that owns a
    /// custom rule's `wasm_function` export.
    pub fn validation_patterns(&self) -> &[(ValidationRulePattern, String)] {
        &self.registries.rules
    }

    /// Everything built from the loaded extensions.
    pub fn registries(&self) -> &RegistryBuild {
        &self.registries
    }

    /// What loading the extensions and building the registries reported,
    /// in the order `specforge check` reports it: load, registry build,
    /// then surface conflicts.
    pub fn environment_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.load_diagnostics.clone();
        diagnostics.extend(self.registries.registry_diagnostics.iter().cloned());
        diagnostics.extend(self.registries.surface_diagnostics.iter().cloned());
        diagnostics
    }

    /// Replace the whole extension environment at once: registries, rules,
    /// load diagnostics and runtime. Nothing of the previous environment
    /// survives, so removing an extension removes its kinds.
    pub fn set_environment(
        &mut self,
        registries: RegistryBuild,
        load_diagnostics: Vec<Diagnostic>,
        runtime: Option<specforge_component::ComponentRuntime>,
    ) {
        self.registries = registries;
        self.load_diagnostics = load_diagnostics;
        self.runtime = runtime.map(std::sync::Arc::new);
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
        self.registries.kinds = kind_reg;
        self.registries.fields = field_reg;
        self.registries.edges = edge_reg;
    }

    /// Entity keyword -> extension name, derived from the loaded manifests.
    pub fn known_extension_keywords(&self) -> &HashMap<String, String> {
        &self.registries.keyword_owners
    }

    pub fn set_known_extension_keywords(&mut self, map: HashMap<String, String>) {
        self.registries.keyword_owners = map;
    }

    pub fn set_validation_patterns(&mut self, patterns: Vec<(ValidationRulePattern, String)>) {
        self.registries.rules = patterns;
    }

    pub fn shutdown(&mut self) {
        self.shutdown = true;
        self.documents.clear();
        self.diagnostics.clear();
        self.pipeline = IncrementalPipeline::empty();
        self.registries = RegistryBuild::default();
        self.load_diagnostics = Vec::new();
        self.runtime = None;
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }
}
