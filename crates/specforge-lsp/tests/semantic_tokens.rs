use specforge_lsp::{MOD_DECLARATION, MOD_REFERENCE, SemanticToken};
use specforge_registry::{KindRegistry, KindRegistryEntry};
use specforge_test_macros::test as spec;

/// A registry of `(kind keyword, declared semantic_token)` pairs.
fn kinds(entries: &[(&str, Option<&str>)]) -> KindRegistry {
    let mut registry = KindRegistry::new();
    for (kind, token) in entries {
        registry.register(KindRegistryEntry {
            kind_name: kind.to_string(),
            source_extension: "@test/ext".into(),
            testable: true,
            supports_verify: true,
            allowed_verify_kinds: vec![],
            lifecycle_field: None,
            declared: specforge_registry::EntityKindDescriptor {
                semantic_token: token.map(str::to_string),
                ..Default::default()
            },
        });
    }
    registry
}

/// The token classified for `text`, which must exist.
fn token<'a>(tokens: &'a [SemanticToken], text: &str) -> &'a SemanticToken {
    tokens
        .iter()
        .find(|t| t.text == text)
        .unwrap_or_else(|| panic!("no token {text:?} in {tokens:?}"))
}

// -- provide_semantic_tokens --------------------------------------------------

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "entity keywords classified as 'type'"
)]
fn entity_keywords_classified_as_type() {
    // A kind's semantic_token classifies its IDs, never its keyword.
    let tokens = specforge_lsp::classify_tokens(
        "port repo \"Repo\" {\n}\n",
        &kinds(&[("port", Some("interface"))]),
    );
    assert_eq!(token(&tokens, "port").token_type, "type");
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "entity ID declaration uses its kind's semantic_token from the KindRegistry"
)]
fn entity_id_declaration_uses_kind_semantic_token() {
    let tokens = specforge_lsp::classify_tokens(
        "port repo \"Repo\" {\n}\ninvariant always \"Always\" {\n}\n",
        &kinds(&[("port", Some("interface")), ("invariant", Some("property"))]),
    );
    assert_eq!(token(&tokens, "repo").token_type, "interface");
    assert_eq!(token(&tokens, "always").token_type, "property");
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "entity ID declaration without a declared semantic_token is 'function'"
)]
fn entity_id_declaration_without_semantic_token_is_function() {
    let tokens =
        specforge_lsp::classify_tokens("gizmo thing \"Thing\" {\n}\n", &kinds(&[("gizmo", None)]));
    assert_eq!(token(&tokens, "thing").token_type, "function");
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "entity ID declaration whose semantic_token is not in the legend is 'function'"
)]
fn entity_id_declaration_with_unknown_semantic_token_is_function() {
    // "constant" is no standard LSP token type, so no client could color it.
    let tokens = specforge_lsp::classify_tokens(
        "axiom excluded_middle \"Excluded Middle\" {\n}\n",
        &kinds(&[("axiom", Some("constant"))]),
    );
    assert!(!specforge_lsp::TOKEN_TYPES.contains(&"constant"));
    assert_eq!(token(&tokens, "excluded_middle").token_type, "function");
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "semantic token legend lists every standard LSP token type"
)]
fn legend_lists_every_standard_token_type() {
    let standard = [
        "namespace",
        "type",
        "class",
        "enum",
        "interface",
        "struct",
        "typeParameter",
        "parameter",
        "variable",
        "property",
        "enumMember",
        "event",
        "function",
        "method",
        "macro",
        "keyword",
        "modifier",
        "comment",
        "string",
        "number",
        "regexp",
        "operator",
        "decorator",
    ];
    let mut legend = specforge_lsp::TOKEN_TYPES.to_vec();
    legend.sort_unstable();
    let mut expected = standard.to_vec();
    expected.sort_unstable();
    assert_eq!(legend, expected);
}

// -- provide_extension_entity_semantic_tokens ---------------------------------

#[spec(
    behavior = "provide_extension_entity_semantic_tokens",
    verify = "extension kind's semantic_token classifies its entity ID declaration"
)]
fn extension_kind_semantic_token_classifies_declaration() {
    let tokens = specforge_lsp::classify_tokens(
        "feature checkout \"Checkout\" {\n}\n",
        &kinds(&[("feature", Some("class"))]),
    );
    let id = token(&tokens, "checkout");
    assert_eq!(id.token_type, "class");
    assert_ne!(id.modifiers & MOD_DECLARATION, 0);
}

