use super::*;
use specforge_extension_sdk::{ValidatorEntity, ValidatorField, ValidatorRef};

fn context(entity: ValidatorEntity, referenced: Vec<ValidatorRef>) -> ValidatorContext {
    ValidatorContext {
        entity,
        referenced,
        declared_types: vec!["order".to_string()],
        primitives: vec![],
    }
}

fn field(key: &str, value: serde_json::Value) -> ValidatorField {
    ValidatorField {
        key: key.to_string(),
        value,
        annotations: vec![],
    }
}

fn entity(kind: &str, fields: Vec<ValidatorField>) -> ValidatorEntity {
    ValidatorEntity {
        id: kind.to_string(),
        kind: kind.to_string(),
        fields,
        methods: vec![],
    }
}

fn pass() -> ValidatorVerdict {
    ValidatorVerdict::Pass
}

fn fail(field: &str, value: &str) -> ValidatorVerdict {
    ValidatorVerdict::Fail {
        field: Some(field.to_string()),
        value: Some(value.to_string()),
    }
}

#[test]
fn milestone_ranges_fail_on_dangling_behavior() {
    let ctx = context(
        entity(
            "milestone",
            vec![field("behaviors", serde_json::json!("gone"))],
        ),
        vec![ValidatorRef {
            id: "gone".into(),
            kind: None,
        }],
    );
    assert_eq!(
        validate_milestone_behavior_ranges(&ctx),
        fail("behaviors", "gone")
    );
}

#[test]
fn milestone_ranges_pass_when_all_exist() {
    let ctx = context(
        entity(
            "milestone",
            vec![field("behaviors", serde_json::json!("b1"))],
        ),
        vec![ValidatorRef {
            id: "b1".into(),
            kind: Some("behavior".into()),
        }],
    );
    assert_eq!(validate_milestone_behavior_ranges(&ctx), pass());
}

#[test]
fn type_annotations_fail_on_unknown_annotation() {
    let mut f = field("total", serde_json::json!("i64"));
    f.annotations = vec!["readonly".to_string(), "indexed".to_string()];
    let ctx = context(entity("type", vec![f]), vec![]);
    assert_eq!(
        validate_type_field_annotations(&ctx),
        fail("total", "@indexed")
    );
}

#[test]
fn type_annotations_pass_on_known_annotations() {
    let mut f = field("total", serde_json::json!("i64"));
    f.annotations = vec![
        "readonly".to_string(),
        "unique".to_string(),
        "optional".to_string(),
        "literal".to_string(),
    ];
    let ctx = context(entity("type", vec![f]), vec![]);
    assert_eq!(validate_type_field_annotations(&ctx), pass());
}

#[test]
fn verdict_wire_shape_is_pinned() {
    assert_eq!(
        serde_json::to_value(pass()).unwrap(),
        serde_json::json!({ "verdict": "pass" })
    );
    assert_eq!(
        serde_json::to_value(fail("triggers", "missing_b")).unwrap(),
        serde_json::json!({ "verdict": "fail", "field": "triggers", "value": "missing_b" })
    );
    let sparse = ValidatorVerdict::Fail {
        field: None,
        value: None,
    };
    assert_eq!(
        serde_json::to_value(sparse).unwrap(),
        serde_json::json!({ "verdict": "fail" })
    );
    let round: ValidatorVerdict =
        serde_json::from_value(serde_json::json!({ "verdict": "fail", "field": "f" })).unwrap();
    assert_eq!(
        round,
        ValidatorVerdict::Fail {
            field: Some("f".into()),
            value: None,
        }
    );
}

#[test]
fn ref_ids_accept_joined_string_and_array_forms() {
    assert_eq!(
        ref_ids(&serde_json::json!("a, b,c")),
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    assert_eq!(
        ref_ids(&serde_json::json!(["x", "y"])),
        vec!["x".to_string(), "y".to_string()]
    );
    assert!(ref_ids(&serde_json::json!(7)).is_empty());
}
