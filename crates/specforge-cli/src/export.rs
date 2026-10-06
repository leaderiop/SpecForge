use specforge_common::Severity;
use specforge_ops::export;
use specforge_ops::schema::SchemaRequest;
use specforge_ops::view::ProjectView;
use std::path::Path;

use crate::pipeline;
use crate::{ExportFormat, SchemaFormat};

/// Export the project compiled at `path` to stdout. Before it writes, the
/// schema the extensions produce is compared with the one the previous
/// export cached in `<path>/.specforge/schema-cache.json` (the root it
/// compiled, never an ancestor's): each breaking change is a W053 warning
/// on stderr, and the cached schema's version, bumped by what changed, is
/// the version the export carries. After a successful export that schema
/// replaces the cache, so the next export compares against it. The export
/// is written whatever the comparison finds.
pub fn run(
    path: &Path,
    format: ExportFormat,
    scope: Option<&str>,
    schema: export::Schema,
    schema_version: Option<&str>,
    max_tokens: Option<usize>,
) -> i32 {
    let (project, _runtime) = pipeline::compile_project(path);
    let view = ProjectView::of(&project);
    let cache = view.schema_cache().expect("a compiled project has a root");
    let generated = view.versioned_schema();

    // The export goes to stdout: there is no output directory holding
    // earlier exports, and `.specforge/` holds extensions and the watch
    // snapshot too, so nothing here shows the project was exported before.
    for diagnostic in &cache.breaking_changes(&generated) {
        eprintln!("{}", render_plain(diagnostic));
    }

    let format = match format {
        ExportFormat::Graph => export::Format::Graph,
        ExportFormat::Brief => export::Format::Brief,
        ExportFormat::Context => export::Format::Context,
        ExportFormat::Dot => export::Format::Dot,
    };
    let request = export::Request {
        format: Some(format),
        scope,
        max_tokens,
        schema,
        schema_version,
        ..export::Request::default()
    };
    let output = match export::export(&view, &request) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("{}", err.message);
            return 1;
        }
    };
    println!("{}", output);

    if let Err(e) = cache.record(&generated) {
        eprintln!(
            "warning: could not write the schema cache in {}: {e}",
            cache.dir().display()
        );
    }
    0
}

/// A spanless diagnostic as `severity[CODE]: message`, with its suggestion
/// on a `= help:` line.
pub(crate) fn render_plain(diagnostic: &specforge_common::Diagnostic) -> String {
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

/// An operation's failure as `error[CODE]: message`, with its suggestion
/// on a `= help:` line.
pub(crate) fn render_op_error(error: &specforge_ops::OpError) -> String {
    let mut diagnostic = specforge_common::Diagnostic::error(error.code.as_ref(), &error.message);
    if let Some(suggestion) = &error.suggestion {
        diagnostic = diagnostic.with_suggestion(suggestion);
    }
    render_plain(&diagnostic)
}

/// `specforge schema`: the schema operation over the project compiled at
/// `path`, versioned as the next export would be (the cache is only
/// read). `--kind` prints that kind's entry; an unknown kind is refused
/// with the closest one (exit 1).
pub fn run_schema(path: &Path, kind: Option<&str>, publish: bool, format: SchemaFormat) -> i32 {
    let (project, _runtime) = pipeline::compile_project(path);
    let view = ProjectView::of(&project);

    if publish {
        let emit_format = match format {
            SchemaFormat::Context => specforge_emitter::EmitFormat::Context,
            SchemaFormat::Brief => specforge_emitter::EmitFormat::Brief,
            SchemaFormat::Graph => specforge_emitter::EmitFormat::Json,
        };
        let output = match specforge_emitter::publish_json_schema_format(
            &view.versioned_schema(),
            emit_format,
        ) {
            Ok(out) => out,
            Err(err) => {
                eprintln!("{}", err);
                return 1;
            }
        };
        println!("{}", output);
        return 0;
    }

    let request = SchemaRequest {
        kind,
        ..SchemaRequest::default()
    };
    let outcome = match specforge_ops::schema::schema(&view, &request) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("{}", err.message);
            if let Some(suggestion) = &err.suggestion {
                eprintln!("  = help: {suggestion}");
            }
            return 1;
        }
    };
    // `--kind` shows the kind's entry alone.
    let output = match kind {
        Some(kind) => specforge_emitter::emit_schema_for_kind(&outcome.schema, kind),
        None => specforge_emitter::emit_schema(&outcome.schema),
    };
    match output {
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
