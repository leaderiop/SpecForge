use tower_lsp::lsp_types::*;

use crate::backend::Backend;
use crate::position;
use crate::util;

pub fn references(backend: &Backend, params: ReferenceParams) -> Option<Vec<Location>> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let file_path =
        position::uri_to_file_path(&params.text_document_position.text_document.uri)?;
    let pos = params.text_document_position.position;

    let source = state.sources.get(&file_path)?;
    let entity_id = util::entity_at_position(source, pos.line, pos.character)?;

    // Verify it's a known entity
    if !state.symbols.contains(&entity_id) {
        return None;
    }

    let include_declaration = params.context.include_declaration;

    let mut locations = Vec::new();

    // Walk all source files to find identifier matches
    for (path, src) in &state.sources {
        let occurrences = util::find_identifier_occurrences(src, &entity_id);
        for (line, start_col, end_col) in occurrences {
            // Skip the declaration site unless requested
            if !include_declaration {
                if let Some(decl) = state.symbols.get(&entity_id) {
                    if decl.file == *path
                        && decl.span.start_line.saturating_sub(1) == line
                        && decl.span.start_col.saturating_sub(1) == start_col
                    {
                        continue;
                    }
                }
            }

            if let Some(uri) = position::file_path_to_uri(path) {
                locations.push(Location {
                    uri,
                    range: Range {
                        start: Position {
                            line,
                            character: start_col,
                        },
                        end: Position {
                            line,
                            character: end_col,
                        },
                    },
                });
            }
        }
    }

    Some(locations)
}
