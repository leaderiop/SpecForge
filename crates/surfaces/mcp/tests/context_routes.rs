//! The context a role reads for one entity (ADR 0356): a budgeted read of a plan for its executor.

use std::sync::Arc;

use crate::support::*;
use serde_json::{Value, json};
use specforge_ops::read::context::{ContextRequest, context};
use specforge_ops::read::export::Format;
use specforge_ops::view::ProjectView;
use specforge_project::SharedRuntime;
use specforge_project::testing::{
    CONTEXT_PLAN, gsd_context_project, gsd_context_runtime, gsd_routed_runtime,
};
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

// Flips once the executor's pack serves the read: the read then keeps every task and the phase's context.
#[specforge_test(
    behavior = "serve_context_pack",
    verify = "a budgeted context read of a plan for its executor keeps every core member and cuts only enrich sections"
)]
fn a_1500_token_read_for_a_plans_executor_keeps_its_tasks_and_context() {
    let mut server = Served::initialized(gsd_context_project().serving(gsd_routed_runtime()));
    let (_, read) = resource(
        &mut server,
        "specforge://context/v1.0/02/02-01?max_tokens=1500",
    );
    let truncated = &read["token_budget"]["truncated_entities"];
    assert!(
        truncated
            .as_array()
            .is_some_and(|cut| cut.iter().any(|id| id == "v1.0/02/02-01/T1")),
        "{read}"
    );
    let nodes = ids(&read["nodes"]);
    assert!(!nodes.contains(&"v1.0/02/02-CONTEXT"), "{read}");
    assert!(nodes.contains(&"v1.0/01"), "{read}");
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
