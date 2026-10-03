use specforge_common::{Diagnostic, DiagnosticData, SourceSpan, Sym};
use specforge_graph::{Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as spec;

fn node(id: &str, kind: &str, file: &str, line: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: Some(format!("{id} title")),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new(file),
            start_line: line,
            start_col: 0,
            end_line: line + 3,
            end_col: 1,
        },
        methods: Vec::new(),
    }
}

/// A registry where `kinds` accept verify statements of `verify_kinds`.
fn verifiable(kinds: &[&str], verify_kinds: &[&str]) -> specforge_registry::KindRegistry {
    let mut registry = specforge_registry::KindRegistry::new();
    for kind in kinds {
        registry.register(specforge_registry::KindRegistryEntry {
            kind_name: kind.to_string(),
            description: None,
            source_extension: "@test/ext".into(),
            testable: true,
            singleton: false,
            supports_verify: true,
            allowed_verify_kinds: verify_kinds.iter().map(|k| k.to_string()).collect(),
            has_body_parser: false,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            open_fields: false,
            contract_target: false,
            declares_types: false,
        });
    }
    registry
}

// -- code_actions_for_missing_verify ------------------------------------------

#[spec(
    behavior = "code_actions_for_missing_verify",
    verify = "code action offered on untested testable entity"
)]
fn missing_verify_action_offered() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions =
        specforge_lsp::code_actions_missing_verify(&g, "a.spec", &verifiable(&["behavior"], &[]));
    assert!(!actions.is_empty());
    assert!(actions[0].entity_id == "my_behavior");
}

#[spec(
    behavior = "code_actions_for_missing_verify",
    verify = "generated verify stubs added to entity block in .spec file"
)]
#[tokio::test]
async fn missing_verify_produces_stub() {
    // `first` has no verify statement; `second` follows it in the file.
    let text = "behavior first \"First\" {\n  contract \"c\"\n}\n\n\
                behavior second \"Second\" {\n  contract \"c\"\n  verify unit \"s\"\n}\n";
    let (mut client, uri, _dir) = crate::e2e::start_server_with_extensions(
        &["@specforge/software", "@specforge/testing"],
        "flows.spec",
        text,
    )
    .await;

    let resp = client.code_action(&uri, 0, 0, 8, 0).await;
    let actions = resp["result"].as_array().cloned().unwrap_or_default();
    let stub = actions
        .iter()
        .find(|a| a["title"] == "Add verify stub for first")
        .unwrap_or_else(|| panic!("no verify stub action in {resp}"));
    // The edit targets the .spec file itself, and nothing else.
    let changes = stub["edit"]["changes"].as_object().unwrap();
    assert_eq!(changes.keys().collect::<Vec<_>>(), [&uri]);
    let edits = changes[&uri].as_array().unwrap();
    assert_eq!(edits.len(), 1);

    // Applied, the stub lands inside `first`'s block, before its brace.
    let edited = insert(text, &edits[0]);
    assert_eq!(
        edited,
        "behavior first \"First\" {\n  contract \"c\"\n  verify unit \"first — TODO\"\n}\n\n\
         behavior second \"Second\" {\n  contract \"c\"\n  verify unit \"s\"\n}\n"
    );
    let parsed = specforge_parser::parse(&edited, "flows.spec");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let first = parsed
        .entities
        .iter()
        .find(|e| e.id.raw == "first")
        .unwrap();
    assert!(first.fields.get("verify").is_some(), "{:?}", first.fields);
}

/// `text` with the zero-width insertion `edit` applied.
fn insert(text: &str, edit: &serde_json::Value) -> String {
    let (start, end) = (&edit["range"]["start"], &edit["range"]["end"]);
    assert_eq!(start, end, "an insertion, not a replacement: {edit}");
    let line = start["line"].as_u64().unwrap() as usize;
    let character = start["character"].as_u64().unwrap() as usize;
    let offset: usize = text.split_inclusive('\n').take(line).map(str::len).sum();
    let offset = offset + character;
    format!(
        "{}{}{}",
        &text[..offset],
        edit["newText"].as_str().unwrap(),
        &text[offset..]
    )
}

#[test]
fn verify_stub_uses_unit_kind() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions =
        specforge_lsp::code_actions_missing_verify(&g, "a.spec", &verifiable(&["behavior"], &[]));
    assert!(actions[0].edit_text.contains("verify unit"));
}

