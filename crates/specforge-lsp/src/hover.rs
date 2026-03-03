use specforge_common::FieldValue;
use tower_lsp::lsp_types::*;

use crate::backend::Backend;
use crate::position;
use crate::util;

pub fn hover(backend: &Backend, params: HoverParams) -> Option<Hover> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let file_path = position::uri_to_file_path(
        &params.text_document_position_params.text_document.uri,
    )?;
    let pos = params.text_document_position_params.position;

    let source = state.sources.get(&file_path)?;
    let entity_id = util::entity_at_position(source, pos.line, pos.character)?;

    let node = state.graph.get_node(&entity_id)?;

    // Build hover content
    let title = node
        .title
        .as_deref()
        .unwrap_or_else(|| node.id.raw());

    let mut content = format!("**{}** `{}` — \"{}\"\n", node.kind, node.id.raw(), title);

    // Look for contract/guarantee text in the AST
    if let Some(description) = find_entity_description(&state.files, &entity_id) {
        content.push_str(&format!("\n> {}\n", description));
    }

    // Count incoming references
    let incoming = state.graph.incoming_edges(node.id.raw());
    let ref_count = incoming.len();
    content.push_str(&format!(
        "\nReferenced by: {} {} | File: {}",
        ref_count,
        if ref_count == 1 { "entity" } else { "entities" },
        node.file,
    ));

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: Some(position::span_to_range(&node.span)),
    })
}

/// Extract the primary description (contract/guarantee) from an entity's AST fields.
fn find_entity_description(
    files: &[specforge_parser::SpecFile],
    entity_id: &str,
) -> Option<String> {
    for file in files {
        for entity in &file.entities {
            if entity.id.raw() != entity_id {
                continue;
            }
            // Look for contract, guarantee, problem, definition fields
            for (key, value) in &entity.fields.entries {
                match key.as_str() {
                    "contract" | "guarantee" | "problem" | "definition" => {
                        if let FieldValue::String(s) = value {
                            return Some(s.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}
