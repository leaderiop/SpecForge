use specforge_common::EntityKind;
use tower_lsp::lsp_types::*;

use crate::backend::Backend;
use crate::position;

/// Map EntityKind to an LSP SymbolKind.
pub fn entity_kind_to_symbol_kind(kind: EntityKind) -> SymbolKind {
    match kind {
        EntityKind::Behavior => SymbolKind::FUNCTION,
        EntityKind::Invariant => SymbolKind::PROPERTY,
        EntityKind::Feature => SymbolKind::MODULE,
        EntityKind::TypeDef => SymbolKind::STRUCT,
        EntityKind::Port => SymbolKind::INTERFACE,
        EntityKind::Event => SymbolKind::EVENT,
        EntityKind::Spec => SymbolKind::NAMESPACE,
        EntityKind::Ref => SymbolKind::FILE,
        EntityKind::Capability => SymbolKind::METHOD,
        EntityKind::Deliverable => SymbolKind::PACKAGE,
        EntityKind::Roadmap => SymbolKind::ENUM,
        EntityKind::Library => SymbolKind::OBJECT,
        EntityKind::Glossary => SymbolKind::KEY,
        EntityKind::Decision => SymbolKind::CONSTANT,
        EntityKind::Constraint => SymbolKind::TYPE_PARAMETER,
        EntityKind::FailureMode => SymbolKind::NULL,
    }
}

pub fn document_symbol(
    backend: &Backend,
    params: DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let file_path = position::uri_to_file_path(&params.text_document.uri)?;

    let entity_ids = state.file_index.entities_in(&file_path);
    if entity_ids.is_empty() {
        return Some(DocumentSymbolResponse::Nested(Vec::new()));
    }

    let mut symbols = Vec::new();
    for entity_id in entity_ids {
        if let Some(node) = state.graph.get_node(entity_id) {
            let name = node.title.clone().unwrap_or_else(|| node.id.raw().to_string());
            let detail = Some(format!("{}", node.kind));
            let kind = entity_kind_to_symbol_kind(node.kind);
            let range = position::span_to_range(&node.span);

            #[allow(deprecated)]
            symbols.push(DocumentSymbol {
                name,
                detail,
                kind,
                tags: None,
                deprecated: None,
                range,
                selection_range: range,
                children: None,
            });
        }
    }

    Some(DocumentSymbolResponse::Nested(symbols))
}
