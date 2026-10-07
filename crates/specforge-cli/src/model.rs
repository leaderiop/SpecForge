use std::path::Path;

use specforge_ops::model::ModelOptions;
use specforge_ops::view::ProjectView;

use crate::pipeline;

/// `specforge model`: the model operation over the project compiled at
/// `path`; its W146 warnings go to stderr.
pub fn run(path: &Path, options: &ModelOptions) -> i32 {
    let (project, _runtime) = pipeline::compile_project(path);
    let outcome = specforge_ops::model::model(&ProjectView::of(&project), options);
    for warning in &outcome.warnings {
        eprintln!("{}", crate::export::render_plain(warning));
    }
    print!("{}", outcome.rendered);
    0
}
