use std::path::Path;

use crate::pipeline;

pub fn run(path: &Path, entity: &str) -> i32 {
    let ctx = pipeline::compile(path);

    match specforge_emitter::trace(&ctx.graph, entity) {
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
