use specforge_graph::Graph;

/// A code action to be offered in the editor.
#[derive(Debug, Clone)]
pub struct CodeAction {
    pub entity_id: String,
    pub file: String,
    pub action_kind: String,
    pub title: String,
    pub edit_text: String,
    pub insert_line: usize,
}

/// Generate "add missing verify" code actions for testable entities in a file
/// that have no verify statements.
pub fn code_actions_missing_verify(
    graph: &Graph,
    file: &str,
    testable_kinds: &[&str],
) -> Vec<CodeAction> {
    graph
        .nodes_in_file(file)
        .into_iter()
        .filter(|n| testable_kinds.contains(&n.kind.raw.as_str()))
        .filter(|n| {
            // Check if entity has any verify fields already
            n.fields.get("verify").is_none()
        })
        .map(|n| {
            let stub = format!(
                "  verify unit \"{} — TODO\"",
                n.id.raw
            );
            CodeAction {
                entity_id: n.id.raw.to_string(),
                file: n.source_span.file.to_string(),
                action_kind: "quickfix".into(),
                title: format!("Add verify stub for {}", n.id.raw),
                edit_text: stub,
                insert_line: n.source_span.end_line,
            }
        })
        .collect()
}

/// Generate a code action to add a missing import for an entity that exists
/// in another file.
pub fn code_action_add_import(
    graph: &Graph,
    entity_id: &str,
    current_file: &str,
    spec_root: &str,
) -> Option<CodeAction> {
    let node = graph.node(entity_id)?;
    let source_file = node.source_span.file.as_str();

    // Don't offer import if the entity is in the same file
    if source_file == current_file {
        return None;
    }

    // Convert file path to use-import path: strip spec_root prefix and .spec suffix
    let import_path = source_file
        .strip_prefix(spec_root)
        .unwrap_or(source_file)
        .strip_prefix('/')
        .unwrap_or(source_file)
        .strip_suffix(".spec")
        .unwrap_or(source_file);

    Some(CodeAction {
        entity_id: entity_id.to_string(),
        file: current_file.to_string(),
        action_kind: "quickfix".into(),
        title: format!("Add import for {entity_id}"),
        edit_text: format!("use \"{import_path}\""),
        insert_line: 0,
    })
}

/// Generate a code action to create an entity stub for an unresolved reference.
/// Returns None if `target_kind` is None (can't infer kind without field metadata).
pub fn code_action_create_stub(
    entity_id: &str,
    target_kind: Option<&str>,
    current_file: &str,
) -> Option<CodeAction> {
    let kind = target_kind?;

    let stub = format!(
        "{kind} {entity_id} \"{entity_id}\" {{\n  // TODO: fill in fields\n}}"
    );

    Some(CodeAction {
        entity_id: entity_id.to_string(),
        file: current_file.to_string(),
        action_kind: "refactor".into(),
        title: format!("Create {kind} stub for {entity_id}"),
        edit_text: stub,
        insert_line: usize::MAX, // append to end of file
    })
}
