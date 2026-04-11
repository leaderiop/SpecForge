use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_registry::{KindRegistry, FieldRegistry, EdgeRegistry, SurfaceContributions, SurfaceRegistryEntry};
use std::path::Path;

pub struct CompileResult {
    pub graph: Graph,
    pub diagnostics: Vec<Diagnostic>,
    pub kind_registry: KindRegistry,
    pub field_registry: FieldRegistry,
    pub edge_registry: EdgeRegistry,
    pub extension_info: Vec<(String, String)>,
    pub surface_entries: Vec<SurfaceRegistryEntry>,
    pub manifest_surfaces: Vec<(String, SurfaceContributions)>,
}

/// Compile a project using the full shared pipeline.
///
/// This delegates to `specforge_emitter::compile()` so MCP gets
/// identical results to `specforge check` and `specforge export`.
pub fn compile_project(project_root: &Path) -> CompileResult {
    let ctx = specforge_emitter::compile(project_root);
    CompileResult {
        graph: ctx.graph,
        diagnostics: ctx.diagnostics,
        kind_registry: ctx.kind_registry,
        field_registry: ctx.field_registry,
        edge_registry: ctx.edge_registry,
        extension_info: ctx.extension_info,
        surface_entries: ctx.surface_entries,
        manifest_surfaces: ctx.manifest_surfaces,
    }
}
