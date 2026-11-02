//! `specforge.trace` along the route an extension declares (ADR 0330): the route document `specforge trace <entity>
//! --format json` writes, the reach when asked, and a route refused on `route`.

use crate::support::*;
use serde_json::json;
use specforge_project::testing::{GSD_ROUTED_SPEC, gsd_routed_project, gsd_routed_runtime};
use specforge_test::prelude::*;

/// `@gsd/core` with the route, over the routed fixture's spec.
fn serving() -> Served {
    Served::initialized(gsd_routed_project(GSD_ROUTED_SPEC).serving(gsd_routed_runtime()))
}

#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "specforge.trace follows a kind's declared route, a named route, or the reach as asked"
)]
fn trace_answers_the_route_document() {
    let mut server = serving();
    let document = server.tool("specforge.trace", json!({"entity_id": "AUTH-07"}));
    assert_eq!(document["route"], "gsd/requirement-thread", "{document}");
    assert_eq!(
        document["text"],
        "AUTH-07 → plan 03-02 → T1 → 5e2a91c0d3b4 (Gsd-Task) → auth-keeps-session (passed) → TR1 missing"
    );
    assert_eq!(document["complete"], true);
    let named = server.tool(
        "specforge.trace",
        json!({"entity_id": "AUTH-07", "route": "gsd/requirement-thread"}),
    );
    assert_eq!(named, document);
}

#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "specforge.trace follows a kind's declared route, a named route, or the reach as asked"
)]
fn reach_answers_the_chain() {
    let mut server = serving();
    let chain = server.tool(
        "specforge.trace",
        json!({"entity_id": "AUTH-07", "reach": true}),
    );
    assert!(chain.get("route").is_none(), "{chain}");
    assert_eq!(chain["entity_id"], "AUTH-07");
    assert!(chain["upstream"].is_array(), "{chain}");
}

#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "specforge.trace follows a kind's declared route, a named route, or the reach as asked"
)]
fn an_unknown_route_is_invalid_input_on_route() {
    let mut server = serving();
    for arguments in [
        json!({"entity_id": "AUTH-07", "route": "gsd/nope"}),
        json!({"entity_id": "v1.0/03/03-02", "route": "gsd/requirement-thread"}),
        json!({"entity_id": "AUTH-07", "route": "gsd/requirement-thread", "reach": true}),
    ] {
        let reply = server.call_tool("specforge.trace", arguments.clone());
        let error = crate::tool_errors::mcp_error(&reply);
        assert_eq!(error["code"], "invalid_input", "{arguments}: {error}");
        assert_eq!(error["argument"], "route", "{arguments}: {error}");
    }
}
