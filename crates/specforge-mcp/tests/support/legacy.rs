//! The in-memory writers the tests used before `TestProject` (plan 10):
//! moved verbatim from `tests/support.rs`, deleted once no test calls them.

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

/// Make `diagnostics` what the served in-memory project reports last, in
/// order: its environment's surface registration conflicts, as a registry
/// build that refused or deduplicated surfaces reports them (check reports
/// them after everything else).
pub fn report(
    state: &mut specforge_mcp::state::McpState,
    diagnostics: Vec<specforge_common::Diagnostic>,
) {
    state.edit_environment(|env| env.registries.surface_diagnostics = diagnostics);
}
