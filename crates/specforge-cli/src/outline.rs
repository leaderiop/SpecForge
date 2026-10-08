use std::path::Path;

use specforge_ops::model::OutlineOptions;
use specforge_ops::view::ProjectView;

use crate::outcome::Exit;
use crate::pipeline;

/// `specforge outline`: the outline operation over the project compiled at
/// `path`.
pub fn run(path: &Path, options: &OutlineOptions) -> Exit {
    let (project, _runtime) = pipeline::compile_project(path);
    print!(
        "{}",
        specforge_ops::model::outline(&ProjectView::of(&project), options)
    );
    Exit::Passed
}
