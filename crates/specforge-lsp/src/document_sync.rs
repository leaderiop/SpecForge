use std::collections::HashMap;

use tower_lsp::lsp_types::*;

use crate::backend::Backend;
use crate::diagnostics;
use crate::position;
use crate::state::ServerState;

pub async fn did_open(backend: &Backend, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri;
    let text = params.text_document.text;

    if let Some(file_path) = position::uri_to_file_path(&uri) {
        {
            let mut state_lock = backend.state.lock().unwrap();
            if let Some(state) = state_lock.as_mut() {
                state.update_source(&file_path, text);
                state.incremental_rebuild(&[file_path]);
            }
        }

        publish_diagnostics_from_state(backend).await;
    }
}

pub async fn did_change(backend: &Backend, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;

    if let Some(file_path) = position::uri_to_file_path(&uri) {
        {
            let mut state_lock = backend.state.lock().unwrap();
            if let Some(state) = state_lock.as_mut() {
                // Apply incremental changes to the in-memory source
                if let Some(source) = state.sources.get_mut(&file_path) {
                    for change in &params.content_changes {
                        apply_change(source, change);
                    }
                    // Clone the updated source for rebuild
                    let updated = source.clone();
                    state.update_source(&file_path, updated);
                } else {
                    // File not yet tracked — use last change as full content
                    if let Some(change) = params.content_changes.last() {
                        state.update_source(&file_path, change.text.clone());
                    }
                }
                state.incremental_rebuild(&[file_path]);
            }
        }

        publish_diagnostics_from_state(backend).await;
    }
}

/// Apply a single incremental text change to a source string.
fn apply_change(source: &mut String, change: &TextDocumentContentChangeEvent) {
    if let Some(range) = change.range {
        let start = line_col_to_offset(source, range.start.line, range.start.character);
        let end = line_col_to_offset(source, range.end.line, range.end.character);
        source.replace_range(start..end, &change.text);
    } else {
        // No range means full content replacement (fallback)
        *source = change.text.clone();
    }
}

/// Convert a 0-indexed (line, col) pair to a byte offset in source.
fn line_col_to_offset(source: &str, line: u32, col: u32) -> usize {
    let mut offset = 0;
    for (i, l) in source.lines().enumerate() {
        if i == line as usize {
            return offset + (col as usize).min(l.len());
        }
        offset += l.len() + 1; // +1 for '\n'
    }
    // If line is past end, return end of source
    source.len()
}

pub async fn did_close(_backend: &Backend, _params: DidCloseTextDocumentParams) {
    // No special handling needed — state persists
}

/// Collect diagnostics from state (holding lock briefly), then publish asynchronously.
pub async fn publish_diagnostics_from_state(backend: &Backend) {
    let diags_by_file = {
        let state_lock = backend.state.lock().unwrap();
        match state_lock.as_ref() {
            Some(state) => collect_diagnostics(state),
            None => return,
        }
    };
    // Lock is dropped here — safe to .await

    for (uri, file_diags) in diags_by_file {
        backend
            .client
            .publish_diagnostics(uri, file_diags, None)
            .await;
    }
}

/// Collect all diagnostics grouped by file URI. Pure function, no async.
fn collect_diagnostics(state: &ServerState) -> Vec<(Url, Vec<tower_lsp::lsp_types::Diagnostic>)> {
    let mut diags_by_file: HashMap<String, Vec<tower_lsp::lsp_types::Diagnostic>> = HashMap::new();

    // Ensure all known source files get an entry (even if empty, to clear old diagnostics)
    for file_path in state.sources.keys() {
        diags_by_file.entry(file_path.clone()).or_default();
    }

    for diag in &state.diagnostics {
        let lsp_diag = diagnostics::to_lsp_diagnostic(diag);
        diags_by_file
            .entry(diag.span.file.clone())
            .or_default()
            .push(lsp_diag);
    }

    diags_by_file
        .into_iter()
        .filter_map(|(file_path, file_diags)| {
            let uri = position::file_path_to_uri(&file_path)?;
            Some((uri, file_diags))
        })
        .collect()
}
