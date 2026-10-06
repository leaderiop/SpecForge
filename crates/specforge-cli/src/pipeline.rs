use std::path::Path;

use specforge_component::{ComponentRuntime, project_runtime};
use specforge_project::{CompilationContext, CompiledProject};

/// Compile the project at `path` with its extensions, and the runtime they
/// ran in: what the read commands hold to build a project view
/// (`specforge_ops::view::ProjectView::of`), rooted at `path`.
pub fn compile_project(path: &Path) -> (CompiledProject, ComponentRuntime) {
    let runtime = project_runtime(path);
    let project = CompiledProject::compile(path, Some(&runtime));
    (project, runtime)
}

/// Compile a project and also return the runtime that produced the context,
/// so later stages (extension passes, source scanning) reuse one engine
/// instead of rebuilding per stage (audit C7-08: single runtime per run).
pub fn compile_with_runtime(path: &Path) -> (CompilationContext, ComponentRuntime) {
    let (project, runtime) = compile_project(path);
    (project.into_context(), runtime)
}

/// Compile the project at `path` with its extensions.
pub fn compile(path: &Path) -> CompilationContext {
    compile_with_runtime(path).0
}
