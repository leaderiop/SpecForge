use specforge_ops::export;
use specforge_ops::schema::SchemaRequest;
use specforge_ops::view::ProjectView;
use std::path::Path;

use crate::pipeline;

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
    format: export::Format,
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
        eprintln!("{}", specforge_common::render_plain(diagnostic));
    }

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
            // A diagnostic's code leads its message, as `specforge export`
            // has always said it (`E062: the token budget …`).
            if err.code.starts_with(|c: char| c.is_ascii_uppercase()) {
                eprintln!("{}: {}", err.code, err.message);
            } else {
                eprintln!("{}", err.message);
            }
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

/// An operation's failure as `error[CODE]: message`, with its suggestion
/// on a `= help:` line.
pub(crate) fn render_op_error(error: &specforge_ops::OpError) -> String {
    let mut diagnostic = specforge_common::Diagnostic::untyped(
        error.code.as_ref(),
        specforge_common::Severity::Error,
        &error.message,
    );
    if let Some(suggestion) = &error.suggestion {
        diagnostic = diagnostic.with_suggestion(suggestion);
    }
    specforge_common::render_plain(&diagnostic)
}

/// `specforge schema`: the schema operation over the project compiled at
/// `path`, versioned as the next export would be (the cache is only read):
/// the document `specforge.schema` returns for the same request, pretty
/// printed. With `publish`, the JSON Schema an export of that format
/// conforms to. An unknown kind is refused with the closest one (exit 1).
pub fn run_schema(path: &Path, request: &SchemaRequest, publish: Option<export::Format>) -> i32 {
    let (project, _runtime) = pipeline::compile_project(path);
    let view = ProjectView::of(&project);
    let output = match publish {
        Some(format) => specforge_ops::schema::json_schema(&view, format),
        None => specforge_ops::schema::schema(&view, request)
            .map(|outcome| serde_json::to_string_pretty(&outcome).expect("a schema serializes")),
    };
    match output {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(error) => {
            eprintln!("{}", error.message);
            if let Some(suggestion) = &error.suggestion {
                eprintln!("  = help: {suggestion}");
            }
            1
        }
    }
}
