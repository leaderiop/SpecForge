//! An agent's call stays off the network until it asks otherwise (ADR 0337 D4): what an MCP tool is lent is `cached`
//! unless its `effects` argument says, and a resource read has no such argument.

use std::sync::Arc;

use crate::support::*;
use serde_json::json;
use specforge_effects::testing::{self, FakeHttp, MemoryConsent};
use specforge_effects::{HttpRequest, Ports};
use specforge_extension_sdk::host;
use specforge_extension_sdk::prelude::*;
use specforge_mcp::McpServer;
use specforge_project::SharedRuntime;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@t/lent";
const TOOL: &str = "specforge.lent.ping";
const URL: &str = "https://example.test/a";

fn declaration() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.capability(
        Capability::http("api")
            .hosts(&["example.test"])
            .schemes(&["https"])
            .why("call the API"),
    );
    c.command("refused", |cmd| {
        cmd.title("Refused").handler(|_| {
            let refused = host::http::send(&HttpRequest {
                method: "GET".to_string(),
                url: "https://elsewhere.test/a".to_string(),
                ..Default::default()
            });
            panic!("traps after {refused:?}");
        });
    });
    c.command("ping", |cmd| {
        cmd.title("Ping").handler(|_| {
            let answered = host::http::send(&HttpRequest {
                method: "GET".to_string(),
                url: URL.to_string(),
                ..Default::default()
            });
            CommandOutput::ok(match answered {
                Ok(response) => format!("status {}\n", response.status),
                Err(host::HostError::Failed(failure)) => format!("{}\n", failure.class),
                Err(other) => format!("{other:?}\n"),
            })
        });
    });
    c
}

/// A served project, every approval given, and the fake its live calls reach.
fn served() -> (Served, Arc<FakeHttp>) {
    let runtime = InProcessRuntime::new()
        .with(declaration)
        .importing(EXT, &["specforge:host/http@0.1.0"]);
    let http = Arc::new(FakeHttp::new().route(URL, 200, "answer"));
    let project = TestProject::new().serving(runtime).enabling(&[EXT]);
    let shared = Arc::clone(project.served().unwrap()) as SharedRuntime;
    let server = McpServer::over(Ports {
        http: http.clone(),
        ..testing::ports()
    })
    .with_consent(Arc::new(MemoryConsent::approving_everything()))
    .with_extension_runtime(shared);
    (Served::over(project, server), http)
}

#[test]
fn a_tool_that_traps_still_reports_what_it_was_refused() {
    let (mut served, _) = served();
    let reply = call_tool(&mut served, "specforge.lent.refused", json!({}));
    assert!(reply.to_string().contains("E090"), "{reply}");
}

#[test]
fn an_mcp_tool_stays_off_the_network_by_default() {
    let (mut served, http) = served();

    let reply = call_tool(&mut served, TOOL, json!({}));
    assert_eq!(tool_text(&reply), "not_performed\n", "{reply}");
    assert!(http.requests().is_empty(), "nothing went out");

    let reply = call_tool(&mut served, TOOL, json!({"effects": "online"}));
    assert_eq!(tool_text(&reply), "status 200\n", "{reply}");
    assert_eq!(http.requests().len(), 1);

    // What the call fetched, cached answers, off the network.
    let reply = call_tool(&mut served, TOOL, json!({}));
    assert_eq!(tool_text(&reply), "status 200\n", "{reply}");
    assert_eq!(http.requests().len(), 1);
}

#[test]
fn a_tool_of_an_effectful_extension_lists_effects_and_refuses_a_mode_it_does_not_know() {
    let (mut served, _) = served();

    let listed = call(&mut served, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().unwrap();
    let ping = tools.iter().find(|t| t["name"] == TOOL).unwrap();
    assert_eq!(
        ping["inputSchema"]["properties"]["effects"]["enum"],
        json!(["online", "auto", "cached", "replay"]),
        "{ping}"
    );

    let reply = call_tool(&mut served, TOOL, json!({"effects": "offline"}));
    assert!(
        reply.to_string().contains("`effects` must be one of"),
        "{reply}"
    );
}
