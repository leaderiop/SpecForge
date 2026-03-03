use tower_lsp::lsp_types::*;

use crate::backend::Backend;
use crate::position;
use crate::util;

pub fn prepare_rename(
    backend: &Backend,
    params: TextDocumentPositionParams,
) -> Option<PrepareRenameResponse> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let file_path = position::uri_to_file_path(&params.text_document.uri)?;
    let pos = params.position;

    let source = state.sources.get(&file_path)?;
    let entity_id = util::entity_at_position(source, pos.line, pos.character)?;

    // Verify it's a declared entity
    let decl = state.symbols.get(&entity_id)?;

    let range = position::span_to_range(&decl.span);
    Some(PrepareRenameResponse::Range(range))
}

pub fn rename(backend: &Backend, params: RenameParams) -> Option<WorkspaceEdit> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let file_path = position::uri_to_file_path(
        &params.text_document_position.text_document.uri,
    )?;
    let pos = params.text_document_position.position;

    let source = state.sources.get(&file_path)?;
    let entity_id = util::entity_at_position(source, pos.line, pos.character)?;

    // Verify it's a declared entity
    if !state.symbols.contains(&entity_id) {
        return None;
    }

    let new_name = &params.new_name;

    // Check new name uniqueness
    if state.symbols.contains(new_name) {
        return None;
    }

    let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
        std::collections::HashMap::new();

    // Find all occurrences across all files
    for (path, src) in &state.sources {
        let occurrences = util::find_identifier_occurrences(src, &entity_id);
        if occurrences.is_empty() {
            continue;
        }

        let uri = match position::file_path_to_uri(path) {
            Some(u) => u,
            None => continue,
        };

        let edits: Vec<TextEdit> = occurrences
            .into_iter()
            .map(|(line, start_col, end_col)| TextEdit {
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
                new_text: new_name.clone(),
            })
            .collect();

        changes.insert(uri, edits);
    }

    Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}
