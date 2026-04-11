use specforge_parser::{parse, FieldValue};
use specforge_test_macros::test as specforge_test;

// B:parse_spec_file_to_ast — verify contract "requires/ensures consistency for spec file parsing"
#[test]
#[specforge_test(behavior = "parse_spec_file_to_ast", verify = "requires/ensures consistency for spec file parsing")]
fn parse_spec_file_to_ast_contract() {
    // Requires: valid UTF-8 source string and file path
    // Ensures: entities populated, no errors, path preserved
    let source = r#"
use "types/core"

behavior alpha "Alpha" {
    contract "The system MUST handle alpha"
    status planned
}

behavior beta "Beta" {
    contract "The system MUST handle beta"
}
"#;
    let result = parse(source, "behaviors/test.spec");

    assert!(result.errors.is_empty(), "valid source must produce no errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 2, "two entities must be populated");
    assert_eq!(result.imports.len(), 1, "one import must be populated");
    assert_eq!(result.path, "behaviors/test.spec", "path must be preserved");

    // Each entity has kind, id, span
    for entity in &result.entities {
        assert!(!entity.kind.raw.is_empty(), "entity kind must be populated");
        assert!(!entity.id.raw.is_empty(), "entity id must be populated");
        assert_eq!(entity.span.file, "behaviors/test.spec", "span file must match input");
    }
}

// B:recover_from_syntax_errors — verify contract "requires/ensures consistency for syntax error recovery"
#[test]
#[specforge_test(behavior = "recover_from_syntax_errors", verify = "requires/ensures consistency for syntax error recovery")]
fn recover_from_syntax_errors_contract() {
    // Requires: source with syntax errors
    // Ensures: partial AST produced + errors collected (no panic)
    let source = r#"
behavior good "Good" {
    status planned
}

behavior ??? {
    broken content
}

behavior also_good "Also Good" {
    status done
}
"#;
    let result = parse(source, "test.spec");

    // Must not panic — reaching here proves no panic
    assert!(!result.errors.is_empty(), "broken source must produce errors");
    // Partial AST: at least the valid blocks should be recovered
    assert!(
        !result.entities.is_empty(),
        "must produce partial AST even with errors"
    );
    // Each error has a span with file information
    for error in &result.errors {
        assert_eq!(error.span.file, "test.spec", "error span must reference source file");
    }
}

// B:parse_use_imports — verify contract "requires/ensures consistency for use import parsing"
#[test]
#[specforge_test(behavior = "parse_use_imports", verify = "requires/ensures consistency for use import parsing")]
fn parse_use_imports_contract() {
    // Requires: source with use declarations
    // Ensures: imports list populated with path, kind, and optional bindings
    let source = r#"
use "behaviors/parsing"
use { SpecFile, ParseError } from "types/core"
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.imports.len(), 2, "two imports must be parsed");

    // Full import
    assert_eq!(result.imports[0].path, "behaviors/parsing");
    assert_eq!(result.imports[0].kind, specforge_parser::ImportKind::Full);
    assert!(result.imports[0].bindings.is_none(), "full import has no bindings");

    // Selective import
    assert_eq!(result.imports[1].path, "types/core");
    assert_eq!(result.imports[1].kind, specforge_parser::ImportKind::Selective);
    let bindings = result.imports[1].bindings.as_ref().expect("selective import must have bindings");
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].name, "SpecFile");
    assert_eq!(bindings[1].name, "ParseError");
}

// B:parse_all_block_types — verify contract "requires/ensures consistency for block type parsing"
#[test]
#[specforge_test(behavior = "parse_all_block_types", verify = "requires/ensures consistency for block type parsing")]
fn parse_all_block_types_contract() {
    // Requires: source with behavior, feature, type (different keywords)
    // Ensures: all parsed with correct kind, id, and title
    let source = r#"
behavior do_auth "Do Auth" {
    contract "auth"
}

feature auth_flow "Auth Flow" {
    problem "need auth"
}

type AuthToken {
    path string
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 3, "all three block types must be parsed");

    assert_eq!(result.entities[0].kind.raw, "behavior");
    assert_eq!(result.entities[0].id.raw, "do_auth");
    assert_eq!(result.entities[0].title.as_deref(), Some("Do Auth"));

    assert_eq!(result.entities[1].kind.raw, "feature");
    assert_eq!(result.entities[1].id.raw, "auth_flow");

    assert_eq!(result.entities[2].kind.raw, "type");
    assert_eq!(result.entities[2].id.raw, "AuthToken");
}