#[spec(
    behavior = "provide_extension_entity_semantic_tokens",
    verify = "entity ID declaration falls back to 'function' when semantic_token is not specified"
)]
fn extension_kind_without_semantic_token_falls_back_to_function() {
    let tokens =
        specforge_lsp::classify_tokens("widget knob \"Knob\" {\n}\n", &kinds(&[("widget", None)]));
    assert_eq!(token(&tokens, "knob").token_type, "function");
}

#[spec(
    behavior = "provide_extension_entity_semantic_tokens",
    verify = "semantic_token outside the static legend falls back to 'function'"
)]
fn extension_semantic_token_outside_legend_falls_back_to_function() {
    let tokens = specforge_lsp::classify_tokens(
        "release v1 \"V1\" {\n}\n",
        &kinds(&[("release", Some("constant"))]),
    );
    assert_eq!(token(&tokens, "v1").token_type, "function");
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "structural keywords are classified as keyword"
)]
fn structural_keywords_classified() {
    let tokens = specforge_lsp::classify_tokens("use \"behaviors/core\"\n", &KindRegistry::new());
    assert!(
        tokens
            .iter()
            .any(|t| t.text == "use" && t.token_type == "keyword")
    );
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "triple-quoted strings are classified as strings"
)]
fn triple_quoted_strings_classified() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  contract \"\"\"\n    hello\n  \"\"\"\n}\nbehavior bar \"Bar\" {\n}\n",
        &kinds(&[("behavior", None)]),
    );
    let on_lines = |lines: std::ops::RangeInclusive<usize>| {
        tokens
            .iter()
            .filter(|t| lines.contains(&t.line))
            .map(|t| (t.line, t.col, t.text.as_str(), t.token_type.as_str()))
            .collect::<Vec<_>>()
    };
    // The opening quotes, the body and the closing quotes are strings.
    assert_eq!(
        on_lines(1..=3),
        [
            (1, 2, "contract", "property"),
            (1, 11, "\"\"\"", "string"),
            (2, 0, "    hello", "string"),
            (3, 0, "  \"\"\"", "string"),
        ]
    );
    // The string ends at its closing quotes: what follows is not a string.
    assert_eq!(token(&tokens, "bar").token_type, "function");
    assert!(
        tokens
            .iter()
            .filter(|t| t.line > 3)
            .all(|t| t.token_type != "string" || t.text == "\"Bar\""),
        "{tokens:?}"
    );
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "entity ID declarations carry the declaration modifier"
)]
fn entity_id_declarations_carry_declaration_modifier() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n}\nport repo \"Repo\" {\n}\n",
        &kinds(&[("behavior", None), ("port", Some("interface"))]),
    );
    for id in ["foo", "repo"] {
        assert_ne!(
            token(&tokens, id).modifiers & MOD_DECLARATION,
            0,
            "{id} should have the declaration modifier"
        );
    }
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "enhanced fields are classified as property"
)]
fn fields_classified_as_property() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  contract \"x\"\n}\n",
        &kinds(&[("behavior", None)]),
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.text == "contract" && t.token_type == "property")
    );
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "reference list items classified as 'variable' with reference modifier"
)]
fn reference_list_items_classified_as_variable() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  invariants [inv_a, inv_b]\n}\n",
        &kinds(&[("behavior", None)]),
    );
    let inv_a = tokens.iter().find(|t| t.text == "inv_a").unwrap();
    assert_eq!(inv_a.token_type, "variable");
    assert_ne!(
        inv_a.modifiers & MOD_REFERENCE,
        0,
        "should have reference modifier"
    );

    let inv_b = tokens.iter().find(|t| t.text == "inv_b").unwrap();
    assert_eq!(inv_b.token_type, "variable");
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "structural keywords are classified as keyword"
)]
fn verify_keyword_classified() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  verify unit \"it works\"\n}\n",
        &kinds(&[("behavior", None)]),
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.text == "verify" && t.token_type == "keyword")
    );
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "verify kind classified as enumMember"
)]
fn verify_kind_classified_as_enum_member() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  verify unit \"it works\"\n}\n",
        &kinds(&[("behavior", None)]),
    );
    let kind = tokens.iter().find(|t| t.text == "unit").unwrap();
    assert_eq!(kind.token_type, "enumMember");
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "comments classified as comment"
)]
fn comments_classified() {
    let tokens = specforge_lsp::classify_tokens("// this is a comment\n", &KindRegistry::new());
    assert!(tokens.iter().any(|t| t.token_type == "comment"));
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "structural keywords are classified as keyword"
)]
fn define_block_classified() {
    let tokens = specforge_lsp::classify_tokens("define MyType {\n}\n", &KindRegistry::new());
    assert!(
        tokens
            .iter()
            .any(|t| t.text == "define" && t.token_type == "keyword")
    );
    let name = tokens.iter().find(|t| t.text == "MyType").unwrap();
    assert_eq!(name.token_type, "function");
    assert_ne!(name.modifiers & MOD_DECLARATION, 0);
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "reference list items classified as 'variable' with reference modifier"
)]
fn multiline_reference_list() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  types [\n    type_a,\n    type_b\n  ]\n}\n",
        &kinds(&[("behavior", None)]),
    );
    let type_a = tokens.iter().find(|t| t.text == "type_a").unwrap();
    assert_eq!(type_a.token_type, "variable");
    assert_ne!(type_a.modifiers & MOD_REFERENCE, 0);

    let type_b = tokens.iter().find(|t| t.text == "type_b").unwrap();
    assert_eq!(type_b.token_type, "variable");
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "enhanced fields are classified as property"
)]
fn field_name_before_list_classified() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  invariants [inv_a]\n}\n",
        &kinds(&[("behavior", None)]),
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.text == "invariants" && t.token_type == "property")
    );
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "number values classified as number"
)]
fn number_values_classified() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"Foo\" {\n  risk 5\n}\n",
        &kinds(&[("behavior", None)]),
    );
    let num = tokens.iter().find(|t| t.text == "5").unwrap();
    assert_eq!(num.token_type, "number");
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "entity title strings classified as string"
)]
fn entity_title_classified_as_string() {
    let tokens = specforge_lsp::classify_tokens(
        "behavior foo \"My Title\" {\n}\n",
        &kinds(&[("behavior", None)]),
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.text.contains("My Title") && t.token_type == "string")
    );
}

