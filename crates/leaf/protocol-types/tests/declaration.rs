//! The extension declaration: one type from the SDK to the registry build,
//! assembled from and served as the protocol's wire answers.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::json;
use specforge_protocol_types::{
    CodeDescriptor, CommandArgDescriptor, CommandArgType, CommandDescriptor,
    CompilerPassDescriptor, DECLARED_CATEGORIES, DeclaredCategory, DescribeResponse,
    EdgeTypeDescriptor, EntityEnhancementDescriptor, EntityKindDescriptor, ExtensionDeclaration,
    FeatureFlagDescriptor, FieldDescriptor, HandshakeResponse, PROTOCOL_VERSION, PeerDependency,
    ProtocolError, SettingDescriptor, SettingsItem, SurfaceDescriptor, UnknownKey,
    ValidationRuleDescriptor, ValidationSeverity,
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
            protocol_version: PROTOCOL_VERSION.to_string(),
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
                writes: false,
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
        codes: vec![
            CodeDescriptor {
                code: "R001".to_string(),
                title: "Report without a title".to_string(),
                explanation: "Give the report a title.".to_string(),
                docs: Some("https://acme.dev/codes#r001".to_string()),
            },
            CodeDescriptor {
                code: "R002".to_string(),
                title: "Chart without an axis".to_string(),
                explanation: "Name the chart's axis.".to_string(),
                docs: None,
            },
        ],
        settings: vec![SettingsItem::Setting(SettingDescriptor {
            name: "verbose".to_string(),
            field_type: "bool".to_string(),
            ..Default::default()
        })],
    }
}

