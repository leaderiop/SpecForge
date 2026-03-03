use tower_lsp::lsp_types::*;

use crate::backend::Backend;
use crate::position;
use crate::util;

pub fn goto_definition(
    backend: &Backend,
    params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let file_path = position::uri_to_file_path(
        &params.text_document_position_params.text_document.uri,
    )?;
    let pos = params.text_document_position_params.position;

    let source = state.sources.get(&file_path)?;

    // Check if cursor is on a `use` import line
    if let Some(response) = resolve_import(source, pos.line, state) {
        return Some(response);
    }

    // Fall through to entity ID lookup
    let entity_id = util::entity_at_position(source, pos.line, pos.character)?;
    let decl = state.symbols.get(&entity_id)?;
    let location = position::span_to_location(&decl.span)?;

    Some(GotoDefinitionResponse::Scalar(location))
}

/// If the cursor is on a `use` import line, resolve the import path to a file location.
fn resolve_import(
    source: &str,
    line: u32,
    state: &crate::state::ServerState,
) -> Option<GotoDefinitionResponse> {
    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines.get(line as usize)?;
    let trimmed = current_line.trim();

    if !trimmed.starts_with("use ") {
        return None;
    }

    // Extract the import path: everything after "use " until whitespace or end of line
    let import_part = trimmed.strip_prefix("use ")?.trim();
    // Strip any trailing selective import like `{ foo, bar }`
    let import_path = import_part
        .split_whitespace()
        .next()?
        .trim_end_matches('{');

    // Resolve to file: spec_root / import_path
    // The import path may or may not have .spec extension
    let spec_root = state.spec_root.to_string_lossy();
    let resolved = if import_path.ends_with(".spec") {
        format!("{}/{}", spec_root, import_path)
    } else {
        format!("{}/{}.spec", spec_root, import_path)
    };

    // Check if the file exists in our sources
    if !state.sources.contains_key(&resolved) {
        // Try as-is path too
        if !std::path::Path::new(&resolved).exists() {
            return None;
        }
    }

    let uri = position::file_path_to_uri(&resolved)?;
    let location = Location {
        uri,
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
    };

    Some(GotoDefinitionResponse::Scalar(location))
}
