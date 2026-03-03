use specforge_common::{EdgeType, EntityKind, FieldValue, ValidationCode};
use tower_lsp::lsp_types::*;

use crate::backend::Backend;
use crate::position;
use crate::state::ServerState;

pub fn code_actions(
    backend: &Backend,
    params: CodeActionParams,
) -> Option<CodeActionResponse> {
    let state_lock = backend.state.lock().unwrap();
    let state = state_lock.as_ref()?;

    let file_path = position::uri_to_file_path(&params.text_document.uri)?;

    let mut actions = Vec::new();

    // 1. Generate test stub for testable entities missing `tests` field
    generate_test_stub_actions(state, &params, &file_path, &mut actions);

    // 2. Create entity stub for E001 unresolved references
    create_entity_stub_actions(state, &params, &file_path, &mut actions);

    // 3. Add missing import for references to entities in unimported files
    add_missing_import_actions(state, &params, &file_path, &mut actions);

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

/// Offer "Generate test stub" for testable entities that have verify/scenario but no `tests` field.
fn generate_test_stub_actions(
    state: &ServerState,
    params: &CodeActionParams,
    file_path: &str,
    actions: &mut Vec<CodeActionOrCommand>,
) {
    let entity_ids = state.file_index.entities_in(file_path);
    for entity_id in entity_ids {
        let node = match state.graph.get_node(entity_id) {
            Some(n) => n,
            None => continue,
        };

        if !node.kind.is_testable() {
            continue;
        }

        let entity_range = position::span_to_range(&node.span);
        if !ranges_overlap(&params.range, &entity_range) {
            continue;
        }

        let (has_verify_or_scenario, has_tests_field) =
            check_entity_fields(&state.files, entity_id);

        if has_verify_or_scenario && !has_tests_field {
            let test_path = generate_test_path(entity_id, &node.kind);

            let insert_line = entity_range.end.line;
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line: insert_line,
                        character: 0,
                    },
                    end: Position {
                        line: insert_line,
                        character: 0,
                    },
                },
                new_text: format!("  tests [\"{test_path}\"]\n"),
            };

            let mut changes = std::collections::HashMap::new();
            changes.insert(params.text_document.uri.clone(), vec![edit]);

            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Generate test stub for `{entity_id}`"),
                kind: Some(CodeActionKind::QUICKFIX),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }),
                ..Default::default()
            }));
        }
    }
}

/// Offer "Create entity stub" for E001 unresolved references in cursor range.
fn create_entity_stub_actions(
    state: &ServerState,
    params: &CodeActionParams,
    file_path: &str,
    actions: &mut Vec<CodeActionOrCommand>,
) {
    let source = match state.sources.get(file_path) {
        Some(s) => s,
        None => return,
    };

    for diag in &state.diagnostics {
        if diag.code != ValidationCode::E001 {
            continue;
        }
        if diag.span.file != file_path {
            continue;
        }

        let diag_range = position::span_to_range(&diag.span);
        if !ranges_overlap(&params.range, &diag_range) {
            continue;
        }

        // Extract the entity ID from the diagnostic span
        let entity_id = extract_text_from_span(source, &diag.span);
        if entity_id.is_empty() {
            continue;
        }

        // Determine expected kind from the field context
        let kind = determine_kind_from_context(source, diag.span.start_line.saturating_sub(1))
            .unwrap_or(EntityKind::Behavior);

        let keyword = kind.keyword();
        let title = id_to_title(&entity_id);
        let primary_field = primary_field_for_kind(kind);
        let line_count = source.lines().count();

        let stub = format!(
            "\n{keyword} {entity_id} \"{title}\" {{\n  {primary_field} \"\"\"\n    TODO\n  \"\"\"\n}}\n"
        );

        let insert_pos = Position {
            line: line_count as u32,
            character: 0,
        };
        let edit = TextEdit {
            range: Range {
                start: insert_pos,
                end: insert_pos,
            },
            new_text: stub,
        };

        let mut changes = std::collections::HashMap::new();
        changes.insert(params.text_document.uri.clone(), vec![edit]);

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Create `{keyword} {entity_id}` stub"),
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            ..Default::default()
        }));
    }
}

