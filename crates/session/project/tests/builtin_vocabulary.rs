//! The builtins' candidate vocabulary is generated beside them (ADR 0122).

use specforge_component::builtins::BUILTIN_VOCABULARY;
use specforge_component::committed::pinned_builtin_declarations;
use specforge_registry::CandidateVocabulary;

#[specforge_test_macros::test(
    behavior = "suggest_missing_extensions",
    verify = "the builtin vocabulary is what the builtins' pinned declarations declare"
)]
fn the_builtin_vocabulary_is_what_the_builtins_declare() {
    let declarations = pinned_builtin_declarations().unwrap();
    let expected = CandidateVocabulary::of(&declarations).unwrap().to_json();
    assert_eq!(
        BUILTIN_VOCABULARY, expected,
        "the builtin vocabulary is stale: re-vendor with cargo run -p xtask --bin revendor"
    );
    assert!(!CandidateVocabulary::from_json(BUILTIN_VOCABULARY).is_empty());
}

#[specforge_test_macros::test(
    behavior = "link_entity_references",
    verify = "no builtin reference field declares a default value, so an absent value names nothing"
)]
fn no_builtin_reference_field_declares_a_default() {
    let declarations = pinned_builtin_declarations().unwrap();
    let mut checked = 0;
    for field in declarations
        .iter()
        .flat_map(|d| &d.entities)
        .flat_map(|kind| &kind.fields)
        .filter(|f| matches!(f.field_type.as_str(), "reference" | "reference_list"))
    {
        assert_eq!(field.default_value, None, "{}", field.name);
        checked += 1;
    }
    assert!(checked > 40, "only {checked} reference fields were found");
}

#[specforge_test_macros::test(
    behavior = "link_entity_references",
    verify = "no builtin kind declares a scope field, so every builtin entity id is its name"
)]
fn no_builtin_kind_is_scoped() {
    let declarations = pinned_builtin_declarations().unwrap();
    let text = serde_json::to_string(&declarations).unwrap();
    assert!(
        !text.contains("scoped_by"),
        "a builtin kind declares a scope"
    );
}
