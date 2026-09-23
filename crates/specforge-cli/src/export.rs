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
    if max_tokens.is_some() && !budget_applies(fmt, no_schema) {
        eprintln!("token budget applies only to JSON export");
    }

    if no_schema || fmt == EmitFormat::Dot {
        let options = EmitOptions {
            format: fmt,
            scope,
            schema: None,
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
    } else {
        let mut schema = build_schema(&ctx);

        if let Some(ver_str) = schema_version {
            match ver_str.parse::<SchemaVersion>() {
                Ok(requested) => {
                    let min = &schema.schema_version;
                    let max = &schema.schema_version;
                    if let Err(e) = specforge_emitter::negotiate_version(&requested, min, max) {
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

pub fn run_schema(path: &Path, kind: Option<&str>, publish: bool) -> i32 {
    let ctx = pipeline::compile(path);

    let schema = build_schema(&ctx);

    if publish {
        let output = specforge_emitter::publish_json_schema(&schema);
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

/// The emitter honors `token_budget` only for JSON output without an embedded schema.
fn budget_applies(format: EmitFormat, no_schema: bool) -> bool {
    format == EmitFormat::Json && no_schema
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_applies_only_to_schemaless_json() {
        assert!(budget_applies(EmitFormat::Json, true));
        assert!(
            !budget_applies(EmitFormat::Json, false),
            "embedded-schema path ignores the budget"
        );
        assert!(!budget_applies(EmitFormat::Context, true));
        assert!(!budget_applies(EmitFormat::Brief, true));
        assert!(!budget_applies(EmitFormat::Dot, true));
    }
}
