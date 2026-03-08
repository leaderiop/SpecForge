use specforge_parser::{parse, FieldValue};

#[test]
fn parse_generic_entity_block() {
    let source = r#"behavior foo "My Title" {
  contract "hello world"
}"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "behavior");
    assert_eq!(entity.id.raw, "foo");
    assert_eq!(entity.title.as_deref(), Some("My Title"));

    let contract = entity.fields.get("contract").expect("missing contract field");
    match contract {
        FieldValue::String(s) => assert_eq!(s, "hello world"),
        other => panic!("expected StringValue, got {:?}", other),
    }
}

#[test]
fn parse_unknown_keyword_without_error() {
    let source = r#"zork my_thing "Alien Entity" {
  status "active"
}"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unknown keyword should not produce errors");
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].kind.raw, "zork");
    assert_eq!(result.entities[0].id.raw, "my_thing");
}

#[test]
fn parse_full_use_import() {
    let source = "use behaviors/parsing";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty());
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].path, "behaviors/parsing");
    assert!(result.imports[0].selected_ids.is_none());
}

#[test]
fn parse_selective_use_import() {
    let source = "use behaviors/parsing { parse_spec_file_to_ast, recover_from_syntax_errors }";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty());
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].path, "behaviors/parsing");
    let ids = result.imports[0].selected_ids.as_ref().expect("expected selective ids");
    assert_eq!(ids, &["parse_spec_file_to_ast", "recover_from_syntax_errors"]);
}

// --- Triple-quoted strings ---

#[test]
fn triple_quoted_string_preserves_newlines() {
    let source = r#"behavior foo "Title" {
  contract """
    line one
    line two
  """
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    let contract = result.entities[0].fields.get("contract").unwrap();
    match contract {
        FieldValue::String(s) => {
            assert!(s.contains("line one\nline two"), "got: {:?}", s);
        }
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn triple_quoted_string_strips_common_indent() {
    let source = r#"behavior foo "T" {
  contract """
    hello
      indented
    back
  """
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    let contract = result.entities[0].fields.get("contract").unwrap();
    match contract {
        FieldValue::String(s) => {
            assert_eq!(s, "hello\n  indented\nback");
        }
        other => panic!("expected String, got {:?}", other),
    }
}

// --- Verify statements ---

#[test]
fn parse_verify_in_entity_block() {
    let source = r#"behavior foo "T" {
  verify unit "first test"
  verify property "second test"
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    let verify = result.entities[0].fields.get("verify").unwrap();
    match verify {
        FieldValue::VerifyList(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].kind, "unit");
            assert_eq!(items[0].description, "first test");
            assert_eq!(items[1].kind, "property");
            assert_eq!(items[1].description, "second test");
        }
        other => panic!("expected VerifyList, got {:?}", other),
    }
}

#[test]
fn parse_verify_in_define_block() {
    let source = r#"define my_entity {
  testable true
  verify unit "define test"
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    assert_eq!(result.entities[0].kind.raw, "define");
    let verify = result.entities[0].fields.get("verify").unwrap();
    match verify {
        FieldValue::VerifyList(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].description, "define test");
        }
        other => panic!("expected VerifyList, got {:?}", other),
    }
}

// --- Ref blocks ---

#[test]
fn parse_ref_inline() {
    let source = r#"ref gh.issue:42 "Fix parsing bug""#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "ref");
    assert_eq!(entity.title.as_deref(), Some("Fix parsing bug"));

    match entity.fields.get("scheme").unwrap() {
        FieldValue::String(s) => assert_eq!(s, "gh"),
        other => panic!("expected String, got {:?}", other),
    }
    match entity.fields.get("ref_kind").unwrap() {
        FieldValue::String(s) => assert_eq!(s, "issue"),
        other => panic!("expected String, got {:?}", other),
    }
    match entity.fields.get("identifier").unwrap() {
        FieldValue::String(s) => assert_eq!(s, "42"),
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn parse_ref_full_block() {
    let source = r#"ref gh.issue:42 "Fix parsing bug" {
  status "open"
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "ref");
    match entity.fields.get("status").unwrap() {
        FieldValue::String(s) => assert_eq!(s, "open"),
        other => panic!("expected String, got {:?}", other),
    }
}

// --- Define blocks ---

#[test]
fn parse_define_block() {
    let source = r#"define risk_item {
  testable true
  singleton false
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "define");
    assert_eq!(entity.id.raw, "risk_item");
    assert!(entity.title.is_none());

    match entity.fields.get("testable").unwrap() {
        FieldValue::Boolean(b) => assert!(*b),
        other => panic!("expected Boolean, got {:?}", other),
    }
}

