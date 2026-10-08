//! The extension declaration: one type from the SDK to the registry build,
//! assembled from and served as the protocol's wire answers.

use serde_json::json;
use specforge_protocol_types::{
    CommandArgDescriptor, CommandArgType, CommandDescriptor, CompilerPassDescriptor,
    DECLARED_CATEGORIES, DeclaredCategory, DescribeResponse, EdgeTypeDescriptor,
    EntityEnhancementDescriptor, EntityKindDescriptor, ExtensionDeclaration, FeatureFlagDescriptor,
    FieldDescriptor, HandshakeResponse, PeerDependency, ProtocolError, SUPPORTED_CATEGORIES,
    SurfaceDescriptor, UnknownKey, ValidationRuleDescriptor, ValidationSeverity,
};

fn field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.to_string(),
        field_type: "string".to_string(),
        ..Default::default()
    }
}

fn kind(name: &str, fields: &[&str]) -> EntityKindDescriptor {
    EntityKindDescriptor {
        name: name.to_string(),
        fields: fields.iter().map(|f| field(f)).collect(),
        verify_kinds: vec!["unit".to_string()],
        ..Default::default()
    }
}

/// A declaration with something in every category.
fn full() -> ExtensionDeclaration {
    ExtensionDeclaration {
        handshake: HandshakeResponse {
            protocol_version: "1.0.0".to_string(),
            name: "@acme/reports".to_string(),
            version: "0.1.0".to_string(),
            peer_dependencies: vec![PeerDependency {
                name: "@acme/base".to_string(),
                version: "^1".to_string(),
                optional: false,
            }],
            ext_short: Some("rep".to_string()),
            description: Some("Reports".to_string()),
            keywords: vec!["reports".to_string()],
            ..Default::default()
        },
        entities: vec![
            kind("report", &["title", "owner"]),
            kind("chart", &["axis"]),
        ],
        edges: vec![EdgeTypeDescriptor {
            label: "charts".to_string(),
            source_kind: Some("report".to_string()),
            target_kind: Some("chart".to_string()),
            ..Default::default()
        }],
        shared_fields: vec![field("tags")],
        enhancements: vec![EntityEnhancementDescriptor {
            target_kind: "behavior".to_string(),
            source_extension: "@acme/base".to_string(),
            fields: vec![field("report")],
            ..Default::default()
        }],
        validation_rules: vec![ValidationRuleDescriptor {
            code: "R001".to_string(),
            severity: ValidationSeverity::Warning,
            message_template: "report '{id}' has no title".to_string(),
            check: "missing_required_field".to_string(),
            target_kind: Some("report".to_string()),
            field: Some("title".to_string()),
            ..Default::default()
        }],
        surfaces: SurfaceDescriptor {
            commands: vec![CommandDescriptor {
                id: "list".to_string(),
                title: "List".to_string(),
                description: "List reports".to_string(),
                category: None,
                export: "cmd__list".to_string(),
                args: vec![CommandArgDescriptor {
                    name: "limit".to_string(),
                    arg_type: CommandArgType::Integer,
                    required: false,
                    default_value: Some("10".to_string()),
                    description: None,
                    minimum: Some(0),
                }],
            }],
            ..Default::default()
        },
        collectors: Vec::new(),
        analyzers: Vec::new(),
        passes: vec![CompilerPassDescriptor {
            name: "audit".to_string(),
            phase: Some("check".to_string()),
            ..Default::default()
        }],
        feature_flags: vec![FeatureFlagDescriptor {
            name: "beta".to_string(),
            description: None,
            default_enabled: false,
        }],
    }
}

