use std::path::Path;

use crate::pipeline;

pub fn run(path: &Path, entity: &str, depth: usize, kind_filter: &[String]) -> i32 {
    let ctx = pipeline::compile(path);

    let kinds: Vec<&str> = kind_filter.iter().map(|s| s.as_str()).collect();
    match specforge_emitter::query(&ctx.graph, entity, depth, &kinds) {
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
