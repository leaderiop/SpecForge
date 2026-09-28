use specforge_emitter::{EmitFormat, EmitOptions, SchemaVersion, emit, generate_schema};
use std::path::Path;

use crate::pipeline;

fn parse_format(format: &str) -> EmitFormat {
    match format {
        "brief" => EmitFormat::Brief,
        "context" => EmitFormat::Context,
        "dot" => EmitFormat::Dot,
        _ => EmitFormat::Json,
    }
}

fn build_schema(ctx: &pipeline::CompilationContext) -> specforge_emitter::GraphProtocolSchema {
    generate_schema(
        &ctx.kind_registry,
        &ctx.edge_registry,
        &ctx.field_registry,
        &ctx.extension_info,
    )
}

pub fn run(
    path: &Path,
    format: &str,
    scope: Option<&str>,
    no_schema: bool,
    schema_version: Option<&str>,
    max_tokens: Option<usize>,
) -> i32 {
    let ctx = pipeline::compile(path);
    let fmt = parse_format(format);

    if no_schema || fmt == EmitFormat::Dot {
        let options = EmitOptions {
            format: fmt,
            scope,
            schema: None,
            token_budget: max_tokens,
            kind_registry: Some(&ctx.kind_registry),
            ..Default::default()
        };
        match emit(&ctx.graph, &options) {
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
        let mut schema = build_schema(&ctx);

        if let Some(ver_str) = schema_version {
            match ver_str.parse::<SchemaVersion>() {
                Ok(requested) => {
                    // Real negotiation: same major as the produced schema,
                    // minor/patch from 0 up to the produced version.
                    let max = schema.schema_version.clone();
                    let min = specforge_emitter::SchemaVersion::new(max.major, 0, 0);
                    if let Err(e) = specforge_emitter::negotiate_version(&requested, &min, &max) {
                        eprintln!("{}", e);
                        return 1;
                    }
                    schema.schema_version = requested;
                }
                Err(e) => {
                    eprintln!("invalid --schema-version: {}", e);
                    return 1;
                }
            }
        }

        let options = EmitOptions {
            format: fmt,
            scope,
            schema: Some(&schema),
            token_budget: max_tokens,
            ..Default::default()
        };
        match emit(&ctx.graph, &options) {
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
}

pub fn run_schema(path: &Path, kind: Option<&str>, publish: bool, format: Option<&str>) -> i32 {
    let ctx = pipeline::compile(path);

    let schema = build_schema(&ctx);

    if publish {
        let emit_format = match format.unwrap_or("graph") {
            "context" => specforge_emitter::EmitFormat::Context,
            "brief" => specforge_emitter::EmitFormat::Brief,
            _ => specforge_emitter::EmitFormat::Json,
        };
        let output = specforge_emitter::publish_json_schema_format(&schema, emit_format);
        println!("{}", output);
        return 0;
    }

    if let Some(kind_name) = kind {
        match specforge_emitter::emit_schema_for_kind(&schema, kind_name) {
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
        let output = specforge_emitter::emit_schema(&schema);
        println!("{}", output);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    }
