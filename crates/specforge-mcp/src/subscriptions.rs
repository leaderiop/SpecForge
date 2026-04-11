use crate::state::{McpState, Subscription};

pub fn subscribe(state: &mut McpState, client_id: &str, channel: &str) -> bool {
    let subs = state.subscriptions.entry(channel.to_string()).or_default();

    // Don't duplicate
    if subs.iter().any(|s| s.client_id == client_id) {
        return false;
    }

    subs.push(Subscription {
        client_id: client_id.to_string(),
        channel: channel.to_string(),
    });
    state.push_event("mcp_subscription_created", serde_json::json!({"client_id": client_id, "channel": channel}));
    true
}

pub fn unsubscribe(state: &mut McpState, client_id: &str, channel: &str) -> bool {
    if let Some(subs) = state.subscriptions.get_mut(channel) {
        let before = subs.len();
        subs.retain(|s| s.client_id != client_id);
        if subs.len() < before {
            state.push_event("mcp_subscription_removed", serde_json::json!({"client_id": client_id, "channel": channel}));
            return true;
        }
    }
    false
}

pub fn unsubscribe_all(state: &mut McpState, client_id: &str) {
    for subs in state.subscriptions.values_mut() {
        subs.retain(|s| s.client_id != client_id);
    }
}

pub fn subscribers<'a>(state: &'a McpState, channel: &str) -> Vec<&'a str> {
    state.subscriptions.get(channel)
        .map(|subs| subs.iter().map(|s| s.client_id.as_str()).collect())
        .unwrap_or_default()
}
