use std::path::Path;
use std::sync::Arc;

use specforge_component::ComponentRuntime;
use specforge_project::CompiledProject;

/// Compile the project at `path` with its extensions loaded into the
/// component runtime (with the per-user compile cache): the one compile
/// every command holds to build a project view
/// (`specforge_ops::view::ProjectView::of`), rooted at `path`. The compiled
/// project's environment holds that runtime, which every operation over its
/// view calls extensions in.
pub fn compile_project(path: &Path) -> CompiledProject {
    CompiledProject::compile(path, Some(Arc::new(ComponentRuntime::with_user_cache())))
}
