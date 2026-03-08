use specforge_graph::build_graph;
use specforge_resolver::resolve_project;
use std::path::Path;

pub fn run(path: &Path, entity: &str) -> i32 {
    let resolved = resolve_project(path);
    let spec_files: Vec<_> = resolved.files.iter().map(|f| f.spec_file.clone()).collect();
    let (graph, _diagnostics) = build_graph(&spec_files);

    match specforge_emitter::trace(&graph, entity) {
        Ok(chain) => {
            println!("{}", specforge_emitter::serialize_trace(&chain));
            0
        }
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    }
}
