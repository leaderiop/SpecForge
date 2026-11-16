//! A fact's JSON value read as a field value by its field's form.

use super::*;

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a fact's value is read by its field's form, as a spec file writes it: a word field's word is bare"
)]
fn a_facts_strings_are_read_by_their_fields_form() {
    let fact = Fact::new("test", "js:a#b")
        .field("status", "passed")
        .field("file", "a.test.js")
        .field("names", serde_json::json!(["v1/TR1", "a b"]));
    let text = String::from_utf8(
        Store::new("node", "@t/node", "abc", vec![fact], BTreeMap::new()).bytes(),
    )
    .unwrap();

    let (file, _) = served(KEY, &text, &declared()).unwrap();

    let fields = &file.entities[0].fields;
    assert_eq!(
        fields.get("status"),
        Some(&FieldValue::Identifier("passed".into()))
    );
    assert_eq!(
        fields.get("file"),
        Some(&FieldValue::String("a.test.js".into())),
        "an undeclared field's string stays text"
    );
    let Some(FieldValue::List(names)) = fields.get("names") else {
        panic!("an array of strings is a list");
    };
    let names: Vec<(&str, Written)> = names.iter().map(|n| (n.text.as_str(), n.written)).collect();
    assert_eq!(names, [("v1/TR1", Written::Bare), ("a b", Written::Quoted)]);
}

#[test]
fn a_facts_field_values_become_field_values_by_their_form() {
    let fact = Fact::new("test", "js:a#b")
        .field("file", "a.test.js")
        .field("runs", 3)
        .field("flaky", true)
        .field("tags", serde_json::json!(["slow", "js:a#c"]))
        .field("meta", serde_json::json!({"x": 1}))
        .field("ratio", 0.5);
    let text = String::from_utf8(
        Store::new("node", "@t/node", "abc", vec![fact], BTreeMap::new()).bytes(),
    )
    .unwrap();

    let (file, _) = served(KEY, &text, &FieldForms::default()).unwrap();

    let fields = &file.entities[0].fields;
    assert_eq!(
        fields.get("file"),
        Some(&FieldValue::String("a.test.js".into()))
    );
    assert_eq!(fields.get("runs"), Some(&FieldValue::Integer(3)));
    assert_eq!(fields.get("flaky"), Some(&FieldValue::Boolean(true)));
    let Some(FieldValue::List(tags)) = fields.get("tags") else {
        panic!("an array of strings is a list");
    };
    let tags: Vec<(&str, Written)> = tags.iter().map(|t| (t.text.as_str(), t.written)).collect();
    assert_eq!(
        tags,
        [("slow", Written::Quoted), ("js:a#c", Written::Quoted)]
    );
    assert_eq!(fields.get("meta"), None);
    assert_eq!(
        fields.get("ratio"),
        Some(&FieldValue::Number(Decimal::parse("0.5").unwrap()))
    );
}

#[specforge_test_macros::test(
    behavior = "build_in_memory_graph",
    verify = "a fact's fraction is a number value, and one no decimal holds is its text on a declared field, for E061"
)]
fn a_fraction_is_a_number_value() {
    use specforge_graph::field_form::ValueShape;
    use specforge_protocol_types::FieldType;
    let fact = Fact::new("test", "js:a#b")
        .field("ratio", 0.25)
        .field("big", 1e20)
        .field("drift", 0.1 + 0.2)
        .field("noise", 2.0 / 3.0);
    let text = String::from_utf8(
        Store::new("node", "@t/node", "abc", vec![fact], BTreeMap::new()).bytes(),
    )
    .unwrap();
    let drift = FieldForm::new(FieldType::Number, Vec::new());
    let forms = FieldForms::new(
        HashMap::from([(("test".to_string(), "drift".to_string()), drift.clone())]),
        BTreeSet::new(),
        BTreeSet::from(["test".to_string()]),
    );

    let (file, _) = served(KEY, &text, &forms).unwrap();

    let fields = &file.entities[0].fields;
    let number = |text: &str| Some(FieldValue::Number(Decimal::parse(text).unwrap()));
    assert_eq!(fields.get("ratio").cloned(), number("0.25"));
    assert_eq!(fields.get("big").cloned(), number("1e20"));
    // The declared field keeps the text, so its form refuses it naming the cap; the undeclared one is none.
    let text = "0.30000000000000004";
    assert_eq!(
        fields.get("drift"),
        Some(&FieldValue::Identifier(text.into()))
    );
    assert_eq!(
        drift
            .admit(ValueShape::Identifier, text)
            .unwrap_err()
            .suggestion
            .as_deref(),
        Some("a number keeps at most 15 significant digits")
    );
    assert_eq!(fields.get("noise"), None);
}
