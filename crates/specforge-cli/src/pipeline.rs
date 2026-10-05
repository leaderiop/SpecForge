use std::path::Path;

use specforge_component::{ComponentRuntime, project_runtime};
use specforge_project::CompilationContext;

/// Compile a project and also return the runtime that produced the context,
/// so later stages (extension passes, source scanning) reuse one engine
/// instead of rebuilding per stage (audit C7-08: single runtime per run).
pub fn compile_with_runtime(path: &Path) -> (CompilationContext, ComponentRuntime) {
    let runtime = project_runtime(path);
    let ctx = specforge_project::CompiledProject::compile(path, Some(&runtime)).into_context();
    (ctx, runtime)
}

/// Compile the project at `path` with its extensions.
pub fn compile(path: &Path) -> CompilationContext {
    compile_with_runtime(path).0
}
