//! The context a role reads for one entity (ADR 0356): a budgeted read of a plan for its executor.

use std::sync::Arc;

use crate::support::*;
use serde_json::{Value, json};
use specforge_ops::read::context::{ContextRequest, context};
use specforge_ops::read::export::Format;
use specforge_ops::view::ProjectView;
use specforge_project::SharedRuntime;
use specforge_project::testing::{CONTEXT_PLAN, gsd_context_project, gsd_context_runtime};
use specforge_test::prelude::*;

/// The pack `specforge context` prints for `route` on the context plan at `max_tokens`.
fn printed(route: &str, max_tokens: Option<usize>) -> String {
    let project = gsd_context_project();
    let compiled = project.compile_in(Arc::new(gsd_context_runtime()) as SharedRuntime);
    let request = ContextRequest {
        route,
        subject: CONTEXT_PLAN,
        max_tokens,
        format: Format::Context,
    };
    context(&ProjectView::of(&compiled), &request)
        .unwrap_or_else(|refusal| panic!("context refused: {:?}", refusal.error))
        .text
}

fn context_served() -> Served {
    Served::initialized(gsd_context_project().serving(gsd_context_runtime()))
}

fn ids(nodes: &Value) -> Vec<&str> {
    nodes
        .as_array()
        .unwrap_or_else(|| panic!("no nodes in {nodes}"))
        .iter()
        .filter_map(|node| node["id"].as_str())
        .collect()
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("no array in {value}"))
        .iter()
        .filter_map(Value::as_str)
        .collect()
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "a budgeted context read of a plan for its executor keeps every core member and cuts only enrich sections"
)]
fn a_1500_token_read_for_a_plans_executor_keeps_its_tasks_and_context() {
    let mut server = context_served();
    let (_, read) = resource(
        &mut server,
        "specforge://context/v1.0/02/02-01?role=gsd/executor&max_tokens=1500",
    );
    let nodes = ids(&read["graph"]["nodes"]);
    assert!(nodes.contains(&"v1.0/02/02-01/T1"), "{read}");
    assert!(!nodes.contains(&"v1.0/01"), "{read}");
    let context = read["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["name"] == "context")
        .unwrap_or_else(|| panic!("the context section is served: {read}"));
    assert_eq!(
        strings(&context["members"]),
        ["v1.0/02/02-CONTEXT", "v1.0/02/DI1"]
    );
    let budget = &read["budget"];
    assert!(
        !strings(&budget["truncated_sections"]).contains(&"context"),
        "{budget}"
    );
    let core: Vec<&str> = read["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|section| section["tier"] == "core" && section["kind"] == "entities")
        .flat_map(|section| strings(&section["members"]))
        .collect();
    let truncated = strings(&budget["truncated_entities"]);
    assert!(core.iter().all(|id| !truncated.contains(id)), "{budget}");
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "specforge://context/{+entity_id}?role= answers the role's pack, and depth or kinds beside role are refused naming the key"
)]
fn with_role_depth_and_kinds_are_refused_naming_the_key() {
    let mut server = context_served();
    for (query, key) in [("depth=1", "depth"), ("kinds=task", "kinds")] {
        let uri = format!("specforge://context/v1.0/02/02-01?role=gsd/executor&{query}");
        let reply = read_resource(&mut server, &uri);
        let error = &reply["error"]["data"];
        assert_eq!(error["code"], "invalid_input", "{reply}");
        assert_eq!(error["argument"], key, "{reply}");
        assert!(
            error["message"].as_str().unwrap().contains("with role"),
            "{reply}"
        );
    }
    let reply = read_resource(
        &mut server,
        "specforge://context/v1.0/02/02-01?role=gsd/nobody",
    );
    assert_eq!(reply["error"]["data"]["argument"], "role", "{reply}");
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "specforge://context/{+entity_id}?role= answers the role's pack, and depth or kinds beside role are refused naming the key"
)]
fn the_role_read_is_byte_equal_to_the_context_tool() {
    let mut server = context_served();
    let read = resource_text(&read_resource(
        &mut server,
        "specforge://context/v1.0/02/02-01?role=gsd/executor&max_tokens=1500",
    ));
    let tool = tool_text(&call_tool(
        &mut server,
        "specforge.context",
        json!({"route": "gsd/executor", "subject": CONTEXT_PLAN, "max_tokens": 1500}),
    ));
    assert_eq!(read, tool);
    assert_eq!(read, printed("gsd/executor", Some(1500)));
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "specforge.context answers the pack specforge context prints"
)]
fn the_context_tool_answers_what_the_cli_prints() {
    let mut server = context_served();
    for (route, max_tokens) in [
        ("gsd/executor", None),
        ("gsd/executor", Some(1500)),
        ("gsd/plan-checker", None),
    ] {
        let mut arguments = json!({"route": route, "subject": CONTEXT_PLAN});
        if let Some(max_tokens) = max_tokens {
            arguments["max_tokens"] = json!(max_tokens);
        }
        let reply = call_tool(&mut server, "specforge.context", arguments);
        assert_eq!(
            tool_text(&reply),
            printed(route, max_tokens),
            "{route} at {max_tokens:?}"
        );
    }
}

#[test]
fn the_context_tool_refuses_an_unknown_route_on_route() {
    let mut server = context_served();
    let reply = call_tool(
        &mut server,
        "specforge.context",
        json!({"route": "gsd/nobody", "subject": CONTEXT_PLAN}),
    );
    let error = crate::tool_errors::mcp_error(&reply);
    assert_eq!(error["code"], "invalid_input", "{error}");
    assert_eq!(error["argument"], "route", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("gsd/executor"),
        "{error}"
    );
}
