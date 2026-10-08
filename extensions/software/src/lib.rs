//! @software — the software vocabulary, authored with the extension SDK.
//!
//! Its kinds, edges, enhancements and rules are declared with the SDK
//! builders in [`declaration`]; the handshake is derived by the SDK from the
//! extension metadata and the contributions. It also hosts the four
//! `validate__*` custom-rule exports (E004/E006/E010/W010).

mod declaration;

use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@specforge/software",
    version = "1.0.0",
    description = "Software design: behaviors, invariants, events, types and ports, and the checks that keep them consistent"
)]
struct Software;

impl Contributions for Software {
    fn contribute(c: &mut ContributionsBuilder) {
        // Optional: product provides the kinds behind the feature, module
        // and milestone links; without it those links are inert (I004).
        // Diagrams (`model`, `outline`) draw the extension in this colour.
        c.theme_color("#4a90d9");
        c.meta.peer_dependencies.push(PeerDependency {
            name: "@specforge/product".to_string(),
            version: "^1.0".to_string(),
            optional: true,
        });
        c.meta.sandbox_policy = Some(SandboxPolicy {
            max_memory_mb: Some(256),
            max_execution_ms: Some(5000),
        });
        // `specforge init` writes this as the starter spec of a project that
        // enables software.
        c.starter_template(include_str!("starter.spec"));

        declaration::declare(c);
    }
}

// ── Custom validators (`check: "custom"` rules) ────────────────────────────
// Each `validate__<rule>` export is declared with its rule (`declaration`):
// it receives the protocol's `ValidatorContext` and answers a
// `ValidatorVerdict` (ADR 0013). The host precomputes everything the
// validator needs, so each is a pure function of the context.

use specforge_extension_sdk::{ValidatorContext, ValidatorVerdict};

/// Type names accepted by E004 without a declared `type` entity. Mirrors
/// the host's `PRIMITIVE_TYPES`; the host also sends its copy in
/// `context.primitives`, so the union keeps the guest correct even for a
/// host that ships an empty list.
const PRIMITIVE_TYPES: &[&str] = &[
    "string",
    "void",
    "bool",
    "i8",
    "i16",
    "i32",
    "i64",
    "u8",
    "u16",
    "u32",
    "u64",
    "f32",
    "f64",
    "usize",
    "isize",
    "any",
    // the portable primitives docs/entities/type.md documents, and `never`
    // for an impossible error channel (docs/entities/port.md)
    "number",
    "integer",
    "boolean",
    "timestamp",
    "never",
    // stdlib containers: their type arguments are checked recursively
    "Result",
    "Option",
    "Vec",
    "Box",
    "Arc",
    "Rc",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "BTreeSet",
    "String",
];

