use std::path::Path;

use specforge_component::ComponentRuntime;
use specforge_project::CompiledProject;

/// Compile the project at `path` with its extensions, and the runtime they
/// ran in: the one compile every command holds to build a project view
/// (`specforge_ops::view::ProjectView::of`), rooted at `path`. Later stages
/// (extension passes, source scanning, collectors) reuse the runtime
/// rather than building one per stage.
pub fn compile_project(path: &Path) -> (CompiledProject, ComponentRuntime) {
    let runtime = ComponentRuntime::with_user_cache();
    let project = CompiledProject::compile(path, Some(&runtime));
    (project, runtime)
}