#[spec(
    behavior = "provide_semantic_tokens",
    verify = "use path classified as string"
)]
fn use_path_classified_as_string() {
    let tokens = specforge_lsp::classify_tokens("use \"core/types\"\n", &KindRegistry::new());
    assert!(
        tokens
            .iter()
            .any(|t| t.text.contains("core/types") && t.token_type == "string")
    );
}

#[spec(
    behavior = "lsp_initialize",
    verify = "initialize response includes semantic token legend"
)]
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

#[spec(
    behavior = "lsp_initialize",
    verify = "initialize response includes semantic token legend"
)]
fn token_modifiers_constant_complete() {
    let mods = specforge_lsp::TOKEN_MODIFIERS;
    assert!(mods.contains(&"declaration"));
    assert!(mods.contains(&"reference"));
}

/// An entity body with a nested block before two more fields, the last
/// one followed by a comment.
const AFTER_A_NESTED_BLOCK: &str = concat!(
    "behavior gamma \"Gamma\" {\n",
    "  requires {\n",
    "    ready \"it is ready\"\n",
    "  }\n",
    "  contract \"after the block\"\n",
    "  features [] // trailing comment\n",
    "}\n",
);

// Flipped by 06-T5.
#[test]
fn pin_fields_after_a_nested_block_are_unclassified() {
    let tokens =
        specforge_lsp::classify_tokens(AFTER_A_NESTED_BLOCK, &kinds(&[("behavior", None)]));
    assert!(
        tokens.iter().all(|t| t.line != 4 && t.line != 5),
        "{tokens:?}"
    );
}

// Flipped by 06-T5.
#[test]
fn pin_a_trailing_comment_is_unclassified() {
    let tokens =
        specforge_lsp::classify_tokens(AFTER_A_NESTED_BLOCK, &kinds(&[("behavior", None)]));
    assert!(
        !tokens
            .iter()
            .any(|t| t.line == 5 && t.token_type == "comment"),
        "{tokens:?}"
    );
}
