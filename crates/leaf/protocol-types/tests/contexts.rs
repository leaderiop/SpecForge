//! The context routes an extension declares (protocol 2.1.0, ADR 0356): the `contexts` describe category on the wire.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::json;
use specforge_protocol_types::{
    ContextDescriptor, ContextSectionDescriptor, ContextTier, DeclaredCategory, DescribeResponse,
    ExtensionDeclaration, HandshakeResponse, PROTOCOL_VERSION, UnknownKey,
};

/// A handshake of `version`.
fn handshake(version: &str) -> HandshakeResponse {
    HandshakeResponse {
        name: "@acme/work".to_string(),
        version: "1.0.0".to_string(),
        protocol_version: version.to_string(),
        ..Default::default()
    }
}

#[specforge_test_macros::test(
    behavior = "declare_context_route",
    verify = "a contexts category round trips on the wire and a 2.0 guest is never asked for it"
)]
fn a_contexts_category_round_trips_and_a_2_0_guest_is_never_asked_for_it() {
    let items = json!([{
        "name": "executor",
        "description": "What an executor reads",
        "subject": "plan",
        "agent": "work-executor",
        "budget": 6000,
        "sections": [
            {"name": "plan", "selector": "self", "tier": "core"},
            {"name": "decisions", "selector": "self.phase<decision.phase", "tier": "enrich"},
        ],
        "settings": ["plan_check"],
    }]);
    let mut declared = ExtensionDeclaration::default();
    assert!(!declared.declares(DeclaredCategory::Contexts));
    declared
        .set_category(DeclaredCategory::Contexts, &items)
        .unwrap();
    let section = |name: &str, selector: &str, tier| ContextSectionDescriptor {
        name: name.to_string(),
        selector: selector.to_string(),
        tier,
    };
    assert_eq!(
        declared.contexts,
        [ContextDescriptor {
            name: "executor".to_string(),
            description: Some("What an executor reads".to_string()),
            subject: "plan".to_string(),
            agent: Some("work-executor".to_string()),
            budget: 6000,
            sections: vec![
                section("plan", "self", ContextTier::Core),
                section(
                    "decisions",
                    "self.phase<decision.phase",
                    ContextTier::Enrich
                ),
            ],
            settings: vec!["plan_check".to_string()],
            records: Vec::new(),
            stale: None,
        }]
    );
    assert_eq!(declared.describe_items(DeclaredCategory::Contexts), items);
    assert_eq!(
        DeclaredCategory::from_name("contexts"),
        Some(DeclaredCategory::Contexts)
    );
    // A tier left out is core: a forgotten tier fails loud, never cut silently.
    let untiered: Vec<ContextSectionDescriptor> =
        serde_json::from_value(json!([{"name": "plan", "selector": "self"}])).unwrap();
    assert_eq!(untiered[0].tier, ContextTier::Core);
    // A declaration with no context route serializes no `contexts` key: stored declarations stay byte-identical.
    let stored = serde_json::to_value(ExtensionDeclaration::default()).unwrap();
    assert!(stored.get("contexts").is_none(), "{stored}");

    let loaded = ExtensionDeclaration::from_wire(
        handshake("2.0.0"),
        |category| {
            assert_ne!(
                category,
                DeclaredCategory::Contexts,
                "a 2.0 guest has no contexts"
            );
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: json!([]),
            })
        },
        |_| {},
    )
    .unwrap();
    assert!(loaded.contexts.is_empty());
}

#[specforge_test_macros::test(
    behavior = "declare_context_route",
    verify = "an unknown key of a context route or a section is reported"
)]
fn an_unknown_key_of_a_context_route_or_a_section_is_reported() {
    let mut unknown = Vec::new();
    ExtensionDeclaration::from_wire(
        handshake(PROTOCOL_VERSION),
        |category| {
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: if category == DeclaredCategory::Contexts {
                    json!([{
                        "name": "executor", "subject": "plan", "budget": 10, "subjet": "x",
                        "sections": [{"name": "plan", "selector": "self", "teir": "core"}],
                    }])
                } else {
                    json!([])
                },
            })
        },
        |key| unknown.push(key),
    )
    .unwrap();
    let key = |item: &str, key: &str| UnknownKey {
        category: "contexts",
        item: item.to_string(),
        key: key.to_string(),
    };
    assert_eq!(
        unknown,
        [
            key("executor", "subjet"),
            key("executor.sections[plan]", "teir"),
        ]
    );
}

#[specforge_test_macros::test(
    behavior = "record_harness_values",
    verify = "a context writer round trips on the wire"
)]
fn a_context_writer_round_trips() {
    use specforge_protocol_types::FieldWriter;
    assert_eq!(FieldWriter::parse("context"), Some(FieldWriter::Context));
    assert_eq!(FieldWriter::Context.as_str(), "context");
    let wire = json!({
        "name": "context_pack", "field_type": "block", "writer": "context",
        "members": [
            {"name": "route", "field_type": "string"},
            {"name": "digest", "field_type": "string"},
            {"name": "max_tokens", "field_type": "integer"},
        ],
    });
    let field: specforge_protocol_types::FieldDescriptor =
        serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(field.writer.as_deref(), Some("context"));
    assert_eq!(serde_json::to_value(&field).unwrap()["writer"], "context");
}

#[specforge_test_macros::test(
    behavior = "declare_context_route",
    verify = "a route's records and stale code round trip on the wire, and an unknown record key is reported"
)]
fn records_and_stale_round_trip_and_an_unknown_record_key_is_reported() {
    use specforge_protocol_types::ContextRecordDescriptor;
    let item = json!({
        "name": "executor", "subject": "plan", "budget": 10,
        "sections": [{"name": "plan", "selector": "self", "tier": "core"}],
        "records": [
            {"field": "plan.context_pack", "subject": "self", "suggestion": "run start-plan {entity}"},
            {"field": "research.context_pack", "subject": "self.plan"},
        ],
        "stale": "W524",
    });
    let route: ContextDescriptor = serde_json::from_value(item.clone()).unwrap();
    assert_eq!(
        route.records,
        [
            ContextRecordDescriptor {
                field: "plan.context_pack".into(),
                subject: "self".into(),
                suggestion: Some("run start-plan {entity}".into()),
            },
            ContextRecordDescriptor {
                field: "research.context_pack".into(),
                subject: "self.plan".into(),
                suggestion: None,
            },
        ]
    );
    assert_eq!(route.stale.as_deref(), Some("W524"));
    assert_eq!(serde_json::to_value(&route).unwrap(), item);
    let bare: ContextDescriptor = serde_json::from_value(json!({
        "name": "executor", "subject": "plan", "budget": 10,
        "sections": [{"name": "plan", "selector": "self"}],
    }))
    .unwrap();
    let written = serde_json::to_value(&bare).unwrap();
    assert!(written.get("records").is_none() && written.get("stale").is_none());

    let mut unknown = Vec::new();
    ExtensionDeclaration::from_wire(
        handshake(PROTOCOL_VERSION),
        |category| {
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: if category == DeclaredCategory::Contexts {
                    json!([{
                        "name": "executor", "subject": "plan", "budget": 10,
                        "sections": [{"name": "plan", "selector": "self"}],
                        "records": [{"field": "plan.context_pack", "subject": "self", "subjet": "x"}],
                        "stale": "W524",
                    }])
                } else {
                    json!([])
                },
            })
        },
        |key| unknown.push(key),
    )
    .unwrap();
    assert_eq!(
        unknown,
        [UnknownKey {
            category: "contexts",
            item: "executor.records[#0]".to_string(),
            key: "subjet".to_string(),
        }]
    );
}