// --- Error recovery ---

#[test]
fn parser_collects_multiple_errors() {
    let source = r#"behavior foo "T" {
  contract !!!
}
behavior bar "T2" {
  status "ok"
}"#;
    let result = parse(source, "test.spec");

    assert!(!result.errors.is_empty(), "should have at least one error");
    // Valid block after error should still be parsed
    let valid_entities: Vec<_> = result.entities.iter().filter(|e| e.id.raw == "bar").collect();
    assert_eq!(valid_entities.len(), 1, "bar should be parsed despite earlier error");
}

#[test]
fn valid_blocks_after_error_are_preserved() {
    // A block with a bad field value, followed by a valid block
    let source = r#"behavior bad "B" {
  contract !!!
}

behavior good "OK" {
  status "fine"
}"#;
    let result = parse(source, "test.spec");

    assert!(!result.errors.is_empty());
    let good: Vec<_> = result.entities.iter().filter(|e| e.id.raw == "good").collect();
    assert_eq!(good.len(), 1, "valid block after error should be parsed");
}

// --- Field types ---

#[test]
fn parse_reference_list() {
    let source = r#"feature foo "T" {
  behaviors [parse_spec, recover_errors, build_graph]
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    match result.entities[0].fields.get("behaviors").unwrap() {
        FieldValue::ReferenceList(ids) => {
            assert_eq!(ids, &["parse_spec", "recover_errors", "build_graph"]);
        }
        other => panic!("expected ReferenceList, got {:?}", other),
    }
}

#[test]
fn parse_nested_block() {
    let source = r#"behavior foo "T" {
  requires {
    input_valid "Input must be UTF-8"
  }
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    match result.entities[0].fields.get("requires").unwrap() {
        FieldValue::Block(map) => {
            match map.get("input_valid").unwrap() {
                FieldValue::String(s) => assert_eq!(s, "Input must be UTF-8"),
                other => panic!("expected String, got {:?}", other),
            }
        }
        other => panic!("expected Block, got {:?}", other),
    }
}

// --- Source spans ---

#[test]
fn entity_has_accurate_source_span() {
    let source = "behavior foo \"T\" {\n  status \"ok\"\n}";
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty());

    let span = &result.entities[0].span;
    assert_eq!(span.file, "test.spec");
    assert_eq!(span.start_line, 1);
    assert_eq!(span.start_col, 1);
    assert_eq!(span.end_line, 3);
}

// --- Spec block ---

#[test]
fn parse_spec_block() {
    let source = r#"spec "My Project" {
  version "1.0.0"
  extensions ["@specforge/software"]
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "spec");
    match entity.fields.get("version").unwrap() {
        FieldValue::String(s) => assert_eq!(s, "1.0.0"),
        other => panic!("expected String, got {:?}", other),
    }
}

// --- Smoke: parse a real .spec file ---

#[test]
fn parse_real_spec_file() {
    let source = include_str!("../../../spec/behaviors/parsing.spec");
    let result = parse(source, "spec/behaviors/parsing.spec");

    // Should parse without panicking and find entities
    assert!(
        !result.entities.is_empty(),
        "should find entities in parsing.spec, got errors: {:?}",
        result.errors,
    );

    // Should find the parse_spec_file_to_ast behavior
    let found = result.entities.iter().any(|e| e.id.raw == "parse_spec_file_to_ast");
    assert!(found, "should find parse_spec_file_to_ast entity");
}

// --- Multiple entities in one file ---

#[test]
fn parse_multiple_entities() {
    let source = r#"use types/core

behavior alpha "A" {
  contract "first"
}

behavior beta "B" {
  contract "second"
  verify unit "test beta"
}

invariant gamma "G" {
  guarantee "always true"
}"#;
    let result = parse(source, "test.spec");
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.entities.len(), 3);
    assert_eq!(result.entities[0].id.raw, "alpha");
    assert_eq!(result.entities[1].id.raw, "beta");
    assert_eq!(result.entities[2].id.raw, "gamma");
    assert_eq!(result.entities[2].kind.raw, "invariant");
}
