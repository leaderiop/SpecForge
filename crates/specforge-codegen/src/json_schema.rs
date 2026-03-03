use crate::checksum::prepend_checksum;
use crate::generator::{GenerateContext, GenerateResult, Generator};
use crate::naming;
use specforge_common::{EntityKind, FieldMap, FieldValue};
use specforge_emitter::GeneratedFile;
use serde_json::{Map, Value, json};

pub struct JsonSchemaGenerator;

impl Generator for JsonSchemaGenerator {
    fn name(&self) -> &str {
        "json-schema"
    }

    fn generate(&self, ctx: &GenerateContext) -> GenerateResult {
        let mut files = Vec::new();

        let entity_fields = build_entity_fields(ctx);

        let type_nodes = ctx.graph.nodes_of_kind(EntityKind::TypeDef);
        for node in &type_nodes {
            let raw_id = node.id.raw();
            if let Some(fields) = entity_fields.get(raw_id) {
                let schema = generate_schema(raw_id, fields);
                let content = serde_json::to_string_pretty(&schema).expect("JSON serialization failed");
                let file_name = format!("{}.schema.json", naming::to_file_name(raw_id));
                files.push(GeneratedFile {
                    path: file_name,
                    content: prepend_checksum(&format!("{content}\n")),
                });
            }
        }

        GenerateResult {
            files,
            warnings: Vec::new(),
            errors: Vec::new(),
            entity_checksums: std::collections::HashMap::new(),
        }
    }
}

fn build_entity_fields<'a>(ctx: &'a GenerateContext) -> std::collections::HashMap<&'a str, &'a FieldMap> {
    let mut map = std::collections::HashMap::new();
    for file in ctx.files {
        for entity in &file.entities {
            map.insert(entity.id.raw(), &entity.fields);
        }
    }
    map
}

fn generate_schema(name: &str, fields: &FieldMap) -> Value {
    // Union type
    if let Some(FieldValue::StringList(variants)) = fields.get("variants") {
        return json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": name,
            "enum": variants,
        });
    }

    // Struct type
    if let Some(FieldValue::Block(struct_fields)) = fields.get("fields") {
        let mut properties = Map::new();
        let mut required = Vec::new();

        for (field_name, field_value) in struct_fields.iter() {
            if let FieldValue::Block(entry) = field_value {
                let type_str = match entry.get("type") {
                    Some(FieldValue::String(t)) => t.as_str(),
                    _ => "string",
                };

                let annotations = match entry.get("annotations") {
                    Some(FieldValue::StringList(anns)) => anns.clone(),
                    _ => Vec::new(),
                };

                let is_optional = annotations.iter().any(|a| a == "@optional");
                let is_readonly = annotations.iter().any(|a| a == "@readonly");
                let is_unique = annotations.iter().any(|a| a == "@unique");

                let mut prop = map_type_to_schema(type_str);

                if is_readonly {
                    prop.as_object_mut()
                        .unwrap()
                        .insert("readOnly".to_string(), Value::Bool(true));
                }
                if is_unique {
                    if type_str.ends_with("[]") {
                        prop.as_object_mut()
                            .unwrap()
                            .insert("uniqueItems".to_string(), Value::Bool(true));
                    } else {
                        prop.as_object_mut()
                            .unwrap()
                            .insert("x-specforge-unique".to_string(), Value::Bool(true));
                    }
                }

                if !is_optional {
                    required.push(Value::String(field_name.to_string()));
                }
                properties.insert(field_name.to_string(), prop);
            }
        }

        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": name,
            "type": "object",
            "properties": Value::Object(properties),
        });
        if !required.is_empty() {
            schema.as_object_mut()
                .unwrap()
                .insert("required".to_string(), Value::Array(required));
        }
        return schema;
    }

    // Fallback: empty object
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": name,
        "type": "object",
    })
}

