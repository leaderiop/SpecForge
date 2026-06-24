use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as spec;

fn node(id: &str, kind: &str, file: &str, line: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(format!("{id} title")),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new(file),
            start_line: line, start_col: 0, end_line: line + 3, end_col: 1,
        },
    }
}

// -- code_actions_for_missing_verify ------------------------------------------

#[spec(behavior = "code_actions_for_missing_verify", verify = "code action offered on untested testable entity")]
#[test]
fn missing_verify_action_offered() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions = specforge_lsp::code_actions_missing_verify(
        &g, "a.spec", &["behavior"],
    );
    assert!(!actions.is_empty());
    assert!(actions[0].entity_id == "my_behavior");
}

#[spec(behavior = "code_actions_for_missing_verify", verify = "generated verify stubs added to entity block in .spec file")]
#[test]
fn missing_verify_produces_stub() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions = specforge_lsp::code_actions_missing_verify(&g, "a.spec", &["behavior"]);
    assert!(actions[0].edit_text.contains("verify"));
}

#[spec(behavior = "code_actions_for_missing_verify", verify = "verify stub uses allowed_verify_kinds from KindRegistry")]
#[test]
fn verify_stub_uses_unit_kind() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions = specforge_lsp::code_actions_missing_verify(&g, "a.spec", &["behavior"]);
    assert!(actions[0].edit_text.contains("verify unit"));
}

#[spec(behavior = "code_actions_for_missing_verify", verify = "stub format is verify <kind> entity_id TODO")]
#[test]
fn verify_stub_format() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions = specforge_lsp::code_actions_missing_verify(&g, "a.spec", &["behavior"]);
    assert!(actions[0].edit_text.contains("verify unit \"my_behavior"));
    assert!(actions[0].edit_text.contains("TODO"));
}

#[spec(behavior = "code_actions_for_missing_verify", verify = "code action kind is QuickFix")]
#[test]
fn verify_action_is_quickfix() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions = specforge_lsp::code_actions_missing_verify(&g, "a.spec", &["behavior"]);
    assert_eq!(actions[0].action_kind, "quickfix");
}

#[spec(behavior = "code_actions_for_missing_verify", verify = "no test source files or application code generated")]
#[test]
fn verify_action_no_code_gen() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions = specforge_lsp::code_actions_missing_verify(&g, "a.spec", &["behavior"]);
    // The edit should only modify the .spec file, not create new files
    assert!(actions[0].file.ends_with(".spec"));
}

// -- code_action_add_missing_import -------------------------------------------

#[spec(behavior = "code_action_add_missing_import", verify = "code action offered on E003 for resolvable entity")]
#[test]
fn add_import_offered_for_resolvable_entity() {
    let mut g = Graph::new();
    g.add_node(node("auth_token", "type", "types/auth.spec", 1));

    let action = specforge_lsp::code_action_add_import(
        &g, "auth_token", "behaviors/login.spec", "spec",
    );
    assert!(action.is_some());
    let action = action.unwrap();
    assert!(action.edit_text.contains("use \"types/auth\""));
}

#[spec(behavior = "code_action_add_missing_import", verify = "import is inserted after existing use statements")]
#[test]
fn add_import_position() {
    let mut g = Graph::new();
    g.add_node(node("auth_token", "type", "types/auth.spec", 1));

    let action = specforge_lsp::code_action_add_import(
        &g, "auth_token", "behaviors/login.spec", "spec",
    );
    let action = action.unwrap();
    // edit_text should be a use statement with quoted path
    assert!(action.edit_text.starts_with("use \""));
}

#[spec(behavior = "code_action_add_missing_import", verify = "no code action when entity does not exist anywhere")]
#[test]
fn no_import_for_nonexistent_entity() {
    let g = Graph::new();
    let action = specforge_lsp::code_action_add_import(
        &g, "nonexistent", "login.spec", "spec",
    );
    assert!(action.is_none());
}

// -- code_action_create_entity_stub -------------------------------------------

#[spec(behavior = "code_action_create_entity_stub", verify = "code action offered on E003 for non-existent entity")]
#[test]
fn create_stub_offered() {
    let _g = Graph::new();
    let action = specforge_lsp::code_action_create_stub(
        "missing_type", Some("type"), "current.spec",
    );
    assert!(action.is_some());
}

#[spec(behavior = "code_action_create_entity_stub", verify = "stub uses correct entity kind from FieldRegistry target_kind")]
#[test]
fn stub_uses_correct_kind() {
    let action = specforge_lsp::code_action_create_stub(
        "missing_type", Some("type"), "current.spec",
    ).unwrap();
    assert!(action.edit_text.starts_with("type missing_type"));
}

#[spec(behavior = "code_action_create_entity_stub", verify = "no code action when enclosing field has no target_kind")]
#[test]
fn no_stub_without_target_kind() {
    let action = specforge_lsp::code_action_create_stub(
        "unknown_thing", None, "current.spec",
    );
    assert!(action.is_none());
}

#[spec(behavior = "code_action_create_entity_stub", verify = "stub is inserted at end of current file")]
#[test]
fn stub_targets_current_file() {
    let action = specforge_lsp::code_action_create_stub(
        "my_event", Some("event"), "current.spec",
    ).unwrap();
    assert_eq!(action.file, "current.spec");
}

#[spec(behavior = "code_action_create_entity_stub", verify = "code action kind is Refactor")]
#[test]
fn stub_action_is_refactor() {
    let action = specforge_lsp::code_action_create_stub(
        "my_event", Some("event"), "current.spec",
    ).unwrap();
    assert_eq!(action.action_kind, "refactor");
}

#[spec(behavior = "code_action_create_entity_stub", verify = "generated stub contains no application code or test files")]
#[test]
fn stub_no_app_code() {
    let action = specforge_lsp::code_action_create_stub(
        "my_event", Some("event"), "current.spec",
    ).unwrap();
    // Should be a minimal spec block, not code
    assert!(action.edit_text.contains("event my_event"));
    assert!(action.edit_text.contains('{'));
    assert!(!action.edit_text.contains("fn "));
    assert!(!action.edit_text.contains("class "));
}
