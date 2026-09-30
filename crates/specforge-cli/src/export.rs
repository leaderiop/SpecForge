use specforge_common::{Severity, find_project_root};
use specforge_emitter::{
    EmitFormat, EmitOptions, GraphProtocolSchema, SchemaVersion, detect_breaking_with_diagnostics,
    emit, generate_schema, persist_schema_cache,
};
use std::path::{Path, PathBuf};

use crate::pipeline;
use crate::{ExportFormat, SchemaFormat};

pub(crate) fn build_schema(
    ctx: &pipeline::CompilationContext,
) -> specforge_emitter::GraphProtocolSchema {
    generate_schema(
        &ctx.kind_registry,
        &ctx.edge_registry,
        &ctx.field_registry,
        &ctx.extension_info,
    )
}

/// Where `specforge export` keeps the schema it last exported:
/// `<project>/.specforge/`, the project being the nearest ancestor with a
/// `specforge.json` (or `path` itself).
fn schema_cache_dir(path: &Path) -> PathBuf {
    find_project_root(path)
        .unwrap_or_else(|| path.to_path_buf())
        .join(".specforge")
}

/// Export the project to stdout. Before it writes, the schema the
/// extensions produce is compared with the one the previous export cached
/// in `.specforge/schema-cache.json`, and each breaking change is a W053
/// warning on stderr. After a successful export that schema replaces the
/// cache, so the next export compares against it. The export is written
/// whatever the comparison finds.
pub fn run(
    path: &Path,
    format: ExportFormat,
    scope: Option<&str>,
    no_schema: bool,
    schema_version: Option<&str>,
    max_tokens: Option<usize>,
) -> i32 {
    let ctx = pipeline::compile(path);
    let generated = build_schema(&ctx);
    let cache_dir = schema_cache_dir(path);

    // The export goes to stdout: there is no output directory holding
    // earlier exports, and `.specforge/` holds extensions and the watch
    // snapshot too, so nothing here shows the project was exported before.
    let (_migration, diagnostics) = detect_breaking_with_diagnostics(&cache_dir, &generated, false);
    for diagnostic in &diagnostics {
        eprintln!("{}", render_plain(diagnostic));
    }

    let output = match render_export(
        &ctx,
        &generated,
        format,
        scope,
        no_schema,
        schema_version,
        max_tokens,
    ) {
        Ok(output) => output,
        Err(code) => return code,
    };
    println!("{}", output);

    if let Err(e) = persist_schema_cache(&generated, &cache_dir) {
        eprintln!(
            "warning: could not write the schema cache in {}: {e}",
            cache_dir.display()
        );
    }
    0
}

/// A spanless diagnostic as `severity[CODE]: message`, with its suggestion
/// on a `= help:` line.
fn render_plain(diagnostic: &specforge_common::Diagnostic) -> String {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };
    let mut text = format!("{severity}[{}]: {}", diagnostic.code, diagnostic.message);
    if let Some(suggestion) = &diagnostic.suggestion {
        text.push_str(&format!("\n  = help: {suggestion}"));
    }
    text
}

/// The export text, or the exit code after the error is on stderr.
fn render_export(
    ctx: &pipeline::CompilationContext,
    generated: &GraphProtocolSchema,
    format: ExportFormat,
    scope: Option<&str>,
    no_schema: bool,
    schema_version: Option<&str>,
    max_tokens: Option<usize>,
) -> Result<String, i32> {
    let fmt = match format {
        ExportFormat::Graph => EmitFormat::Json,
        ExportFormat::Brief => EmitFormat::Brief,
        ExportFormat::Context => EmitFormat::Context,
        ExportFormat::Dot => EmitFormat::Dot,
    };

    let emitted = if no_schema || fmt == EmitFormat::Dot {
        let options = EmitOptions {
            format: fmt,
            scope,
            schema: None,
            token_budget: max_tokens,
            kind_registry: Some(&ctx.kind_registry),
            field_registry: Some(&ctx.field_registry),
            ..Default::default()
        };
        emit(&ctx.graph, &options)
    } else {
        let mut schema = generated.clone();

        if let Some(ver_str) = schema_version {
            match ver_str.parse::<SchemaVersion>() {
                Ok(requested) => {
                    // Real negotiation: same major as the produced schema,
                    // minor/patch from 0 up to the produced version.
                    let max = schema.schema_version.clone();
                    let min = specforge_emitter::SchemaVersion::new(max.major, 0, 0);
                    if let Err(e) = specforge_emitter::negotiate_version(&requested, &min, &max) {
                        eprintln!("{}", e);
                        return Err(1);
                    }
                    schema.schema_version = requested;
                }
                Err(e) => {
                    eprintln!("invalid --schema-version: {}", e);
                    return Err(1);
                }
            }
        }

        let options = EmitOptions {
            format: fmt,
            scope,
            schema: Some(&schema),
            token_budget: max_tokens,
            field_registry: Some(&ctx.field_registry),
            ..Default::default()
        };
        emit(&ctx.graph, &options)
    };

    emitted.map_err(|err| {
        eprintln!("{}", err);
        err.exit_code()
    })
}

pub fn run_schema(path: &Path, kind: Option<&str>, publish: bool, format: SchemaFormat) -> i32 {
    let ctx = pipeline::compile(path);

    let schema = build_schema(&ctx);

    if publish {
        let emit_format = match format {
            SchemaFormat::Context => specforge_emitter::EmitFormat::Context,
            SchemaFormat::Brief => specforge_emitter::EmitFormat::Brief,
            SchemaFormat::Graph => specforge_emitter::EmitFormat::Json,
        };
        let output = match specforge_emitter::publish_json_schema_format(&schema, emit_format) {
            Ok(out) => out,
            Err(err) => {
                eprintln!("{}", err);
                return 1;
            }
        };
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
        let output = match specforge_emitter::emit_schema(&schema) {
            Ok(out) => out,
            Err(err) => {
                eprintln!("{}", err);
                return 1;
            }
        };
        println!("{}", output);
        0
    }
}
