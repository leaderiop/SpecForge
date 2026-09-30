pub mod compile;
pub mod lifecycle;
pub mod notifications;
pub mod operations;
pub mod prompts;
pub mod protocol;
pub mod registry;
pub mod resources;
pub mod state;
pub mod subscriptions;
pub mod tool;
pub mod tools;
pub mod types;

use protocol::router::route;
use protocol::{JsonRpcResponse, parse_request};
use state::McpState;

/// The client a request speaks for when it names no `client_id`: the one
/// peer of a stdio session.
pub const DEFAULT_CLIENT_ID: &str = "default";

pub struct McpServer {
    state: McpState,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            state: McpState::new(),
        }
    }

    /// A server that compiles `root` when the client's `initialize` does not
    /// name a `projectRoot` — standard MCP clients never send one.
    pub fn with_project_root(root: std::path::PathBuf) -> Self {
        let mut server = Self::new();
        server.state.default_project_root = Some(root);
        server
    }

    pub fn handle_message(&mut self, input: &str) -> Option<String> {
        let request = match parse_request(input) {
            Ok(req) => req,
            Err(err_response) => {
                let error = err_response.error.as_ref();
                let code = error.map_or(protocol::error_codes::PARSE_ERROR, |e| e.code);
                let message = error.map_or("Parse error", |e| e.message.as_str());
                self.state.push_event(
                    "mcp_protocol_error_handled",
                    serde_json::json!({"errorCode": code, "errorMessage": message}),
                );
                return Some(serialize_response(&err_response));
            }
        };

        // Notifications (no id) don't get responses in JSON-RPC
        let is_notification = request.id.is_none();

        let method = request.method.clone();
        let response = route(&mut self.state, &request.method, request.params, request.id);
        if let Some(error) = &response.error {
            self.state.push_event(
                "mcp_protocol_error_handled",
                serde_json::json!({
                    "errorCode": error.code,
                    "errorMessage": error.message,
                    "method": method,
                }),
            );
        }

        if is_notification {
            return None;
        }

        Some(serialize_response(&response))
    }

    pub fn state(&self) -> &McpState {
        &self.state
    }

    /// Drain the server→client notification outbox (C9-01): notifications
    /// queued for subscribed channels since the last drain.
    pub fn take_notifications(&mut self) -> Vec<serde_json::Value> {
        notifications::pending_notifications(&mut self.state)
    }

    /// A client went away: drop every subscription it held. Transports call
    /// this when a connection closes (stdio: at end of input).
    pub fn disconnect(&mut self, client_id: &str) {
        subscriptions::unsubscribe_all(&mut self.state, client_id);
    }

    pub fn state_mut(&mut self) -> &mut McpState {
        &mut self.state
    }
}

fn serialize_response(response: &JsonRpcResponse) -> String {
    serde_json::to_string(response).expect("JSON-RPC response serialization cannot fail")
}
