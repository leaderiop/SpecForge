//! The structural keywords (`spec`, `ref`, `use`, `define`) keep their own
//! grammar rules beside the one generic entity block: they parse with no
//! extension loaded, since the parser knows no entity kind.

use specforge_test_macros::test as spec;

#[spec(
    behavior = "collapse_grammar_to_generic_entity_block",
    verify = "spec_block remains as separate grammar rule"
)]
fn a_spec_block_parses_without_extensions() {
    let parsed = specforge_parser::parse("spec \"my-spec\" {\n  version \"1.0\"\n}\n", "test.spec");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].kind.raw, "spec");
}

#[spec(
    behavior = "collapse_grammar_to_generic_entity_block",
    verify = "ref_block remains as separate grammar rule"
)]
fn a_ref_block_parses_without_extensions() {
    let parsed = specforge_parser::parse("ref gh.issue:42 \"Fix bug\"\n", "test.spec");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].kind.raw, "ref");
}

#[spec(
    behavior = "collapse_grammar_to_generic_entity_block",
    verify = "use_import remains as separate grammar rule"
)]
fn a_use_import_parses_without_extensions() {
    let parsed = specforge_parser::parse("use \"types/core\"\n", "test.spec");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    assert_eq!(parsed.imports.len(), 1);
    assert!(parsed.entities.is_empty());
}

#[spec(
    behavior = "collapse_grammar_to_generic_entity_block",
    verify = "define_block remains as separate grammar rule"
)]
fn a_define_block_parses_without_extensions() {
    let parsed = specforge_parser::parse(
        "define user_story {\n  required [description]\n}\n",
        "test.spec",
    );
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    assert_eq!(parsed.entities.len(), 1, "{:?}", parsed.entities);
    assert_eq!(parsed.entities[0].kind.raw, "define");
    assert_eq!(parsed.entities[0].id.raw, "user_story");
}