fn map_type_to_schema(type_expr: &str) -> Value {
    let type_expr = type_expr.trim();
    match type_expr {
        "string" => json!({"type": "string"}),
        "integer" | "int" => json!({"type": "integer"}),
        "float" | "number" => json!({"type": "number"}),
        "bool" | "boolean" => json!({"type": "boolean"}),
        _ => {
            if let Some(inner) = type_expr.strip_suffix("[]") {
                let items = map_type_to_schema(inner);
                return json!({"type": "array", "items": items});
            }
            if let Some(inner) = type_expr.strip_suffix('?') {
                let mut base = map_type_to_schema(inner);
                // Make nullable by adding oneOf with null
                if let Some(obj) = base.as_object_mut() {
                    if let Some(t) = obj.remove("type") {
                        return json!({"oneOf": [{"type": t}, {"type": "null"}]});
                    }
                }
                return base;
            }
            // Named type reference
            json!({"$ref": format!("#/$defs/{type_expr}")})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_struct_type() {
        let mut struct_fields = FieldMap::new();

        let mut name_entry = FieldMap::new();
        name_entry.insert("type", FieldValue::String("string".to_string()));
        struct_fields.insert("name", FieldValue::Block(name_entry));

        let mut age_entry = FieldMap::new();
        age_entry.insert("type", FieldValue::String("integer".to_string()));
        age_entry.insert("annotations", FieldValue::StringList(vec!["@optional".to_string()]));
        struct_fields.insert("age", FieldValue::Block(age_entry));

        let mut fields = FieldMap::new();
        fields.insert("fields", FieldValue::Block(struct_fields));

        let schema = generate_schema("User", &fields);
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["title"], "User");
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["properties"]["age"]["type"], "integer");
        // Only "name" should be required (age is @optional)
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "name");
    }

    #[test]
    fn schema_union_type() {
        let mut fields = FieldMap::new();
        fields.insert(
            "variants",
            FieldValue::StringList(vec!["active".to_string(), "inactive".to_string()]),
        );

        let schema = generate_schema("Status", &fields);
        assert_eq!(schema["title"], "Status");
        let enum_vals = schema["enum"].as_array().unwrap();
        assert_eq!(enum_vals.len(), 2);
    }

    #[test]
    fn schema_readonly_field() {
        let mut struct_fields = FieldMap::new();
        let mut id_entry = FieldMap::new();
        id_entry.insert("type", FieldValue::String("string".to_string()));
        id_entry.insert("annotations", FieldValue::StringList(vec!["@readonly".to_string()]));
        struct_fields.insert("id", FieldValue::Block(id_entry));

        let mut fields = FieldMap::new();
        fields.insert("fields", FieldValue::Block(struct_fields));

        let schema = generate_schema("Entity", &fields);
        assert_eq!(schema["properties"]["id"]["readOnly"], true);
    }

    #[test]
    fn schema_unique_scalar() {
        let mut struct_fields = FieldMap::new();
        let mut email_entry = FieldMap::new();
        email_entry.insert("type", FieldValue::String("string".to_string()));
        email_entry.insert("annotations", FieldValue::StringList(vec!["@unique".to_string()]));
        struct_fields.insert("email", FieldValue::Block(email_entry));

        let mut fields = FieldMap::new();
        fields.insert("fields", FieldValue::Block(struct_fields));

        let schema = generate_schema("User", &fields);
        assert_eq!(schema["properties"]["email"]["x-specforge-unique"], true);
    }

    #[test]
    fn schema_unique_array() {
        let mut struct_fields = FieldMap::new();
        let mut tags_entry = FieldMap::new();
        tags_entry.insert("type", FieldValue::String("string[]".to_string()));
        tags_entry.insert("annotations", FieldValue::StringList(vec!["@unique".to_string()]));
        struct_fields.insert("tags", FieldValue::Block(tags_entry));

        let mut fields = FieldMap::new();
        fields.insert("fields", FieldValue::Block(struct_fields));

        let schema = generate_schema("Post", &fields);
        assert_eq!(schema["properties"]["tags"]["uniqueItems"], true);
    }

    #[test]
    fn map_primitive_types() {
        assert_eq!(map_type_to_schema("string"), json!({"type": "string"}));
        assert_eq!(map_type_to_schema("integer"), json!({"type": "integer"}));
        assert_eq!(map_type_to_schema("boolean"), json!({"type": "boolean"}));
    }

    #[test]
    fn map_array_type() {
        assert_eq!(
            map_type_to_schema("string[]"),
            json!({"type": "array", "items": {"type": "string"}})
        );
    }

    #[test]
    fn map_named_ref() {
        assert_eq!(
            map_type_to_schema("Address"),
            json!({"$ref": "#/$defs/Address"})
        );
    }
}
