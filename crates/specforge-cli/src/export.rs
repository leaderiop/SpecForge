use specforge_graph::build_graph;
use specforge_resolver::resolve_project;
use std::path::Path;

pub fn run(path: &Path, format: &str, scope: Option<&str>) -> i32 {
    let resolved = resolve_project(path);
    let spec_files: Vec<_> = resolved.files.iter().map(|f| f.spec_file.clone()).collect();
    let (graph, _diagnostics) = build_graph(&spec_files);

    if let Some(scope_id) = scope {
        let result = match format {
            "context" => specforge_emitter::emit_context_scoped(&graph, scope_id),
            _ => specforge_emitter::emit_json_scoped(&graph, scope_id),
        };
        match result {
            Ok(output) => {
                println!("{}", output);
                0
            }
            Err(err) => {
                eprintln!("{}", err);
                1
            }
        }
    } else {
        let output = match format {
            "brief" => specforge_emitter::emit_brief(&graph),
            "context" => specforge_emitter::emit_context(&graph),
            "dot" => specforge_emitter::emit_dot(&graph),
            _ => specforge_emitter::emit_graph(&graph),
        };
        println!("{}", output);
        0
    }
}