#[spec(
    behavior = "code_actions_for_missing_verify",
    verify = "stub format is verify <kind> entity_id TODO"
)]
fn verify_stub_format() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions =
        specforge_lsp::code_actions_missing_verify(&g, "a.spec", &verifiable(&["behavior"], &[]));
    assert!(actions[0].edit_text.contains("verify unit \"my_behavior"));
    assert!(actions[0].edit_text.contains("TODO"));
}

#[spec(
    behavior = "code_actions_for_missing_verify",
    verify = "code action kind is QuickFix"
)]
fn verify_action_is_quickfix() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions =
        specforge_lsp::code_actions_missing_verify(&g, "a.spec", &verifiable(&["behavior"], &[]));
    assert_eq!(actions[0].action_kind, "quickfix");
}

#[spec(
    behavior = "code_actions_for_missing_verify",
    verify = "no test source files or application code generated"
)]
fn verify_action_no_code_gen() {
    let mut g = Graph::new();
    g.add_node(node("my_behavior", "behavior", "a.spec", 5));

    let actions =
        specforge_lsp::code_actions_missing_verify(&g, "a.spec", &verifiable(&["behavior"], &[]));
    // The edit should only modify the .spec file, not create new files
    assert!(actions[0].file.ends_with(".spec"));
}

#[spec(
    behavior = "code_actions_for_missing_verify",
    verify = "verify stub uses allowed_verify_kinds from KindRegistry"
)]
fn verify_stub_uses_the_kinds_first_allowed_verify_kind() {
    let mut g = Graph::new();
    g.add_node(node("unique_ids", "invariant", "a.spec", 5));
    g.add_node(node("untestable", "feature", "a.spec", 12));

    let registry = verifiable(&["invariant"], &["property", "unit"]);
    let actions = specforge_lsp::code_actions_missing_verify(&g, "a.spec", &registry);

    assert_eq!(actions.len(), 1, "the feature takes no verify statements");
    assert_eq!(
        actions[0].edit_text,
        "  verify property \"unique_ids — TODO\""
    );
}

// -- code_action_create_entity_stub -------------------------------------------

#[test]
fn create_stub_offered() {
    let _g = Graph::new();
    let action =
        specforge_lsp::code_action_create_stub("missing_type", Some("type"), "current.spec");
    assert!(action.is_some());
}

#[test]
fn stub_uses_correct_kind() {
    let action =
        specforge_lsp::code_action_create_stub("missing_type", Some("type"), "current.spec")
            .unwrap();
    assert!(action.edit_text.starts_with("type missing_type"));
}

#[spec(
    behavior = "code_action_create_entity_stub",
    verify = "no code action when enclosing field has no target_kind"
)]
fn no_stub_without_target_kind() {
    let action = specforge_lsp::code_action_create_stub("unknown_thing", None, "current.spec");
    assert!(action.is_none());
}

#[test]
fn stub_targets_current_file() {
    let action =
        specforge_lsp::code_action_create_stub("my_event", Some("event"), "current.spec").unwrap();
    assert_eq!(action.file, "current.spec");
}

#[spec(
    behavior = "code_action_create_entity_stub",
    verify = "code action kind is Refactor"
)]
fn stub_action_is_refactor() {
    let action =
        specforge_lsp::code_action_create_stub("my_event", Some("event"), "current.spec").unwrap();
    assert_eq!(action.action_kind, "refactor");
}

#[spec(
    behavior = "code_action_create_entity_stub",
    verify = "generated stub contains no application code or test files"
)]
fn stub_no_app_code() {
    let action =
        specforge_lsp::code_action_create_stub("my_event", Some("event"), "current.spec").unwrap();
    // Should be a minimal spec block, not code
    assert!(action.edit_text.contains("event my_event"));
    assert!(action.edit_text.contains('{'));
    assert!(!action.edit_text.contains("fn "));
    assert!(!action.edit_text.contains("class "));
}

// -- actions from a diagnostic's data, not its text ----------------------------

/// An E003 on `line` (1-based) whose message and suggestion say nothing a
/// parser could use: only its data names the reference.
fn reworded_e003(line: usize, data: Option<DiagnosticData>) -> Diagnostic {
    let mut diag =
        Diagnostic::error("E003", "this wording is not a contract").with_span(SourceSpan {
            file: Sym::new("auth.spec"),
            start_line: line,
            start_col: 14,
            end_line: line,
            end_col: 25,
        });
    diag.data = data.map(Box::new);
    diag
}

