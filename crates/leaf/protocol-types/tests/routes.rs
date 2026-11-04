//! The routes an extension declares (protocol 1.15.0, ADR 0330): the `routes` describe category on the wire.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::json;
use specforge_protocol_types::{
    DeclaredCategory, DescribeResponse, ExtensionDeclaration, HandshakeResponse, PROTOCOL_VERSION,
    RouteAnnotationDescriptor, RouteDescriptor, RouteStepDescriptor, UnknownKey,
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

/// The requirement thread plan 04 declares, by hand.
fn route() -> RouteDescriptor {
    let step = |name: &str, via: &str| RouteStepDescriptor {
        name: name.to_string(),
        via: via.to_string(),
        ..Default::default()
    };
    RouteDescriptor {
        name: "requirement-thread".to_string(),
        description: Some("A requirement's thread".to_string()),
        start: "requirement".to_string(),
        steps: vec![
            RouteStepDescriptor {
                label: Some("plan".to_string()),
                ..step("plan", "<plan.requirements")
            },
            step("task", "<task.plan"),
            RouteStepDescriptor {
                annotations: vec![RouteAnnotationDescriptor {
                    field: Some("linked_by".to_string()),
                    ..Default::default()
                }],
                ..step("commit", "<commit.task")
            },
            RouteStepDescriptor {
                from: Some("start".to_string()),
                annotations: vec![RouteAnnotationDescriptor {
                    field: Some("status".to_string()),
                    of: Some("<test_run.test".to_string()),
                    absent: Some("not run".to_string()),
                    ..Default::default()
                }],
                ..step("test", "<truth.requirement.proof")
            },
            RouteStepDescriptor {
                from: Some("start".to_string()),
                annotations: vec![RouteAnnotationDescriptor {
                    judgement: Some("E501".to_string()),
                    ..Default::default()
                }],
                ..step("truth", "<truth.requirement")
            },
        ],
        ..Default::default()
    }
}
#[specforge_test_macros::test(
    behavior = "declare_trace_route",
    verify = "a routes category round trips on the wire and a 1.14 guest is never asked for it"
)]
fn a_routes_category_round_trips() {
    let items = json!([{
        "name": "requirement-thread",
        "description": "A requirement's thread",
        "start": "requirement",
        "steps": [
            {"name": "plan", "via": "<plan.requirements", "label": "plan"},
            {"name": "task", "via": "<task.plan"},
            {"name": "commit", "via": "<commit.task", "annotations": [{"field": "linked_by"}]},
            {"name": "test", "from": "start", "via": "<truth.requirement.proof",
             "annotations": [{"field": "status", "of": "<test_run.test", "absent": "not run"}]},
            {"name": "truth", "from": "start", "via": "<truth.requirement",
             "annotations": [{"judgement": "E501"}]},
        ],
    }]);
    let mut declared = ExtensionDeclaration::default();
    assert!(!declared.declares(DeclaredCategory::Routes));
    declared
        .set_category(DeclaredCategory::Routes, &items)
        .unwrap();
    assert!(declared.declares(DeclaredCategory::Routes));
    assert_eq!(declared.routes, [route()]);
    assert_eq!(declared.describe_items(DeclaredCategory::Routes), items);
    assert_eq!(
        DeclaredCategory::from_name("routes"),
        Some(DeclaredCategory::Routes)
    );
    // Routes come right after settings; capabilities (ADR 0335) and checkers (ADR 0336) after them.
    assert_eq!(
        DeclaredCategory::ALL
            .iter()
            .position(|c| *c == DeclaredCategory::Routes),
        Some(DeclaredCategory::ALL.len() - 3)
    );
    // A declaration with no route serializes no `routes` key: stored declarations stay byte-identical.
    let stored = serde_json::to_value(ExtensionDeclaration::default()).unwrap();
    assert!(stored.get("routes").is_none(), "{stored}");
}

#[specforge_test_macros::test(
    behavior = "declare_trace_route",
    verify = "a routes category round trips on the wire and a 1.14 guest is never asked for it"
)]
fn a_1_14_guest_is_asked_no_routes_and_loads() {
    let loaded = ExtensionDeclaration::from_wire(
        handshake("1.14.0"),
        |category| {
            assert_ne!(
                category,
                DeclaredCategory::Routes,
                "a 1.14 guest has no routes"
            );
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: json!([]),
            })
        },
        |_| {},
    )
    .unwrap();
    assert!(loaded.routes.is_empty());
}

#[specforge_test_macros::test(
    behavior = "declare_trace_route",
    verify = "an unknown key of a route, a step or an annotation is reported"
)]
fn an_unknown_route_step_key_is_reported() {
    let mut unknown = Vec::new();
    ExtensionDeclaration::from_wire(
        handshake(PROTOCOL_VERSION),
        |category| {
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items: if category == DeclaredCategory::Routes {
                    json!([{
                        "name": "requirement-thread", "start": "requirement", "strat": "x",
                        "annotations": [{"field": "title", "absnet": "-"}],
                        "steps": [{"name": "plan", "via": "<plan.requirements", "lable": "plan",
                                   "annotations": [{"judgment": "E501"}]}],
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
        category: "routes",
        item: item.to_string(),
        key: key.to_string(),
    };
    assert_eq!(
        unknown,
        [
            key("requirement-thread", "strat"),
            key("requirement-thread.steps[plan]", "lable"),
            key("requirement-thread.steps[plan].annotations[#0]", "judgment"),
            key("requirement-thread.annotations[#0]", "absnet"),
        ]
    );
}
