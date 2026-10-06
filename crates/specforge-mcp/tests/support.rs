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
                field_type: ManifestFieldType::String,
                source_extension: "@test/ext".to_string(),
                proof_role: None,
                declared: specforge_registry::FieldDescriptor {
                    name: field.to_string(),
                    normative: field == "contract",
                    headline: true,
                    ..Default::default()
                },
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
            field_type: ManifestFieldType::ReferenceList,
            source_extension: "@test/ext".to_string(),
            proof_role: None,
            declared: specforge_registry::FieldDescriptor {
                name: field.to_string(),
                edge: Some(format!("{kind}_{field}")),
                target_kind: Some(target.to_string()),
                ..Default::default()
            },
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

/// The W004 rule requiring `kind`'s entities to declare obligations, as
/// the extension declaring `kind` registers it.
pub fn obligations_rule(
    kind: &str,
) -> (
    specforge_registry::validation_engine::ValidationRulePattern,
    String,
) {
    use specforge_registry::validation_engine::{ValidationPatternKind, ValidationRulePattern};
    (
        ValidationRulePattern {
            code: "W004".into(),
            severity: specforge_common::Severity::Warning,
            message_template: "{kind} '{id}' is testable but declares no verify obligations".into(),
            check: ValidationPatternKind::NoVerifyStatements,
            target_kind: Some(kind.into()),
            edge_type: None,
            edge_peer_kind: None,
            field: Some("verify".into()),
            constraint: None,
            wasm_function: None,
        },
        "@test/ext".into(),
    )
}

/// Make entities of `kind` owe obligations ([`obligations_rule`]): one
/// that declares none counts toward coverage instead of being exempt.
pub fn obligate(server: &mut McpServer, kind: &str) {
    server
        .state_mut()
        .edit_environment(|env| env.registries.rules.push(obligations_rule(kind)));
}
