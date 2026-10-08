use crate::support::*;
use serde_json::json;
use specforge_mcp::McpServer;
use specforge_test::prelude::*;
use std::fs;
use tempfile::TempDir;

fn init_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());
    server
}

// B:mcp_subscription_cleanup — verify unit "subscribe adds subscription"
#[specforge_test(
    behavior = "notify_graph_delta_via_mcp",
    verify = "clients can subscribe and unsubscribe from delta notifications"
)]
fn subscribe_adds_subscription() {
    let mut server = init_server();
    let reply = call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    assert_eq!(reply["result"], json!({}), "{reply}");

    let subs = specforge_mcp::subscriptions::subscribers(
        server.state(),
        specforge_mcp::subscriptions::Watched::Graph,
    );
    assert_eq!(subs, vec!["default"]);
}

#[test]
fn subscribing_twice_records_one_subscription() {
    let mut server = init_server();
    for _ in 0..2 {
        let reply = call(
            &mut server,
            "resources/subscribe",
            json!({"uri": "specforge://graph"}),
        );
        assert_eq!(reply["result"], json!({}), "{reply}");
    }
    assert_eq!(events(&server, "mcp_subscription_created").len(), 1);
}

// B:mcp_subscription_cleanup — verify unit "unsubscribe removes subscription"
#[specforge_test(
    behavior = "notify_graph_delta_via_mcp",
    verify = "clients can subscribe and unsubscribe from delta notifications"
)]
fn unsubscribe_removes_subscription() {
    let mut server = init_server();
    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    let reply = call(
        &mut server,
        "resources/unsubscribe",
        json!({"uri": "specforge://graph"}),
    );
    assert_eq!(reply["result"], json!({}), "{reply}");

    let subs = specforge_mcp::subscriptions::subscribers(
        server.state(),
        specforge_mcp::subscriptions::Watched::Graph,
    );
    assert!(subs.is_empty());
}

// B:mcp_subscription_cleanup — verify unit "shutdown clears all subscriptions"
#[specforge_test(
    behavior = "mcp_shutdown",
    verify = "shutdown unsubscribes all active subscriptions"
)]
fn shutdown_clears_subscriptions() {
    let mut server = init_server();
    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );

    let req = json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}});
    server.handle_message(&req.to_string());

    assert!(server.state().subscriptions.is_empty());
    assert!(
        server
            .state()
            .events
            .iter()
            .any(|e| e.name == "mcp_subscription_removed"
                && e.params["clientId"] == "default"
                && e.params["subscriptionType"] == "specforge/graphChanged"),
        "shutdown emits mcp_subscription_removed"
    );
}

// ---- C9-01: subscribe → recompile → notification loop ----

/// Real mini project so the routed refresh path performs an honest recompile.
fn project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("base.spec"),
        "behavior base \"Base\" {\n    contract \"The system MUST exist\"\n}\n",
    )
    .unwrap();
    dir
}

fn init_with_project(root: &TempDir) -> McpServer {
    let mut server = McpServer::new();
    call(
        &mut server,
        "initialize",
        json!({"projectRoot": root.path().to_str().unwrap()}),
    );
    server
}

/// New content on disk: an extra entity, written while the server runs
/// (the next request that reads the project sees it).
fn evolve_project(root: &TempDir, entity: &str) {
    fs::write(
        root.path().join("spec").join(format!("{}.spec", entity)),
        format!(
            "feature {entity} \"{}\" {{\n    behaviors [base]\n}}\n",
            entity
        ),
    )
    .unwrap();
}

/// C9-01 acceptance: subscribe → recompile with changed content →
/// notification delivered to the captured client sink.
#[test]
fn subscribe_recompile_delivers_graph_notification() {
    let dir = project();
    let mut server = init_with_project(&dir);
    assert!(server.state().graph().node_count() > 0);

    let resp = call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    assert!(resp["result"].is_object(), "subscribe must succeed: {resp}");

    evolve_project(&dir, "fresh_added");
    // Any routed resources/read brings the project up to date.
    call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://diagnostics"}),
    );

    let notifications = server.take_notifications();
    assert_eq!(
        notifications.len(),
        1,
        "exactly one graph delta notification expected: {notifications:?}"
    );
    assert_eq!(notifications[0]["method"], "specforge/graphChanged");
    let added = notifications[0]["params"]["added_nodes"]
        .as_array()
        .unwrap();
    assert!(
        added.iter().any(|n| n == "fresh_added"),
        "delta must name the new entity: {added:?}"
    );
    assert!(
        server
            .state()
            .events
            .iter()
            .any(|e| e.name == "mcp_delta_notified")
    );
}

#[specforge_test(
    behavior = "mcp_shutdown",
    verify = "shutdown flushes pending notifications"
)]
fn shutdown_keeps_pending_notifications_for_the_host() {
    let dir = project();
    let mut server = init_with_project(&dir);
    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    evolve_project(&dir, "fresh_added");
    call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://diagnostics"}),
    );

    call(&mut server, "shutdown", json!({}));

    let notifications = server.take_notifications();
    assert_eq!(notifications.len(), 1, "{notifications:?}");
    assert_eq!(notifications[0]["method"], "specforge/graphChanged");
}

