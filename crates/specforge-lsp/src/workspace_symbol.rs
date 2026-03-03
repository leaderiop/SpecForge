use tower_lsp::lsp_types::*;

use crate::backend::Backend;
use crate::document_symbol::entity_kind_to_symbol_kind;
use crate::position;

pub fn workspace_symbol(
    backend: &Backend,
    params: WorkspaceSymbolParams,
) -> Option<Vec<SymbolInformation>> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let query = params.query.to_lowercase();
    let mut results = Vec::new();

    for node in state.graph.nodes() {
        let raw_id = node.id.raw().to_string();
        let title = node.title.clone().unwrap_or_default();

        // Match by prefix or substring on ID and title
        let matches = if query.is_empty() {
            true
        } else {
            raw_id.to_lowercase().contains(&query)
                || title.to_lowercase().contains(&query)
        };

        if !matches {
            continue;
        }

        let location = match position::span_to_location(&node.span) {
            Some(loc) => loc,
            None => continue,
        };

        #[allow(deprecated)]
        results.push(SymbolInformation {
            name: raw_id,
            kind: entity_kind_to_symbol_kind(node.kind),
            tags: None,
            deprecated: None,
            location,
            container_name: Some(node.kind.keyword().to_string()),
        });
    }

    Some(results)
}