fn unresolved(target: &str, entity: &str, field: &str, close: Option<&str>) -> DiagnosticData {
    DiagnosticData::UnresolvedReference {
        target: target.into(),
        entity: entity.into(),
        field: field.into(),
        did_you_mean: close.map(String::from),
    }
}

#[spec(
    behavior = "emit_live_diagnostics",
    verify = "code actions act on the diagnostics last published for the document"
)]
fn a_rename_quickfix_reads_the_data_whatever_the_message_says() {
    let content = "behavior login \"L\" {\n  invariants [tokn_unique]\n}\n";
    let diag = reworded_e003(
        2,
        Some(unresolved(
            "tokn_unique",
            "login",
            "invariants",
            Some("token_unique"),
        )),
    );

    let actions = specforge_lsp::code_actions_from_diagnostics(&[diag], content);

    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].title, "Replace with 'token_unique'");
    assert_eq!(actions[0].edit_text, "token_unique");
    assert_eq!(actions[0].insert_line, 2);
    assert_eq!(actions[0].replace_cols, Some((14, 25)));
}

#[test]
fn the_old_message_and_suggestion_text_alone_offer_nothing() {
    let content = "behavior login \"L\" {\n  invariants [tokn_unique]\n}\n";
    let mut diag = reworded_e003(2, None);
    diag.message = "unresolved reference 'tokn_unique' in entity 'login'".into();
    diag.suggestion = Some("did you mean 'token_unique'?".into());

    assert!(specforge_lsp::code_actions_from_diagnostics(&[diag], content).is_empty());
}

#[test]
fn an_import_rename_quickfix_reads_the_path_from_the_data() {
    let content = "use \"autth\"\n";
    let diag = Diagnostic::error("E025", "reworded")
        .with_span(SourceSpan {
            file: Sym::new("main.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 11,
        })
        .with_data(DiagnosticData::UnresolvedImport {
            path: "autth".into(),
            did_you_mean: Some("auth".into()),
        });

    let actions = specforge_lsp::code_actions_from_diagnostics(&[diag], content);

    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].edit_text, "auth");
    assert_eq!(actions[0].replace_cols, Some((5, 10)));
}

/// `behavior.invariants` targets the `invariant` kind.
fn invariants_target_invariant() -> specforge_registry::FieldRegistry {
    use specforge_registry::{FieldRegistry, FieldRegistryEntry, ManifestFieldType};
    let mut reg = FieldRegistry::new();
    reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_name: "invariants".into(),
        description: None,
        field_type: ManifestFieldType::ReferenceList,
        source_extension: "@specforge/software".into(),
        edge: None,
        target_kind: Some("invariant".into()),
        file_reference: false,
        required: false,
        inverse_of: None,
        normative: false,
        exempts_obligations: false,
        headline: false,
        derived_from: None,
    });
    reg
}

#[spec(
    behavior = "code_action_create_entity_stub",
    verify = "the stub is read from the diagnostic's data, whatever its message says"
)]
fn a_stub_reads_target_entity_and_field_from_the_data() {
    let mut graph = Graph::new();
    graph.add_node(node("login", "behavior", "auth.spec", 1));
    let data = unresolved("session_limit", "login", "invariants", None);
    // Twice: one stub per target.
    let diags = [
        reworded_e003(2, Some(data.clone())),
        reworded_e003(3, Some(data)),
    ];

    let actions = specforge_lsp::code_actions_create_stubs(
        &diags,
        &graph,
        &invariants_target_invariant(),
        "auth.spec",
    );

    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].title, "Create invariant stub for session_limit");
    assert_eq!(actions[0].file, "auth.spec");

    // The same diagnostic as text alone, in the wording the compiler
    // prints today, offers no stub.
    let mut text_only = reworded_e003(2, None);
    text_only.message = "unresolved reference 'session_limit' in entity 'login'".into();
    assert!(
        specforge_lsp::code_actions_create_stubs(
            &[text_only],
            &graph,
            &invariants_target_invariant(),
            "auth.spec",
        )
        .is_empty()
    );
}

#[test]
fn no_stub_for_a_target_that_now_exists() {
    let mut graph = Graph::new();
    graph.add_node(node("login", "behavior", "auth.spec", 1));
    graph.add_node(node("session_limit", "invariant", "auth.spec", 5));
    let diags = [reworded_e003(
        2,
        Some(unresolved("session_limit", "login", "invariants", None)),
    )];

    assert!(
        specforge_lsp::code_actions_create_stubs(
            &diags,
            &graph,
            &invariants_target_invariant(),
            "auth.spec",
        )
        .is_empty()
    );
}