#[specforge_test(
    behavior = "mcp_server_shutdown",
    verify = "emits mcp_server_shutdown with correct counts when MCP server shuts down"
)]
fn shutdown_event_counts_what_it_released() {
    let dir = project();
    let mut server = init_with_project(&dir);
    for uri in ["specforge://graph", "specforge://diagnostics"] {
        call(&mut server, "resources/subscribe", json!({"uri": uri}));
    }
    evolve_project(&dir, "fresh_added");
    call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://diagnostics"}),
    );
    let pending = server.state().notification_outbox.len();
    assert!(pending > 0, "the rebuild queued notifications");

    call(&mut server, "shutdown", json!({}));

    let event = server
        .state()
        .events
        .iter()
        .find(|e| e.name == "mcp_server_shutdown")
        .expect("mcp_server_shutdown emitted");
    assert_eq!(event.params["pending_notifications_flushed"], pending);
    assert_eq!(event.params["subscriptions_released"], 2);
    // The served project's runtime is released with its session.
    assert_eq!(event.params["wasm_engines_released"], 1);
}

fn subscribe_as(server: &mut McpServer, client: &str, uri: &str) {
    let resp = call(
        server,
        "resources/subscribe",
        json!({"uri": uri, "client_id": client}),
    );
    assert!(resp["result"].is_object(), "{resp}");
}

#[specforge_test(
    behavior = "mcp_subscription_cleanup",
    verify = "client disconnect removes all subscriptions for that client"
)]
fn disconnect_removes_only_that_clients_subscriptions() {
    let mut server = init_server();
    subscribe_as(&mut server, "c1", "specforge://graph");
    subscribe_as(&mut server, "c1", "specforge://diagnostics");
    subscribe_as(&mut server, "c2", "specforge://graph");

    server.disconnect("c1");

    let state = server.state();
    assert_eq!(
        specforge_mcp::subscriptions::subscribers(
            state,
            specforge_mcp::subscriptions::Watched::Graph
        ),
        ["c2"]
    );
    assert!(
        specforge_mcp::subscriptions::subscribers(
            state,
            specforge_mcp::subscriptions::Watched::Diagnostics
        )
        .is_empty()
    );
    let removed = state
        .events
        .iter()
        .filter(|e| e.name == "mcp_subscription_removed" && e.params["clientId"] == "c1")
        .count();
    assert_eq!(removed, 2);
}

#[specforge_test(
    behavior = "mcp_subscription_cleanup",
    verify = "rapid connect/disconnect cycles leave zero subscriptions"
)]
fn rapid_connect_disconnect_cycles_leave_nothing_behind() {
    let mut server = init_server();
    for i in 0..50 {
        let client = format!("client_{i}");
        subscribe_as(&mut server, &client, "specforge://graph");
        subscribe_as(&mut server, &client, "specforge://diagnostics");
        server.disconnect(&client);
    }
    assert!(
        server.state().subscriptions.is_empty(),
        "{:?}",
        server.state().subscriptions
    );
}

/// C9-01 contract: no subscribers → notification suppressed.
#[test]
fn recompile_without_subscribers_emits_nothing() {
    let dir = project();
    let mut server = init_with_project(&dir);

    evolve_project(&dir, "fresh_added");
    call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://diagnostics"}),
    );

    assert!(
        server.take_notifications().is_empty(),
        "no subscription means no notification"
    );
}

/// C9-01 contract: unsubscribe stops delivery.
#[test]
fn unsubscribe_stops_delivery() {
    let dir = project();
    let mut server = init_with_project(&dir);

    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    evolve_project(&dir, "fresh_one");
    call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://diagnostics"}),
    );
    assert_eq!(server.take_notifications().len(), 1);

    let resp = call(
        &mut server,
        "resources/unsubscribe",
        json!({"uri": "specforge://graph"}),
    );
    assert!(resp["result"].is_object());

    evolve_project(&dir, "fresh_two");
    call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://diagnostics"}),
    );
    assert!(
        server.take_notifications().is_empty(),
        "unsubscribed clients must not receive notifications"
    );
}

/// Subscribing to the diagnostics resource watches the diagnostics channel.
#[test]
fn diagnostics_subscription_uses_diagnostics_channel() {
    let dir = project();
    let mut server = init_with_project(&dir);

    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://diagnostics"}),
    );
    assert!(
        !specforge_mcp::subscriptions::subscribers(
            server.state(),
            specforge_mcp::subscriptions::Watched::Diagnostics
        )
        .is_empty()
    );
    assert!(
        specforge_mcp::subscriptions::subscribers(
            server.state(),
            specforge_mcp::subscriptions::Watched::Graph
        )
        .is_empty()
    );
}