/// `Result<A, B>` -> `A, B`; `string[]` -> `string`; whitespace trimmed.
fn base_type_names(ty: &str) -> Vec<String> {
    // Generics, arrays, the unit type `()` and function types
    // `fn(A) -> B` reduce to the named types inside them.
    ty.chars()
        .map(|c| {
            if matches!(c, '<' | '>' | '[' | ']' | '(' | ')' | '-') {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty() && *s != "fn")
        .map(str::to_string)
        .collect()
}

/// Reference IDs declared by a field value. The wire value of a
/// reference-list field is either the comma-joined ID string (the host's
/// `join(", ")` stringification) or an array of IDs.
fn ref_ids(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => s
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn resolved_kind<'a>(context: &'a ValidatorContext, id: &str) -> Option<&'a str> {
    context
        .referenced
        .iter()
        .find(|r| r.id == id)
        .and_then(|r| r.kind.as_deref())
}

/// E006: every event trigger must reference an existing behavior.
fn validate_event_triggers(context: &ValidatorContext) -> ValidatorVerdict {
    for field in &context.entity.fields {
        if field.key != "triggers" {
            continue;
        }
        for id in ref_ids(&field.value) {
            match resolved_kind(context, &id) {
                // The ID names no node in the graph (dangling reference).
                None => {
                    return ValidatorVerdict::Fail {
                        field: Some("triggers".into()),
                        value: Some(id),
                    };
                }
                // The target exists but is not a behavior.
                Some(kind) if kind != "behavior" => {
                    return ValidatorVerdict::Fail {
                        field: Some("triggers".into()),
                        value: Some(id),
                    };
                }
                _ => {}
            }
        }
    }
    ValidatorVerdict::Pass
}

/// E010: milestone behavior references must exist.
fn validate_milestone_behavior_ranges(context: &ValidatorContext) -> ValidatorVerdict {
    for field in &context.entity.fields {
        if field.key != "behaviors" {
            continue;
        }
        for id in ref_ids(&field.value) {
            if resolved_kind(context, &id).is_none() {
                return ValidatorVerdict::Fail {
                    field: Some("behaviors".into()),
                    value: Some(id),
                };
            }
        }
    }
    ValidatorVerdict::Pass
}

/// W010: type fields may only carry known annotations.
fn validate_type_field_annotations(context: &ValidatorContext) -> ValidatorVerdict {
    const KNOWN: &[&str] = &["readonly", "unique", "optional", "literal"];
    for field in &context.entity.fields {
        for ann in &field.annotations {
            if !KNOWN.contains(&ann.as_str()) {
                return ValidatorVerdict::Fail {
                    field: Some(field.key.clone()),
                    value: Some(format!("@{ann}")),
                };
            }
        }
    }
    ValidatorVerdict::Pass
}

/// E004: port method param/return types must be primitives or declared types.
fn validate_port_methods(context: &ValidatorContext) -> ValidatorVerdict {
    let known_type = |name: &str| -> bool {
        PRIMITIVE_TYPES.contains(&name)
            || context.primitives.iter().any(|p| p == name)
            || context.declared_types.iter().any(|d| d == name)
    };
    for method in &context.entity.methods {
        let mut type_refs: Vec<String> = Vec::new();
        for p in &method.params {
            type_refs.extend(base_type_names(&p.ty));
        }
        if let Some(ret) = &method.returns {
            type_refs.extend(base_type_names(ret));
        }
        for t in type_refs {
            if !known_type(&t) {
                return ValidatorVerdict::Fail {
                    field: Some(method.name.clone()),
                    value: Some(t),
                };
            }
        }
    }
    ValidatorVerdict::Pass
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod validator_tests {
    use super::*;
    use specforge_extension_sdk::{
        ValidatorEntity, ValidatorField, ValidatorMethod, ValidatorParam, ValidatorRef,
    };

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
    fn base_type_names_reach_inside_every_type_form() {
        assert_eq!(
            base_type_names("Result<(), string>"),
            vec!["Result", "string"]
        );
        assert_eq!(base_type_names("float[][]"), vec!["float"]);
        assert_eq!(
            base_type_names("fn(string, i32) -> bool"),
            vec!["string", "i32", "bool"]
        );
        assert!(base_type_names("fn()").is_empty());
    }

    #[test]
    fn event_triggers_pass_on_valid_behavior_refs() {
        let ctx = context(
            entity(
                "event",
                vec![field("triggers", serde_json::json!("b1, b2"))],
            ),
            vec![
                ValidatorRef {
                    id: "b1".into(),
                    kind: Some("behavior".into()),
                },
                ValidatorRef {
                    id: "b2".into(),
                    kind: Some("behavior".into()),
                },
            ],
        );
        assert_eq!(validate_event_triggers(&ctx), pass());
    }

    #[test]
    fn event_triggers_fail_on_dangling_ref() {
        let ctx = context(
            entity(
                "event",
                vec![field("triggers", serde_json::json!("missing_b"))],
            ),
            vec![],
        );
        assert_eq!(validate_event_triggers(&ctx), fail("triggers", "missing_b"));
    }

    #[test]
    fn event_triggers_fail_on_non_behavior_target() {
        let ctx = context(
            entity(
                "event",
                vec![field(
                    "triggers",
                    serde_json::json!(["ok_b", "not_a_behavior"]),
                )],
            ),
            vec![
                ValidatorRef {
                    id: "ok_b".into(),
                    kind: Some("behavior".into()),
                },
                ValidatorRef {
                    id: "not_a_behavior".into(),
                    kind: Some("type".into()),
                },
            ],
        );
        assert_eq!(
            validate_event_triggers(&ctx),
            fail("triggers", "not_a_behavior")
        );
    }

    #[test]
    fn event_triggers_ignores_other_fields() {
        let ctx = context(
            entity(
                "event",
                vec![field("payload", serde_json::json!("some_type"))],
            ),
            vec![ValidatorRef {
                id: "some_type".into(),
                kind: Some("type".into()),
            }],
        );
        assert_eq!(validate_event_triggers(&ctx), pass());
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

    fn port(methods: Vec<ValidatorMethod>) -> ValidatorContext {
        context(
            ValidatorEntity {
                id: "order_port".to_string(),
                kind: "port".to_string(),
                fields: vec![],
                methods,
            },
            vec![],
        )
    }

    #[test]
    fn port_methods_fail_on_undeclared_type() {
        let ctx = port(vec![ValidatorMethod {
            name: "submit".to_string(),
            params: vec![ValidatorParam {
                name: "order".to_string(),
                ty: "Undeclared".to_string(),
            }],
            returns: None,
        }]);
        assert_eq!(validate_port_methods(&ctx), fail("submit", "Undeclared"));
    }

    #[test]
    fn port_methods_accept_generics_arrays_and_declared_types() {
        let ctx = port(vec![ValidatorMethod {
            name: "find".to_string(),
            params: vec![
                ValidatorParam {
                    name: "q".to_string(),
                    ty: "String".to_string(),
                },
                ValidatorParam {
                    name: "ids".to_string(),
                    ty: "Vec<string>[]".to_string(),
                },
            ],
            returns: Some("Result<order, string>".to_string()),
        }]);
        assert_eq!(validate_port_methods(&ctx), pass());
    }

    #[test]
    fn port_methods_accept_documented_primitives_without_host_list() {
        let ctx = port(vec![ValidatorMethod {
            name: "stat".to_string(),
            params: vec![ValidatorParam {
                name: "ratio".to_string(),
                ty: "number".to_string(),
            }],
            returns: Some("Result<(integer, boolean, timestamp), never>".to_string()),
        }]);
        assert_eq!(validate_port_methods(&ctx), pass());
    }

    #[test]
    fn port_methods_fail_on_generic_argument_of_known_container() {
        let ctx = port(vec![ValidatorMethod {
            name: "load".to_string(),
            params: vec![],
            returns: Some("Option<Undeclared>".to_string()),
        }]);
        assert_eq!(validate_port_methods(&ctx), fail("load", "Undeclared"));
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
}
