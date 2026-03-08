use specforge_graph::build_graph;
use specforge_resolver::resolve_project;
use std::path::Path;

pub fn run(path: &Path, entity: &str, depth: usize, kind_filter: &[String]) -> i32 {
    let resolved = resolve_project(path);
    let spec_files: Vec<_> = resolved.files.iter().map(|f| f.spec_file.clone()).collect();
    let (graph, _diagnostics) = build_graph(&spec_files);

    let kinds: Vec<&str> = kind_filter.iter().map(|s| s.as_str()).collect();
    match specforge_emitter::query(&graph, entity, depth, &kinds) {
        Ok(output) => {
            println!("{}", output);
            0
        }
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    }
}