/// The wire answer `declaration` gives for `category`, as a guest serves it.
fn answer(
    declaration: &ExtensionDeclaration,
    category: &str,
) -> Result<DescribeResponse, ProtocolError> {
    let items = declaration
        .describe_items(category)
        .ok_or_else(|| ProtocolError::UnsupportedCategory(category.to_string()))?;
    // Through the bytes, as the host receives them.
    let bytes = serde_json::to_vec(&DescribeResponse {
        category: category.to_string(),
        items,
    })
    .unwrap();
    Ok(serde_json::from_slice(&bytes).unwrap())
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a declaration round-trips through its wire answers unchanged"
)]
fn a_declaration_round_trips_through_its_wire_answers() {
    let declared = full();
    let mut asked = Vec::new();
    let loaded = ExtensionDeclaration::from_wire(
        declared.handshake.clone(),
        |category| {
            asked.push(category.to_string());
            answer(&declared, category)
        },
        |key| panic!("unexpected unknown key {key:?}"),
    )
    .unwrap();
    assert_eq!(loaded, declared);
    assert_eq!(asked, DECLARED_CATEGORIES);
    // Every supported category has an answer, the reserved ones empty.
    for category in SUPPORTED_CATEGORIES {
        assert!(declared.describe_items(category).is_some(), "{category}");
    }
    assert_eq!(declared.describe_items("grammars"), Some(json!([])));
    assert_eq!(declared.describe_items("nope"), None);
    // So does an empty one: no surfaces is no item at all.
    let empty = ExtensionDeclaration::default();
    assert_eq!(empty.describe_items("surfaces"), Some(json!([])));
    let loaded = ExtensionDeclaration::from_wire(
        empty.handshake.clone(),
        |category| answer(&empty, category),
        |_| {},
    )
    .unwrap();
    assert_eq!(loaded, empty);
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a describe category that does not parse fails the load naming the category"
)]
fn a_category_that_does_not_parse_fails_naming_it() {
    let declared = full();
    let error = ExtensionDeclaration::from_wire(
        declared.handshake.clone(),
        |category| match category {
            "passes" => Ok(DescribeResponse {
                category: category.to_string(),
                items: json!([{ "nam": "x" }]),
            }),
            other => answer(&declared, other),
        },
        |_| {},
    )
    .unwrap_err();
    match &error {
        ProtocolError::DescribeFailed { category, reason } => {
            assert_eq!(category, "passes");
            assert!(reason.contains("missing field `name`"), "{reason}");
        }
        other => panic!("expected DescribeFailed, got {other:?}"),
    }
    assert!(
        error.to_string().starts_with("describe 'passes' failed:"),
        "{error}"
    );

    // A transport failure of one category is the load's failure too.
    let error = ExtensionDeclaration::from_wire(
        declared.handshake.clone(),
        |category| match category {
            "surfaces" => Err(ProtocolError::DescribeFailed {
                category: category.to_string(),
                reason: "guest_error: boom".to_string(),
            }),
            other => answer(&declared, other),
        },
        |_| {},
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("describe 'surfaces' failed"),
        "{error}"
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a describe item key the protocol does not define produces W138"
)]
fn an_unknown_item_key_is_reported() {
    let declared = full();
    let mut unknown = Vec::new();
    let loaded = ExtensionDeclaration::from_wire(
        declared.handshake.clone(),
        |category| match category {
            "entities" => Ok(DescribeResponse {
                category: category.to_string(),
                items: json!([{
                    "name": "x",
                    "testabel": true,
                    "contract_target": false,
                    "fields": [{ "name": "f", "field_type": "string", "requird": true }],
                }]),
            }),
            "surfaces" => Ok(DescribeResponse {
                category: category.to_string(),
                items: json!([{
                    "commands": [{
                        "id": "list", "title": "List", "description": "d", "export": "cmd__list",
                        "args": [{ "name": "n", "arg_type": "integer", "minimun": 1 }],
                        "sandbox": { "fs_read": true, "fs_raed": true }
                    }]
                }]),
            }),
            other => answer(&declared, other),
        },
        |key| unknown.push(key),
    )
    .unwrap();
    // The typo costs the field it meant, nothing else.
    assert!(!loaded.entities[0].testable);
    assert_eq!(
        unknown,
        [
            UnknownKey {
                category: "entities",
                item: "x".to_string(),
                key: "testabel".to_string(),
            },
            UnknownKey {
                category: "entities",
                item: "x.fields[f]".to_string(),
                key: "requird".to_string(),
            },
            UnknownKey {
                category: "surfaces",
                item: "#0.commands[list]".to_string(),
                key: "sandbox".to_string(),
            },
            UnknownKey {
                category: "surfaces",
                item: "#0.commands[list].args[n]".to_string(),
                key: "minimun".to_string(),
            },
        ]
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "the fields category is every kind's fields, concatenated"
)]
fn the_fields_category_is_every_kinds_fields() {
    let declared = full();
    let fields = declared.describe_items("fields").unwrap();
    let names: Vec<&str> = fields
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["title", "owner", "axis"]);
    // Shared fields are their own category, never part of `fields`.
    assert!(!names.contains(&"tags"));
    assert!(!DECLARED_CATEGORIES.contains(&"fields"));
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "an absent short is the name's last segment"
)]
fn an_absent_short_is_the_names_last_segment() {
    let mut declaration = full();
    assert_eq!(declaration.short(), "rep");
    declaration.handshake.ext_short = None;
    assert_eq!(declaration.short(), "reports");
    declaration.handshake.name = "solo".to_string();
    assert_eq!(declaration.short(), "solo");
    declaration.handshake.name = "@solo".to_string();
    assert_eq!(declaration.short(), "solo");
}

