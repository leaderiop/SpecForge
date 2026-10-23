use super::*;
use serde_json::json;

#[test]
fn a_schema_declaring_a_target_argument_is_refused() {
    for &name in TARGET_ARGUMENTS {
        for schema in [
            json!({"type": "object", "properties": {name: {"type": "string"}}}),
            json!({"type": "object", "required": [name]}),
        ] {
            assert_eq!(
                refusal(&schema),
                Some(ToolRefusal { argument: name }),
                "{schema}"
            );
            assert_eq!(
                refusal(&schema).map(|r| r.to_string()),
                Some(format!(
                    "its input schema declares '{name}', which the call target takes"
                ))
            );
        }
    }
    for schema in [
        json!({"type": "object"}),
        json!({"type": "object", "properties": {"file": {}}, "required": ["file"]}),
        json!({"type": "object", "properties": {"nested": {"properties": {"path": {}}}}}),
        json!("not an object"),
    ] {
        assert_eq!(refusal(&schema), None, "{schema}");
    }
}
