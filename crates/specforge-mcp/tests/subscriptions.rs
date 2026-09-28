use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_mcp::subscriptions;
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
#[test]
#[specforge_test(
    behavior = "mcp_subscription_cleanup",
    verify = "subscribe adds subscription"
)]
fn subscribe_adds_subscription() {
    let mut server = init_server();
    let added = subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    assert!(added);

    let subs = subscriptions::subscribers(server.state(), "specforge/graphChanged");
    assert_eq!(subs, vec!["client1"]);
}

// B:mcp_subscription_cleanup — verify unit "duplicate subscribe returns false"
#[test]
#[specforge_test(
    behavior = "mcp_subscription_cleanup",
    verify = "duplicate subscribe returns false"
)]
fn duplicate_subscribe_returns_false() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    let added = subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    assert!(!added);
}

// B:mcp_subscription_cleanup — verify unit "unsubscribe removes subscription"
#[test]
#[specforge_test(
    behavior = "mcp_subscription_cleanup",
    verify = "unsubscribe removes subscription"
)]
fn unsubscribe_removes_subscription() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    let removed =
        subscriptions::unsubscribe(server.state_mut(), "client1", "specforge/graphChanged");
    assert!(removed);

    let subs = subscriptions::subscribers(server.state(), "specforge/graphChanged");
    assert!(subs.is_empty());
}

// B:mcp_subscription_cleanup — verify unit "unsubscribe_all removes all for client"
#[test]
#[specforge_test(
    behavior = "mcp_subscription_cleanup",
    verify = "client disconnect removes all subscriptions for that client"
)]
fn unsubscribe_all_removes_all() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    subscriptions::subscribe(
        server.state_mut(),
        "client1",
        "specforge/diagnosticsChanged",
    );
    subscriptions::unsubscribe_all(server.state_mut(), "client1");

    assert!(subscriptions::subscribers(server.state(), "specforge/graphChanged").is_empty());
    assert!(subscriptions::subscribers(server.state(), "specforge/diagnosticsChanged").is_empty());
}

// B:mcp_subscription_cleanup — verify unit "shutdown clears all subscriptions"
#[test]
#[specforge_test(
    behavior = "mcp_subscription_cleanup",
    verify = "shutdown clears all subscriptions"
)]
fn shutdown_clears_subscriptions() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");

    let req = json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}});
    server.handle_message(&req.to_string());

    assert!(server.state().subscriptions.is_empty());
}

// B:mcp_subscription_cleanup — verify unit "rapid connect/disconnect cycles leave zero subscriptions"
#[test]
#[specforge_test(
    behavior = "mcp_subscription_cleanup",
    verify = "rapid connect/disconnect cycles leave zero subscriptions"
)]
fn rapid_connect_disconnect_zero_subscriptions() {
    let mut server = init_server();
    for i in 0..10 {
        let client = format!("rapid_client_{}", i);
        subscriptions::subscribe(server.state_mut(), &client, "specforge/graphChanged");
        subscriptions::subscribe(server.state_mut(), &client, "specforge/diagnosticsChanged");
        subscriptions::unsubscribe_all(server.state_mut(), &client);
    }
    assert!(subscriptions::subscribers(server.state(), "specforge/graphChanged").is_empty());
    assert!(subscriptions::subscribers(server.state(), "specforge/diagnosticsChanged").is_empty());
}

// ---- C9-01: subscribe → recompile → notification loop ----

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
    serde_json::from_str(&server.handle_message(&req.to_string()).unwrap()).unwrap()
}

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

/// Simulate watch having picked up new content: write an extra entity and a
/// newer `.specforge/graph.json` snapshot marker (C9-07 staleness signal).
fn evolve_project(root: &TempDir, entity: &str) {
    fs::write(
        root.path().join("spec").join(format!("{}.spec", entity)),
        format!(
            "feature {entity} \"{}\" {{\n    behaviors [base]\n}}\n",
            entity
        ),
    )
    .unwrap();
    let marker_dir = root.path().join(".specforge");
    fs::create_dir_all(&marker_dir).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    fs::write(marker_dir.join("graph.json"), "{}").unwrap();
}

/// C9-01 acceptance: subscribe → recompile with changed content →
/// notification delivered to the captured client sink.
#[test]
fn subscribe_recompile_delivers_graph_notification() {
    let dir = project();
    let mut server = init_with_project(&dir);
    assert!(server.state().graph.node_count() > 0);

    let resp = call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    assert!(resp["result"].is_object(), "subscribe must succeed: {resp}");

    evolve_project(&dir, "fresh_added");
    // Any routed resources/read refreshes a stale graph.
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
        !specforge_mcp::subscriptions::subscribers(server.state(), "specforge/diagnosticsChanged")
            .is_empty()
    );
    assert!(
        specforge_mcp::subscriptions::subscribers(server.state(), "specforge/graphChanged")
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
