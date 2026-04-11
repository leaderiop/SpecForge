use specforge_mcp::McpServer;
use specforge_mcp::subscriptions;
use specforge_test::prelude::*;
use serde_json::json;

fn init_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());
    server
}

// B:mcp_subscription_cleanup — verify unit "subscribe adds subscription"
#[test]
#[specforge_test(behavior = "mcp_subscription_cleanup", verify = "subscribe adds subscription")]
fn subscribe_adds_subscription() {
    let mut server = init_server();
    let added = subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    assert!(added);

    let subs = subscriptions::subscribers(server.state(), "specforge/graphChanged");
    assert_eq!(subs, vec!["client1"]);
}

// B:mcp_subscription_cleanup — verify unit "duplicate subscribe returns false"
#[test]
#[specforge_test(behavior = "mcp_subscription_cleanup", verify = "duplicate subscribe returns false")]
fn duplicate_subscribe_returns_false() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    let added = subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    assert!(!added);
}

// B:mcp_subscription_cleanup — verify unit "unsubscribe removes subscription"
#[test]
#[specforge_test(behavior = "mcp_subscription_cleanup", verify = "unsubscribe removes subscription")]
fn unsubscribe_removes_subscription() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    let removed = subscriptions::unsubscribe(server.state_mut(), "client1", "specforge/graphChanged");
    assert!(removed);

    let subs = subscriptions::subscribers(server.state(), "specforge/graphChanged");
    assert!(subs.is_empty());
}

// B:mcp_subscription_cleanup — verify unit "unsubscribe_all removes all for client"
#[test]
#[specforge_test(behavior = "mcp_subscription_cleanup", verify = "client disconnect removes all subscriptions for that client")]
fn unsubscribe_all_removes_all() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/diagnosticsChanged");
    subscriptions::unsubscribe_all(server.state_mut(), "client1");

    assert!(subscriptions::subscribers(server.state(), "specforge/graphChanged").is_empty());
    assert!(subscriptions::subscribers(server.state(), "specforge/diagnosticsChanged").is_empty());
}

// B:mcp_subscription_cleanup — verify unit "shutdown clears all subscriptions"
#[test]
#[specforge_test(behavior = "mcp_subscription_cleanup", verify = "shutdown clears all subscriptions")]
fn shutdown_clears_subscriptions() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");

    let req = json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}});
    server.handle_message(&req.to_string());

    assert!(server.state().subscriptions.is_empty());
}

// B:mcp_subscription_cleanup — verify unit "rapid connect/disconnect cycles leave zero subscriptions"
#[test]
#[specforge_test(behavior = "mcp_subscription_cleanup", verify = "rapid connect/disconnect cycles leave zero subscriptions")]
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
