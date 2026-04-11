use specforge_lsp::{MOD_DECLARATION, MOD_REFERENCE};
use specforge_test_macros::test as spec;

// -- provide_semantic_tokens --------------------------------------------------

#[spec(behavior = "provide_semantic_tokens", verify = "entity keywords classified as 'type'")]
#[test]
fn entity_keywords_classified_as_type() {
    let tokens = specforge_lsp::classify_tokens("behavior foo \"Foo\" {\n}\n", &["behavior"]);
    let tok = tokens.iter().find(|t| t.text == "behavior").unwrap();
    assert_eq!(tok.token_type, "type");
}

#[spec(behavior = "provide_semantic_tokens", verify = "structural keywords are classified as keyword")]
#[test]
fn structural_keywords_classified() {
    let tokens = specforge_lsp::classify_tokens("use \"behaviors/core\"\n", &[]);
    assert!(tokens.iter().any(|t| t.text == "use" && t.token_type == "keyword"));
}

#[spec(behavior = "provide_semantic_tokens", verify = "triple-quoted strings are classified as strings")]
#[test]
fn triple_quoted_strings_classified() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  contract \"\"\"\n    hello\n  \"\"\"\n}\n",
        &["behavior"],
    );
    assert!(tokens.iter().any(|t| t.token_type == "string"));
}

#[spec(behavior = "provide_semantic_tokens", verify = "entity IDs classified as 'function' with declaration modifier")]
#[test]
fn entity_ids_classified_as_function_declaration() {
    let tokens = specforge_lsp::classify_tokens("behavior foo \"Foo\" {\n}\n", &["behavior"]);
    let tok = tokens.iter().find(|t| t.text == "foo").unwrap();
    assert_eq!(tok.token_type, "function");
    assert_ne!(tok.modifiers & MOD_DECLARATION, 0, "should have declaration modifier");
}

#[spec(behavior = "provide_semantic_tokens", verify = "enhanced fields are classified as property")]
#[test]
fn fields_classified_as_property() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  contract \"x\"\n}\n",
        &["behavior"],
    );
    assert!(tokens.iter().any(|t| t.text == "contract" && t.token_type == "property"));
}

#[spec(behavior = "provide_semantic_tokens", verify = "reference list items classified as 'variable' with reference modifier")]
#[test]
fn reference_list_items_classified_as_variable() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  invariants [inv_a, inv_b]\n}\n",
        &["behavior"],
    );
    let inv_a = tokens.iter().find(|t| t.text == "inv_a").unwrap();
    assert_eq!(inv_a.token_type, "variable");
    assert_ne!(inv_a.modifiers & MOD_REFERENCE, 0, "should have reference modifier");

    let inv_b = tokens.iter().find(|t| t.text == "inv_b").unwrap();
    assert_eq!(inv_b.token_type, "variable");
}

#[spec(behavior = "provide_semantic_tokens", verify = "verify keyword classified as keyword")]
#[test]
fn verify_keyword_classified() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  verify unit \"it works\"\n}\n",
        &["behavior"],
    );
    assert!(tokens.iter().any(|t| t.text == "verify" && t.token_type == "keyword"));
}

#[spec(behavior = "provide_semantic_tokens", verify = "verify kind classified as enumMember")]
#[test]
fn verify_kind_classified_as_enum_member() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  verify unit \"it works\"\n}\n",
        &["behavior"],
    );
    let kind = tokens.iter().find(|t| t.text == "unit").unwrap();
    assert_eq!(kind.token_type, "enumMember");
}

#[spec(behavior = "provide_semantic_tokens", verify = "comments classified as comment")]
#[test]
fn comments_classified() {
    let tokens = specforge_lsp::classify_tokens("// this is a comment\n", &[]);
    assert!(tokens.iter().any(|t| t.token_type == "comment"));
}

#[spec(behavior = "provide_semantic_tokens", verify = "define keyword classified as keyword, define name as function")]
#[test]
fn define_block_classified() {
    let tokens = specforge_lsp::classify_tokens("define MyType {\n}\n", &[]);
    assert!(tokens.iter().any(|t| t.text == "define" && t.token_type == "keyword"));
    let name = tokens.iter().find(|t| t.text == "MyType").unwrap();
    assert_eq!(name.token_type, "function");
    assert_ne!(name.modifiers & MOD_DECLARATION, 0);
}

#[spec(behavior = "provide_semantic_tokens", verify = "multiline reference lists classified correctly")]
#[test]
fn multiline_reference_list() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  types [\n    type_a,\n    type_b\n  ]\n}\n",
        &["behavior"],
    );
    let type_a = tokens.iter().find(|t| t.text == "type_a").unwrap();
    assert_eq!(type_a.token_type, "variable");
    assert_ne!(type_a.modifiers & MOD_REFERENCE, 0);

    let type_b = tokens.iter().find(|t| t.text == "type_b").unwrap();
    assert_eq!(type_b.token_type, "variable");
}

#[spec(behavior = "provide_semantic_tokens", verify = "field name before list classified as property")]
#[test]
fn field_name_before_list_classified() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  invariants [inv_a]\n}\n",
        &["behavior"],
    );
    assert!(tokens.iter().any(|t| t.text == "invariants" && t.token_type == "property"));
}

#[spec(behavior = "provide_semantic_tokens", verify = "number values classified as number")]
#[test]
fn number_values_classified() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  risk 5\n}\n",
        &["behavior"],
    );
    let num = tokens.iter().find(|t| t.text == "5").unwrap();
    assert_eq!(num.token_type, "number");
}

#[spec(behavior = "provide_semantic_tokens", verify = "entity title strings classified as string")]
#[test]
fn entity_title_classified_as_string() {
    let tokens = specforge_lsp::classify_tokens("behavior foo \"My Title\" {\n}\n", &["behavior"]);
    assert!(tokens.iter().any(|t| t.text.contains("My Title") && t.token_type == "string"));
}

#[spec(behavior = "provide_semantic_tokens", verify = "use path classified as string")]
#[test]
fn use_path_classified_as_string() {
    let tokens = specforge_lsp::classify_tokens("use \"core/types\"\n", &[]);
    assert!(tokens.iter().any(|t| t.text.contains("core/types") && t.token_type == "string"));
}

#[spec(behavior = "provide_semantic_tokens", verify = "TOKEN_TYPES constant has all expected types")]
#[test]
fn token_types_constant_complete() {
    let types = specforge_lsp::TOKEN_TYPES;
    assert!(types.contains(&"keyword"));
    assert!(types.contains(&"type"));
    assert!(types.contains(&"function"));
    assert!(types.contains(&"variable"));
    assert!(types.contains(&"property"));
    assert!(types.contains(&"string"));
    assert!(types.contains(&"comment"));
    assert!(types.contains(&"number"));
    assert!(types.contains(&"enumMember"));
}

#[spec(behavior = "provide_semantic_tokens", verify = "TOKEN_MODIFIERS constant has declaration and reference")]
#[test]
fn token_modifiers_constant_complete() {
    let mods = specforge_lsp::TOKEN_MODIFIERS;
    assert!(mods.contains(&"declaration"));
    assert!(mods.contains(&"reference"));
}
