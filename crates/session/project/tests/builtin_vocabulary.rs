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

/// Every JSON object key and string value under `value`.
fn keys_and_types(value: &serde_json::Value, found: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, inner) in map {
                found.push(key.clone());
                if key == "field_type" {
                    found.extend(inner.as_str().map(str::to_string));
                }
                keys_and_types(inner, found);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|i| keys_and_types(i, found)),
        _ => {}
    }
}

#[specforge_test_macros::test(
    behavior = "check_field_value_types",
    verify = "no builtin or pinned example declaration declares a member, a target union, a bound, a file base, an enum list or a block list"
)]
fn no_builtin_field_declares_a_k3_attribute() {
    let mut files = 0;
    for dir in std::fs::read_dir(specforge_component::committed::snapshots()).unwrap() {
        for file in std::fs::read_dir(dir.unwrap().path()).unwrap() {
            let path = file.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            // Fields are declared by entities, shared fields and enhancements; a surface's command
            // arguments have a `minimum` of their own.
            let fields = ["entities", "shared_fields", "enhancements"];
            if !fields.iter().any(|f| name == format!("describe_{f}.json")) {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let mut found = Vec::new();
            keys_and_types(&value, &mut found);
            for word in [
                "members",
                "target_kinds",
                "minimum",
                "maximum",
                "file_base",
                "enum_list",
                "block_list",
            ] {
                assert!(
                    !found.iter().any(|f| f == word),
                    "{} declares '{word}'",
                    path.display()
                );
            }
            files += 1;
        }
    }
    assert!(files > 20, "only {files} declaration files were found");
}

#[specforge_test_macros::test(
    behavior = "judge_field_history",
    verify = "no builtin or pinned example declaration declares a writer, an immutable or append-only flag, a baseline or computes"
)]
fn no_builtin_declares_a_writer_or_history_flag() {
    let mut files = 0;
    for dir in std::fs::read_dir(specforge_component::committed::snapshots()).unwrap() {
        for file in std::fs::read_dir(dir.unwrap().path()).unwrap() {
            let path = file.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let declared = ["entities", "shared_fields", "enhancements", "passes"];
            if !declared
                .iter()
                .any(|f| name == format!("describe_{f}.json"))
            {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let mut found = Vec::new();
            keys_and_types(&value, &mut found);
            for word in ["writer", "immutable", "append_only", "baseline", "computes"] {
                assert!(
                    !found.iter().any(|f| f == word),
                    "{} declares '{word}'",
                    path.display()
                );
            }
            files += 1;
        }
    }
    assert!(files > 20, "only {files} declaration files were found");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "no builtin declares a feature flag or a setting"
)]
fn the_builtins_declare_no_settings() {
    let declarations = pinned_builtin_declarations().unwrap();
    for declaration in &declarations {
        assert!(
            declaration.feature_flags.is_empty(),
            "{}",
            declaration.name()
        );
    }
    let text = serde_json::to_string(&declarations).unwrap();
    assert!(
        !text.contains("\"settings\""),
        "a builtin declares settings"
    );
    assert!(declarations.len() > 5);
}
