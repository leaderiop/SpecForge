use std::path::Path;

use specforge_ops::trace::TraceExpectations;

use crate::OutputFormat;
use crate::pipeline;

/// `specforge trace [entity]`: one entity's chain, or every entity's when
/// none is named. Expected edges come from the loaded extensions'
/// registries; the ones a chain lacks are reported as missing.
pub fn run(path: &Path, entity: Option<&str>, format: OutputFormat) -> i32 {
    let ctx = pipeline::compile(path);
    let expectations = TraceExpectations::from_registries(&ctx.field_registry, &ctx.kind_registry);

    let chains = match entity {
        Some(entity) => {
            match specforge_ops::trace::trace_with_expectations(&ctx.graph, entity, &expectations) {
                Ok(chain) => vec![chain],
                Err(err) => {
                    eprintln!("{err}");
                    return 1;
                }
            }
        }
        None => specforge_ops::trace::trace_all_with_expectations(&ctx.graph, &expectations),
    };

    match format {
        OutputFormat::Human => {
            let text: Vec<String> = chains
                .iter()
                .map(specforge_ops::trace::render_trace_human)
                .collect();
            print!("{}", text.join("\n"));
            0
        }
        OutputFormat::Json => {
            let json = match entity {
                Some(_) => specforge_ops::trace::serialize_trace(&chains[0]),
                None => specforge_ops::trace::serialize_trace_all(&chains),
            };
            match json {
                Ok(json) => {
                    println!("{json}");
                    0
                }
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            }
        }
    }
}