/// Identity, peers and verify kinds read through the declaration; the
/// content decides its contribution flags, the handshake only the others.
#[test]
fn a_declaration_reads_its_identity_and_derives_its_flags() {
    let mut declaration = full();
    assert_eq!(declaration.name(), "@acme/reports");
    assert_eq!(declaration.version(), "0.1.0");
    assert_eq!(declaration.peers()[0].name, "@acme/base");
    assert_eq!(declaration.verify_kinds(), ["unit"]);

    declaration.handshake.contribution_flags.entities = false;
    declaration.handshake.contribution_flags.providers = true;
    let flags = declaration.contribution_flags();
    assert!(flags.entities && flags.validators && !flags.collectors && !flags.analyzers);
    assert!(flags.providers);
    declaration.handshake.contribution_flags.collectors = true;
    assert!(!declaration.contribution_flags().collectors);
}

/// Absent optional handshake fields are absent on the wire, so a
/// handshake without them is byte-identical to before they existed.
#[test]
fn absent_handshake_metadata_is_not_serialized() {
    let handshake = HandshakeResponse {
        protocol_version: "1.0.0".to_string(),
        name: "@a/b".to_string(),
        version: "1.0.0".to_string(),
        ..Default::default()
    };
    let wire = serde_json::to_value(&handshake).unwrap();
    for key in ["ext_short", "description", "keywords"] {
        assert!(wire.get(key).is_none(), "{key} in {wire}");
    }
    let parsed: HandshakeResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(parsed, handshake);
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "the loader reads the handshake and every describe category once"
)]
fn every_declared_category_is_a_supported_one() {
    let names: Vec<&str> = DeclaredCategory::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(DECLARED_CATEGORIES, names.as_slice());
    for category in DeclaredCategory::ALL {
        assert_eq!(DeclaredCategory::from_name(category.name()), Some(category));
    }
    // The wire list holds every declared name, and only the categories the
    // host never reads besides.
    let mut others: Vec<&str> = SUPPORTED_CATEGORIES
        .iter()
        .copied()
        .filter(|name| !names.contains(name))
        .collect();
    others.sort_unstable();
    assert_eq!(others, ["body_parsers", "fields", "grammars"]);
    for other in others {
        assert_eq!(DeclaredCategory::from_name(other), None);
    }
    assert_eq!(DeclaredCategory::from_name("nope"), None);
}
