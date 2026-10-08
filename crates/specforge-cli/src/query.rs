use std::path::Path;

use specforge_ops::query::{QueryRequest, query};
use specforge_ops::view::ProjectView;

use crate::OutputFormat;
use crate::outcome::Refusal;
use crate::pipeline;

/// `specforge query <entity>`: the query read view over the project
/// compiled at `path`. The document on stdout; the notices (I020) on
/// stderr; a refusal as `error[CODE]` with the closest entity as its hint
/// (exit 1).
pub fn run(path: &Path, request: &QueryRequest) -> i32 {
    let (project, _runtime) = pipeline::compile_project(path);
    match query(&ProjectView::of(&project), request) {
        Ok(outcome) => {
            for notice in &outcome.notices {
                eprintln!("{}", specforge_common::render_plain(notice));
            }
            println!("{}", outcome.document);
            0
        }
        Err(error) => Refusal::of(OutputFormat::Human).report(&error),
    }
}