/// Offer "Add use import" when the current file references entities declared in other files
/// but doesn't have a corresponding `use` import for that file.
fn add_missing_import_actions(
    state: &ServerState,
    params: &CodeActionParams,
    file_path: &str,
    actions: &mut Vec<CodeActionOrCommand>,
) {
    let source = match state.sources.get(file_path) {
        Some(s) => s,
        None => return,
    };

    // Find the current file's SpecFile to get its imports and entities
    let spec_file = state.files.iter().find(|f| f.path == file_path);
    let spec_file = match spec_file {
        Some(f) => f,
        None => return,
    };

    let existing_imports: std::collections::HashSet<&str> = spec_file
        .imports
        .iter()
        .map(|i| i.path.as_str())
        .collect();

    // Collect all referenced entity IDs from entities in this file
    let mut missing_imports: std::collections::HashMap<String, String> =
        std::collections::HashMap::new(); // import_path -> first referenced entity_id

    for entity in &spec_file.entities {
        for (field_name, value) in &entity.fields.entries {
            // Only check reference fields (those that have an EdgeType mapping)
            if EdgeType::from_field_name(field_name).is_none() {
                continue;
            }

            let ref_ids: Vec<&str> = match value {
                FieldValue::Reference(id) => vec![id.raw()],
                FieldValue::ReferenceList(ids) => ids.iter().map(|id| id.raw()).collect(),
                _ => continue,
            };

            for ref_id in ref_ids {
                // Look up where this entity is declared
                let decl = match state.symbols.get(ref_id) {
                    Some(d) => d,
                    None => continue, // E001 — doesn't exist at all
                };

                if decl.file == file_path {
                    continue; // Same file, no import needed
                }

                // Compute import path relative to spec_root
                let spec_root_str = state.spec_root.to_string_lossy();
                let rel_path = decl
                    .file
                    .strip_prefix(spec_root_str.as_ref())
                    .unwrap_or(&decl.file)
                    .trim_start_matches('/')
                    .trim_end_matches(".spec");

                if existing_imports.contains(rel_path) {
                    continue;
                }
                // Also check with .spec extension
                let rel_with_ext = format!("{}.spec", rel_path);
                if existing_imports.contains(rel_with_ext.as_str()) {
                    continue;
                }

                missing_imports
                    .entry(rel_path.to_string())
                    .or_insert_with(|| ref_id.to_string());
            }
        }
    }

    // Only offer import actions if the cursor overlaps something relevant
    // (we offer for the whole file since imports are file-level)
    for (import_path, _example_entity) in &missing_imports {
        // Find insertion point: after last `use` line, or at beginning of file
        let insert_line = find_import_insertion_line(source);

        let edit = TextEdit {
            range: Range {
                start: Position {
                    line: insert_line,
                    character: 0,
                },
                end: Position {
                    line: insert_line,
                    character: 0,
                },
            },
            new_text: format!("use {import_path}\n"),
        };

        let mut changes = std::collections::HashMap::new();
        changes.insert(params.text_document.uri.clone(), vec![edit]);

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Add `use {import_path}`"),
            kind: Some(CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            ..Default::default()
        }));
    }
}

/// Find the line number where a new `use` import should be inserted.
/// Returns the line after the last existing `use` line, or line 0 if none.
fn find_import_insertion_line(source: &str) -> u32 {
    let mut last_use_line: Option<u32> = None;
    for (i, line) in source.lines().enumerate() {
        if line.trim_start().starts_with("use ") {
            last_use_line = Some(i as u32);
        }
    }
    match last_use_line {
        Some(line) => line + 1,
        None => 0,
    }
}

