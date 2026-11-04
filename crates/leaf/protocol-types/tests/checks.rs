//! Value checks on the wire (ADR 0336): a checker's declaration and a field's checks read back as written, and the
//! `checkers` describe category is read with the rest.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

#[test]
fn a_checker_and_a_field_check_round_trip() {
    use specforge_protocol_types::{CheckRef, CheckerDescriptor, FieldDescriptor, ValueForm};
    let checker: CheckerDescriptor = serde_json::from_value(serde_json::json!({
        "name": "file",
        "applies_to": ["string", "string_list"],
        "args": [{"name": "root", "field_type": "string"}],
        "scopes": ["files"],
        "codes": ["E092"],
        "value": "path",
        "root_arg": "root"
    }))
    .unwrap();
    assert_eq!(checker.value, ValueForm::Path);
    let again = serde_json::to_value(&checker).unwrap();
    assert_eq!(
        serde_json::from_value::<CheckerDescriptor>(again).unwrap(),
        checker
    );
    let text: CheckerDescriptor = serde_json::from_value(serde_json::json!({"name": "t"})).unwrap();
    assert_eq!(text.value, ValueForm::Text);
    assert_eq!(
        serde_json::to_value(&text).unwrap(),
        serde_json::json!({"name": "t", "applies_to": []})
    );

    let field = FieldDescriptor {
        name: "docs".into(),
        checks: vec![
            CheckRef::named("a"),
            CheckRef::AnyOf {
                any_of: vec![CheckRef::named("b"), CheckRef::named("c")],
            },
        ],
        ..Default::default()
    };
    let wire = serde_json::to_value(&field).unwrap();
    assert_eq!(
        serde_json::from_value::<FieldDescriptor>(wire).unwrap(),
        field
    );
}

#[test]
fn checkers_are_the_last_describe_category_and_an_unknown_key_in_one_is_reported() {
    use specforge_protocol_types::{DeclaredCategory, DescribeResponse, ExtensionDeclaration};
    assert_eq!(
        DeclaredCategory::ALL.last(),
        Some(&DeclaredCategory::Checkers)
    );
    let mut unknown = Vec::new();
    let handshake = specforge_protocol_types::HandshakeResponse {
        protocol_version: specforge_protocol_types::PROTOCOL_VERSION.to_string(),
        ..Default::default()
    };
    let declaration = ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            let items = match category {
                DeclaredCategory::Checkers => {
                    serde_json::json!([{"name": "a", "aplies_to": [], "args": [{"name": "x", "field_type": "string", "oops": 1}]}])
                }
                _ => serde_json::json!([]),
            };
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items,
            })
        },
        |key| unknown.push(key.key),
    )
    .unwrap();
    assert_eq!(declaration.checkers[0].name, "a");
    assert_eq!(unknown, ["aplies_to", "oops"]);
}
