use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_registry::{KindRegistry, FieldRegistry, EdgeRegistry, ManifestV2, SurfaceContributions, SurfaceRegistryEntry};
use specforge_wasm::WasmRuntime;
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
    pub manifests: Vec<ManifestV2>,
}

/// Compile a project using the full shared pipeline.
///
/// This delegates to `specforge_emitter::compile()` so MCP gets
/// identical results to `specforge check` and `specforge export`.
pub fn compile_project(project_root: &Path) -> CompileResult {
    let ctx = specforge_emitter::compile(project_root);
    from_ctx(ctx)
}

/// Compile a project with a specific runtime (or None for legacy manifest loading).
/// Used by tests that need to control the extension loading path.
pub fn compile_project_with_runtime(project_root: &Path, runtime: Option<&dyn WasmRuntime>) -> CompileResult {
    let ctx = specforge_emitter::compile_with_runtime(project_root, runtime);
    from_ctx(ctx)
}

fn from_ctx(ctx: specforge_emitter::CompilationContext) -> CompileResult {
    CompileResult {
        graph: ctx.graph,
        diagnostics: ctx.diagnostics,
        kind_registry: ctx.kind_registry,
        field_registry: ctx.field_registry,
        edge_registry: ctx.edge_registry,
        extension_info: ctx.extension_info,
        surface_entries: ctx.surface_entries,
        manifest_surfaces: ctx.manifest_surfaces,
        manifests: ctx.manifests,
    }
}
