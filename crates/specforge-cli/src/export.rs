use specforge_common::{Severity, find_project_root};
use specforge_emitter::{
    GraphProtocolSchema, detect_breaking_with_diagnostics, generate_schema, persist_schema_cache,
};
use specforge_ops::export;
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
    schema: export::Schema,
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
        schema,
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
    schema: export::Schema,
    schema_version: Option<&str>,
    max_tokens: Option<usize>,
) -> Result<String, i32> {
    let format = match format {
        ExportFormat::Graph => export::Format::Graph,
        ExportFormat::Brief => export::Format::Brief,
        ExportFormat::Context => export::Format::Context,
        ExportFormat::Dot => export::Format::Dot,
    };
    let project = export::Project {
        graph: &ctx.graph,
        kinds: &ctx.kind_registry,
        fields: &ctx.field_registry,
        schema: generated,
    };
    let request = export::Request {
        format: Some(format),
        scope,
        max_tokens,
        schema,
        schema_version,
        ..export::Request::default()
    };
    export::export(&project, &request).map_err(|err| {
        eprintln!("{}", err.message);
        1
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