/// The wire answer `declaration` gives for `category`, as a guest serves it.
fn answer(
    declaration: &ExtensionDeclaration,
    category: DeclaredCategory,
) -> Result<DescribeResponse, ProtocolError> {
    // Through the bytes, as the host receives them.
    let bytes = serde_json::to_vec(&DescribeResponse {
        category: category.name().to_string(),
        items: declaration.describe_items(category),
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
            asked.push(category.name().to_string());
            answer(&declared, category)
        },
        |key| panic!("unexpected unknown key {key:?}"),
    )
    .unwrap();
    assert_eq!(loaded, declared);
    assert_eq!(asked, DECLARED_CATEGORIES);
    // An empty declaration answers too: no surfaces is no item at all.
    let empty = ExtensionDeclaration::default();
    assert_eq!(empty.describe_items(DeclaredCategory::Surfaces), json!([]));
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
            DeclaredCategory::Passes => Ok(DescribeResponse {
                category: category.name().to_string(),
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
            DeclaredCategory::Surfaces => Err(ProtocolError::DescribeFailed {
                category: category.name().to_string(),
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

/// A guest built before ADR 0286 still sends a rule's `description`: the key is ignored and reported (W138).
#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a describe item key the protocol does not define produces W138"
)]
fn a_rule_description_a_guest_still_sends_is_reported() {
    let declared = full();
    let mut unknown = Vec::new();
    let loaded = ExtensionDeclaration::from_wire(
        declared.handshake.clone(),
        |category| match category {
            DeclaredCategory::ValidationRules => Ok(DescribeResponse {
                category: category.name().to_string(),
                items: json!([{
                    "code": "W901",
                    "severity": "warning",
                    "description": "The old rule description.",
                    "message_template": "m",
                    "check": "no_edges",
                }]),
            }),
            other => answer(&declared, other),
        },
        |key| unknown.push(key),
    )
    .unwrap();
    assert_eq!(loaded.validation_rules[0].code, "W901");
    assert_eq!(
        unknown,
        [UnknownKey {
            category: "validation_rules",
            item: "W901".to_string(),
            key: "description".to_string(),
        }]
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
            DeclaredCategory::Entities => Ok(DescribeResponse {
                category: category.name().to_string(),
                items: json!([{
                    "name": "x",
                    "testabel": true,
                    "contract_target": false,
                    "fields": [{ "name": "f", "field_type": "string", "requird": true }],
                }]),
            }),
            DeclaredCategory::Surfaces => Ok(DescribeResponse {
                category: category.name().to_string(),
                items: json!([{
                    "commands": [{
                        "id": "list", "title": "List", "description": "d", "export": "cmd__list",
                        "args": [{ "name": "n", "arg_type": "integer", "minimun": 1 }],
                        "sandbox": { "fs_read": true, "fs_raed": true }
                    }]
                }]),
            }),
            DeclaredCategory::Enhancements => Ok(DescribeResponse {
                category: category.name().to_string(),
                items: json!([{
                    "target_kind": "x",
                    "source_extension": "@a/b",
                    "fields": [],
                    "edge_types": [{ "label": "L" }],
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
            // A 1.2 guest's enhancement `edge_types`: ignored, reported.
            UnknownKey {
                category: "enhancements",
                item: "x".to_string(),
                key: "edge_types".to_string(),
            },
            // A 1.4 guest's enhancement source_extension: ignored, reported (ADR 0096).
            UnknownKey {
                category: "enhancements",
                item: "x".to_string(),
                key: "source_extension".to_string(),
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

/// Identity, peers and verify kinds read through the declaration; what it
/// contributes is its content, the handshake says only `providers`.
#[test]
fn a_declaration_reads_its_identity_and_its_providers() {
    let mut declaration = full();
    assert_eq!(declaration.name(), "@acme/reports");
    assert_eq!(declaration.version(), "0.1.0");
    assert_eq!(declaration.peers()[0].name, "@acme/base");
    assert_eq!(declaration.verify_kinds(), ["unit"]);

    assert!(!declaration.handshake.providers);
    let wire = serde_json::to_value(&declaration.handshake).unwrap();
    assert!(wire.get("providers").is_none() && wire.get("contribution_flags").is_none());
    declaration.handshake.providers = true;
    let wire = serde_json::to_value(&declaration.handshake).unwrap();
    assert_eq!(wire["providers"], json!(true));
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
    for key in ["ext_short", "description", "keywords", "providers"] {
        assert!(wire.get(key).is_none(), "{key} in {wire}");
    }
    let parsed: HandshakeResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(parsed, handshake);
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "the declared categories are the protocol's only describe categories"
)]
fn the_declared_categories_are_the_only_describe_categories() {
    let names: Vec<&str> = DeclaredCategory::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(DECLARED_CATEGORIES, names.as_slice());
    for category in DeclaredCategory::ALL {
        assert_eq!(DeclaredCategory::from_name(category.name()), Some(category));
    }
    for other in ["fields", "grammars", "body_parsers", "nope"] {
        assert_eq!(DeclaredCategory::from_name(other), None, "{other}");
    }
}

#[test]
fn declares_reads_the_content() {
    let declared = full();
    // `full()` declares nothing in collectors or analyzers.
    let all: Vec<DeclaredCategory> = declared.declared_categories().collect();
    let expected: Vec<DeclaredCategory> = DeclaredCategory::ALL
        .into_iter()
        .filter(|c| {
            !matches!(
                c,
                DeclaredCategory::Collectors | DeclaredCategory::Analyzers
            )
        })
        .collect();
    assert_eq!(all, expected);
    for category in DeclaredCategory::ALL {
        let items = declared.describe_items(category);
        assert_eq!(
            declared.declares(category),
            !items.as_array().unwrap().is_empty(),
            "{category:?}"
        );
    }
    let empty = ExtensionDeclaration::default();
    assert_eq!(empty.declared_categories().count(), 0);
    assert!(!empty.declares(DeclaredCategory::Surfaces));
    let mut surfaced = ExtensionDeclaration::default();
    surfaced.surfaces.commands.push(CommandDescriptor {
        id: "list".to_string(),
        title: "List".to_string(),
        description: "d".to_string(),
        export: "cmd__list".to_string(),
        ..Default::default()
    });
    assert!(surfaced.declares(DeclaredCategory::Surfaces));
    assert_eq!(
        surfaced.declared_categories().collect::<Vec<_>>(),
        [DeclaredCategory::Surfaces]
    );
}

/// A descriptor carrying members, a target union, bounds and a file base keeps them on the wire (ADR 0285); one
/// that writes none reads as before.
#[specforge_test_macros::test(
    behavior = "check_field_value_types",
    verify = "a field descriptor carries its members, target kinds, bounds and file base on the wire"
)]
fn a_field_declares_members_a_union_bounds_and_a_base() {
    let wire = json!({
        "name": "requires", "field_type": "block_list",
        "members": [
            {"name": "phase", "field_type": "reference", "target_kinds": ["phase", "milestone"], "required": true},
            {"name": "score", "field_type": "integer", "minimum": 0, "maximum": 100},
            {"name": "path", "field_type": "string", "file_reference": true, "file_base": "project"},
        ],
    });
    let field: FieldDescriptor = serde_json::from_value(wire.clone()).unwrap();
    let [phase, score, path] = &field.members[..] else {
        panic!("three members: {:?}", field.members);
    };
    assert_eq!(phase.target_kinds, ["phase", "milestone"]);
    assert_eq!((score.minimum, score.maximum), (Some(0), Some(100)));
    assert_eq!(path.file_base.as_deref(), Some("project"));
    let written = serde_json::to_value(&field).unwrap();
    let again: FieldDescriptor = serde_json::from_value(written).unwrap();
    assert_eq!(again, field, "the attributes survive the wire");

    let older = json!({"name": "title", "field_type": "string"});
    let field: FieldDescriptor = serde_json::from_value(older.clone()).unwrap();
    let written = serde_json::to_value(&field).unwrap();
    for key in ["members", "target_kinds", "minimum", "maximum", "file_base"] {
        assert!(
            written.get(key).is_none(),
            "{key} is written for a field that declares none"
        );
    }
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a code catalog crosses the wire and a guest older than 1.7 is not asked for it"
)]
fn codes_round_trip_on_the_wire() {
    let declared = full();
    let items = declared.describe_items(DeclaredCategory::Codes);
    assert_eq!(items[0]["code"], "R001");
    assert_eq!(items[0]["docs"], "https://acme.dev/codes#r001");
    assert!(items[1].get("docs").is_none(), "{}", items[1]);
    assert!(declared.declares(DeclaredCategory::Codes));
    let loaded = ExtensionDeclaration::from_wire(
        declared.handshake.clone(),
        |category| answer(&declared, category),
        |key| panic!("unexpected unknown key {key:?}"),
    )
    .unwrap();
    assert_eq!(loaded.codes, declared.codes);
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a code catalog crosses the wire and a guest older than 1.7 is not asked for it"
)]
fn a_1_6_guest_is_asked_no_codes_and_loads() {
    let mut handshake = full().handshake;
    handshake.protocol_version = "1.6.0".to_string();
    let loaded = ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            assert_ne!(
                category,
                DeclaredCategory::Codes,
                "a 1.6 guest has no codes"
            );
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: json!([]),
            })
        },
        |_| {},
    )
    .unwrap();
    assert!(loaded.codes.is_empty());
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a code catalog crosses the wire and a guest older than 1.7 is not asked for it"
)]
fn a_1_10_guest_is_asked_for_codes() {
    // Numeric, not string, order: "1.10" sorts before "1.7" as text.
    let mut handshake = full().handshake;
    handshake.protocol_version = "1.10.0".to_string();
    let mut asked_codes = false;
    ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            asked_codes |= category == DeclaredCategory::Codes;
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: json!([]),
            })
        },
        |_| {},
    )
    .unwrap();
    assert!(asked_codes);
}

