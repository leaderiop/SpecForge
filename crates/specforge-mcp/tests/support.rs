//! Environments the tests share.

use specforge_mcp::McpServer;
use specforge_registry::{FieldRegistryEntry, ManifestFieldType};

/// Declare `contract` and `status` headline fields of `kind` in the served
/// environment, as `@specforge/software` declares them on `behavior`, so
/// the context export lifts them.
pub fn declare_headline_fields(server: &mut McpServer, kind: &str) {
    server.state_mut().edit_environment(|env| {
        for field in ["contract", "status"] {
            env.registries.fields.register(FieldRegistryEntry {
                kind_name: kind.to_string(),
                field_name: field.to_string(),
                description: None,
                field_type: ManifestFieldType::String,
                source_extension: "@test/ext".to_string(),
                edge: None,
                target_kind: None,
                file_reference: false,
                required: false,
                inverse_of: None,
                normative: field == "contract",
                exempts_obligations: false,
                headline: true,
                derived_from: None,
                proof_role: None,
            });
        }
    });
}

/// Declare a reference-list field `kind.field -> target` in the served
/// environment, as the extension declaring `kind` would: an edge the trace
/// expects entities of `kind` to have.
pub fn declare_reference(server: &mut McpServer, kind: &str, field: &str, target: &str) {
    server.state_mut().edit_environment(|env| {
        env.registries.fields.register(FieldRegistryEntry {
            kind_name: kind.to_string(),
            field_name: field.to_string(),
            description: None,
            field_type: ManifestFieldType::ReferenceList,
            source_extension: "@test/ext".to_string(),
            edge: Some(format!("{kind}_{field}")),
            target_kind: Some(target.to_string()),
            file_reference: false,
            required: false,
            inverse_of: None,
            normative: false,
            exempts_obligations: false,
            headline: false,
            derived_from: None,
            proof_role: None,
        });
    });
}

/// Serve the graph the server serves now, as built in memory, at `root`:
/// a test that hands the server a graph and a project directory beside
/// it. The graph is never refreshed from disk; a call that writes files
/// under `root` serves the project on disk there.
pub fn serve_in_memory_at(state: &mut specforge_mcp::state::McpState, root: &std::path::Path) {
    let graph = state.graph().clone();
    let diagnostics = state.session().graph_diagnostics();
    state.serve_in_memory_at(Some(root.to_path_buf()), graph, diagnostics);
}

/// The update of a served project's sources that changed its graph by
/// `delta` (as the session reports one).
pub fn update_of(delta: specforge_project::GraphDelta) -> specforge_project::Update {
    specforge_project::Update {
        kind: specforge_project::UpdateKind::Sources,
        delta,
        rebuilt_files: Vec::new(),
        changed_diagnostic_files: Vec::new(),
        diagnostics: Vec::new(),
        verification: None,
    }
}