/// Extract text from source at the given span range.
fn extract_text_from_span(source: &str, span: &specforge_common::SourceSpan) -> String {
    let lines: Vec<&str> = source.lines().collect();
    // SourceSpan is 1-indexed
    let line_idx = span.start_line.saturating_sub(1) as usize;
    let start_col = span.start_col.saturating_sub(1) as usize;
    let end_col = span.end_col.saturating_sub(1) as usize;

    if let Some(line) = lines.get(line_idx) {
        let start = start_col.min(line.len());
        let end = end_col.min(line.len());
        if start <= end {
            return line[start..end].to_string();
        }
    }
    String::new()
}

/// Determine the expected entity kind from the field context at a given 0-indexed line.
fn determine_kind_from_context(source: &str, line: u32) -> Option<EntityKind> {
    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines.get(line as usize)?;
    let trimmed = current_line.trim();

    // Look for field name before `[`
    let field_name = if let Some(bracket_pos) = trimmed.find('[') {
        Some(trimmed[..bracket_pos].trim())
    } else {
        None
    };

    let field_name = field_name?;
    let edge_type = EdgeType::from_field_name(field_name)?;
    edge_type.target_kind()
}

/// Convert an entity_id like `validate_input` to a title like "Validate Input".
fn id_to_title(id: &str) -> String {
    id.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Return the primary text field for a given entity kind.
fn primary_field_for_kind(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Behavior => "contract",
        EntityKind::Invariant => "guarantee",
        EntityKind::Feature => "problem",
        EntityKind::Event => "trigger",
        EntityKind::Constraint => "metric",
        EntityKind::Capability => "persona",
        EntityKind::Decision => "context",
        EntityKind::FailureMode => "severity",
        _ => "contract",
    }
}

fn ranges_overlap(a: &Range, b: &Range) -> bool {
    a.start.line <= b.end.line && a.end.line >= b.start.line
}

fn check_entity_fields(
    files: &[specforge_parser::SpecFile],
    entity_id: &str,
) -> (bool, bool) {
    let mut has_verify_or_scenario = false;
    let mut has_tests_field = false;

    for file in files {
        for entity in &file.entities {
            if entity.id.raw() != entity_id {
                continue;
            }
            for (key, value) in &entity.fields.entries {
                match key.as_str() {
                    "tests" => has_tests_field = true,
                    _ => {
                        if let FieldValue::VerifyList(v) = value {
                            if !v.is_empty() {
                                has_verify_or_scenario = true;
                            }
                        }
                        if let FieldValue::ScenarioList(s) = value {
                            if !s.is_empty() {
                                has_verify_or_scenario = true;
                            }
                        }
                    }
                }
            }
        }
    }

    (has_verify_or_scenario, has_tests_field)
}

fn generate_test_path(entity_id: &str, kind: &specforge_common::EntityKind) -> String {
    let dir = match kind {
        specforge_common::EntityKind::Behavior => "behaviors",
        specforge_common::EntityKind::Invariant => "invariants",
        specforge_common::EntityKind::Event => "events",
        specforge_common::EntityKind::Constraint => "constraints",
        specforge_common::EntityKind::Capability => "capabilities",
        _ => "tests",
    };
    format!("tests/{dir}/{entity_id}_test.rs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_to_title_basic() {
        assert_eq!(id_to_title("validate_input"), "Validate Input");
        assert_eq!(id_to_title("data_persistence"), "Data Persistence");
        assert_eq!(id_to_title("auth"), "Auth");
    }

    #[test]
    fn find_import_insertion_line_with_imports() {
        let source = "use common/types\nuse behaviors/auth\n\nbehavior foo \"Foo\" {\n}";
        assert_eq!(find_import_insertion_line(source), 2);
    }

    #[test]
    fn find_import_insertion_line_no_imports() {
        let source = "behavior foo \"Foo\" {\n}";
        assert_eq!(find_import_insertion_line(source), 0);
    }

    #[test]
    fn primary_field_coverage() {
        assert_eq!(primary_field_for_kind(EntityKind::Behavior), "contract");
        assert_eq!(primary_field_for_kind(EntityKind::Invariant), "guarantee");
        assert_eq!(primary_field_for_kind(EntityKind::Feature), "problem");
    }
}
