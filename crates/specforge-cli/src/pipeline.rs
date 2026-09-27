use std::path::Path;

use specforge_component::ComponentRuntime;

pub use specforge_component::project_runtime;
pub use specforge_emitter::compile::CompilationContext;

/// Compile a project and also return the runtime that produced the context,
/// so later stages (extension passes, source scanning) reuse one engine
/// instead of rebuilding per stage (audit C7-08: single runtime per run).
pub fn compile_with_runtime(path: &Path) -> (CompilationContext, ComponentRuntime) {
    let runtime = project_runtime(path);
    let ctx = specforge_emitter::compile::compile_with_runtime(path, Some(&runtime));
    (ctx, runtime)
}

pub fn compile(path: &Path) -> CompilationContext {
    let runtime = project_runtime(path);
    specforge_emitter::compile::compile_with_runtime(path, Some(&runtime))
}

/// Backwards-compat alias for the shared constructor.
pub fn build_runtime(path: &Path) -> ComponentRuntime {
    project_runtime(path)
}