/// Guard: subscribe requires initialization and a uri parameter.
#[test]
fn resources_subscribe_guards() {
    let mut server = McpServer::new();
    let resp = call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    assert!(resp["error"].is_object(), "must reject before initialize");

    let mut server = init_server();
    let resp = call(&mut server, "resources/subscribe", json!({}));
    assert!(resp["error"].is_object(), "must reject missing uri");
}

// ---- Pins: who hears about what (plan 12, T0) ----

/// The `method` of each notification.
fn methods(sent: &[serde_json::Value]) -> Vec<String> {
    sent.iter()
        .map(|n| n["method"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// A served project holding the behavior `alpha`.
fn alpha() -> Served {
    TestProject::new()
        .file("main.spec", "behavior alpha \"Alpha\" {\n}\n")
        .serve(&[TestExtension::software()])
}

/// A new behavior on disk, brought in by a request that reads the project.
fn change(server: &mut Served) {
    server.write("beta.spec", "behavior beta \"Beta\" {\n}\n");
    call_tool(server, "specforge.stats", json!({}));
}

/// The extension names of the served `specforge://schema`.
fn schema_extensions(server: &mut McpServer) -> Vec<String> {
    let (_, schema) = resource(server, "specforge://schema");
    schema["extensions"]
        .as_array()
        .unwrap_or_else(|| panic!("no extensions in {schema}"))
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect()
}

/// P1 (bug A): an environment reload that changes the schema is heard by
/// nobody.
#[test]
fn an_environment_reload_that_keeps_the_graph_is_heard_by_no_one() {
    let mut server = TestProject::new()
        .enabling(&["@specforge/software"])
        .file(
            "main.spec",
            "behavior alpha \"Alpha\" {\n  contract \"MUST work\"\n}\n",
        )
        .serve_components();
    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://schema"}),
    );
    listen(
        &mut server,
        json!(3),
        &["specforge://schema", "specforge://graph"],
    );
    assert_eq!(schema_extensions(&mut server), ["@specforge/software"]);

    server.write(
        "specforge.json",
        &json!({"name": "t", "version": "0.1.0",
            "extensions": ["@specforge/software", "@specforge/product"]})
        .to_string(),
    );
    call_tool(&mut server, "specforge.stats", json!({}));

    assert!(server.take_notifications().is_empty());
    assert_eq!(
        schema_extensions(&mut server),
        ["@specforge/software", "@specforge/product"]
    );
}

/// P2 (bug B): unsubscribing one graph view unsubscribes them all.
#[test]
fn unsubscribing_one_graph_view_unsubscribes_them_all() {
    let mut server = alpha();
    for uri in ["specforge://graph", "specforge://context"] {
        call(&mut server, "resources/subscribe", json!({"uri": uri}));
    }
    call(
        &mut server,
        "resources/unsubscribe",
        json!({"uri": "specforge://context"}),
    );
    change(&mut server);
    assert!(server.take_notifications().is_empty());
}

/// P3 (bug C): a subscribed resource hears only the SpecForge delta.
#[test]
fn a_subscribed_resource_hears_only_the_specforge_delta() {
    let mut server = alpha();
    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    change(&mut server);
    assert_eq!(
        methods(&server.take_notifications()),
        ["specforge/graphChanged"]
    );
}

/// P4 (bug D): shutdown ends a listen stream without recording it.
#[test]
fn shutdown_ends_a_listen_stream_without_recording_it() {
    let mut server = alpha();
    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    listen(&mut server, json!(7), &["specforge://diagnostics"]);
    call(&mut server, "shutdown", json!({}));
    assert_eq!(
        events(&server, "mcp_subscription_removed"),
        [json!({"subscriptionType": "specforge/graphChanged", "clientId": "default"})]
    );
    assert_eq!(
        events(&server, "mcp_server_shutdown")[0]["subscriptions_released"],
        1
    );
}

/// P5 (bug E): a resource a listen names twice is heard twice.
#[test]
fn a_resource_a_listen_names_twice_is_heard_twice() {
    let mut server = alpha();
    let ack = listen(
        &mut server,
        json!(3),
        &["specforge://graph", "specforge://graph"],
    );
    assert_eq!(
        ack[0]["params"]["notifications"]["resourceSubscriptions"],
        json!(["specforge://graph", "specforge://graph"])
    );
    change(&mut server);
    let sent = server.take_notifications();
    assert_eq!(sent.len(), 2, "{sent:?}");
    for n in &sent {
        assert_eq!(n["method"], "notifications/resources/updated");
        assert_eq!(n["params"]["uri"], "specforge://graph");
    }
}

/// P6 (bug F): a string listen id is recorded as JSON text.
#[test]
fn a_string_listen_id_is_quoted_in_its_events() {
    let mut server = alpha();
    listen(&mut server, json!("abc"), &["specforge://graph"]);
    assert_eq!(
        events(&server, "mcp_subscription_created"),
        [json!({"subscriptionType": "specforge://graph", "clientId": "\"abc\""})]
    );
}