#[test]
fn the_writer_and_history_keys_round_trip_and_stay_off_the_wire_when_unset() {
    let wire = json!({
        "name": "thing", "append_only": true,
        "fields": [{"name": "s", "field_type": "string", "writer": "computed", "immutable": true,
                    "append_only": true}],
    });
    let kind: EntityKindDescriptor = serde_json::from_value(wire.clone()).unwrap();
    assert!(kind.append_only);
    assert_eq!(kind.fields[0].writer.as_deref(), Some("computed"));
    assert_eq!(
        serde_json::to_value(&kind).unwrap()["fields"][0]["writer"],
        "computed"
    );
    let plain = serde_json::to_value(field("a")).unwrap();
    for key in ["writer", "immutable", "append_only"] {
        assert!(plain.get(key).is_none(), "{key}");
    }
    let pass: CompilerPassDescriptor = serde_json::from_value(
        json!({"name": "p", "computes": ["thing.s"], "baseline": ["thing"]}),
    )
    .unwrap();
    assert_eq!(
        (pass.computes, pass.baseline),
        (vec!["thing.s".into()], vec!["thing".into()])
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a settings category round-trips its three item kinds and an unknown kind costs that item"
)]
fn a_settings_category_round_trips() {
    let items = json!([
        {"kind": "setting", "name": "workflow.tdd_mode", "field_type": "bool", "default": false,
         "gates": ["E403"]},
        {"kind": "preset", "source": "profile", "values": {"strict": {
            "settings": {"workflow.tdd_mode": true}, "severities": {"W103": "error"}}}},
        {"kind": "secret", "name": "brave_api_key", "env": "BRAVE_API_KEY"},
        {"kind": "mystery", "name": "x"},
        {"kind": "setting", "field_type": "bool"},
    ]);
    let mut declared = ExtensionDeclaration::default();
    declared
        .set_category(DeclaredCategory::Settings, &items)
        .unwrap();
    assert!(declared.declares(DeclaredCategory::Settings));
    assert!(matches!(declared.settings[0], SettingsItem::Setting(_)));
    assert!(matches!(declared.settings[1], SettingsItem::Preset(_)));
    assert!(matches!(declared.settings[2], SettingsItem::Secret(_)));
    // An unknown kind and a setting with no name cost those items, not the category.
    for bad in &declared.settings[3..] {
        assert!(matches!(bad, SettingsItem::Unreadable(_)), "{bad:?}");
    }
    assert_eq!(declared.describe_items(DeclaredCategory::Settings), items);
    // A 1.7 guest is not asked for it; a key the descriptor does not define is W138.
    let mut handshake = full().handshake;
    handshake.protocol_version = "1.7.0".to_string();
    ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            assert_ne!(category, DeclaredCategory::Settings);
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: json!([]),
            })
        },
        |_| {},
    )
    .unwrap();
    let mut unknown = Vec::new();
    let mut handshake = full().handshake;
    handshake.protocol_version = PROTOCOL_VERSION.to_string();
    ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: if category == DeclaredCategory::Settings {
                    json!([{"kind": "secret", "name": "k", "env": "K", "enviro": "x"}])
                } else {
                    json!([])
                },
            })
        },
        |key| unknown.push((key.category, key.item, key.key)),
    )
    .unwrap();
    assert_eq!(
        unknown,
        [("settings", "k".to_string(), "enviro".to_string())]
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a feature flag reads as a bool setting of its name and default"
)]
fn a_feature_flag_reads_as_a_bool_setting() {
    let flag = FeatureFlagDescriptor {
        name: "experimental".to_string(),
        description: Some("Try it".to_string()),
        default_enabled: true,
    };
    let setting = flag.as_setting();
    assert_eq!(
        (setting.name.as_str(), setting.field_type.as_str()),
        ("experimental", "bool")
    );
    assert_eq!(setting.default, Some(json!(true)));
    assert_eq!(setting.description.as_deref(), Some("Try it"));
}

#[specforge_test_macros::test(
    behavior = "declare_judged_rule",
    verify = "a judged rule is declared with its judge, outcomes and per-target selectors"
)]
fn a_judged_rule_is_declared_with_selectors() {
    let wire = json!({
        "code": "W311", "severity": "warning", "check": "judged", "message_template": "m",
        "judged": {
            "judge": "checker", "outcomes": ["passed", "issues"], "passing": ["passed"],
            "targets": [{"kind": "plan", "unless_set": ["pattern"],
                "inputs": [{"name": "plan", "selector": "self"}]}],
            "values": ["phase_spec.scores"], "outcome_fields": ["plan.verdict"],
        },
    });
    let rule: ValidationRuleDescriptor = serde_json::from_value(wire.clone()).unwrap();
    // It round-trips whole: the host keeps what an extension declared.
    let written = serde_json::to_value(&rule).unwrap();
    assert_eq!(written["judged"], wire["judged"]);
    assert_eq!(
        serde_json::from_value::<ValidationRuleDescriptor>(written).unwrap(),
        rule
    );
}
