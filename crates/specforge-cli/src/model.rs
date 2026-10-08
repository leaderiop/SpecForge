use std::path::Path;

use specforge_ops::model::ModelOptions;
use specforge_ops::view::ProjectView;

use crate::OutputFormat;
use crate::outcome::{Exit, Refusal};
use crate::pipeline;

/// `specforge model`: the model operation over the project compiled at
/// `path`. The model on stdout, the notices (I020) on stderr, a refusal as
/// `error[CODE]` with its hint (exit 1).
pub fn run(path: &Path, options: &ModelOptions) -> Exit {
    let (project, _runtime) = pipeline::compile_project(path);
    match specforge_ops::model::model(&ProjectView::of(&project), options) {
        Ok(outcome) => {
            for notice in &outcome.notices {
                eprintln!("{}", specforge_common::render_plain(notice));
            }
            print!("{}", outcome.document);
            Exit::Passed
        }
        Err(error) => Refusal::of(OutputFormat::Human).report(&error),
    }
}