// B:parse_triple_quoted_strings — verify contract "requires/ensures consistency for triple-quoted string parsing"
#[test]
#[specforge_test(behavior = "parse_triple_quoted_strings", verify = "requires/ensures consistency for triple-quoted string parsing")]
fn parse_triple_quoted_strings_contract() {
    // Requires: entity with triple-quoted string field
    // Ensures: field preserves content with common indentation stripped
    let source = "behavior doc \"Doc\" {\n    contract \"\"\"\n        Given a valid input\n        When processed\n        Then output is correct\n    \"\"\"\n}\n";
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let contract = result.entities[0].fields.get("contract").expect("missing contract field");
    match contract {
        FieldValue::String(s) => {
            assert!(s.contains("Given a valid input"), "content must be preserved");
            assert!(s.contains("When processed"), "content must be preserved");
            assert!(s.contains("Then output is correct"), "content must be preserved");
            assert!(!s.starts_with("        "), "common indentation must be stripped");
        }
        other => panic!("expected String, got {:?}", other),
    }
}

// B:parse_verify_statements — verify contract "requires/ensures consistency for verify statement parsing"
#[test]
#[specforge_test(behavior = "parse_verify_statements", verify = "requires/ensures consistency for verify statement parsing")]
fn parse_verify_statements_contract() {
    // Requires: entity with verify lines
    // Ensures: verify list with kind and description for each
    let source = r#"
behavior validate "Validate" {
    verify unit "rejects invalid input"
    verify integration "validates end-to-end"
    verify property "N inputs produce N outputs"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    let verify = result.entities[0].fields.get("verify").expect("missing verify field");
    match verify {
        FieldValue::VerifyList(stmts) => {
            assert_eq!(stmts.len(), 3, "three verify statements must be parsed");
            assert_eq!(stmts[0].kind, "unit");
            assert_eq!(stmts[0].description, "rejects invalid input");
            assert_eq!(stmts[1].kind, "integration");
            assert_eq!(stmts[2].kind, "property");
        }
        other => panic!("expected VerifyList, got {:?}", other),
    }
}

// B:parse_ref_blocks — verify contract "requires/ensures consistency for ref block parsing"
#[test]
#[specforge_test(behavior = "parse_ref_blocks", verify = "requires/ensures consistency for ref block parsing")]
fn parse_ref_blocks_contract() {
    // Requires: ref block with scheme.kind:identifier syntax
    // Ensures: ref entity in AST with decomposed scheme, ref_kind, identifier
    let source = r#"ref gh.issue:42 "Support Wasm extensions""#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "ref");
    assert_eq!(entity.id.raw, "gh.issue:42");
    assert_eq!(entity.title.as_deref(), Some("Support Wasm extensions"));
    assert!(matches!(entity.fields.get("scheme"), Some(FieldValue::String(s)) if s == "gh"));
    assert!(matches!(entity.fields.get("ref_kind"), Some(FieldValue::String(s)) if s == "issue"));
    assert!(matches!(entity.fields.get("identifier"), Some(FieldValue::String(s)) if s == "42"));
}

// B:parse_define_blocks — verify contract "requires/ensures consistency for define block parsing"
#[test]
#[specforge_test(behavior = "parse_define_blocks", verify = "requires/ensures consistency for define block parsing")]
fn parse_define_blocks_contract() {
    // Requires: define block with name and body fields
    // Ensures: define entity in AST with kind="define", no title, fields preserved
    let source = r#"
define my_custom_type {
    base_kind "entity"
    testable true
    verify unit "custom type works"
}
"#;
    let result = parse(source, "test.spec");

    assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
    assert_eq!(result.entities.len(), 1);

    let entity = &result.entities[0];
    assert_eq!(entity.kind.raw, "define");
    assert_eq!(entity.id.raw, "my_custom_type");
    assert!(entity.title.is_none(), "define blocks have no title");
    assert!(matches!(entity.fields.get("base_kind"), Some(FieldValue::String(s)) if s == "entity"));
    assert!(matches!(entity.fields.get("testable"), Some(FieldValue::Boolean(true))));
    assert!(entity.fields.get("verify").is_some(), "verify field must be present");
}
