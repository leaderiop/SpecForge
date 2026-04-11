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
pub mod tools;
pub mod types;

use protocol::{parse_request, JsonRpcResponse};
use protocol::router::route;
use state::McpState;

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

    pub fn handle_message(&mut self, input: &str) -> Option<String> {
        let request = match parse_request(input) {
            Ok(req) => req,
            Err(err_response) => {
                self.state.push_event("mcp_protocol_error_handled", serde_json::json!({"phase": "parse"}));
                return Some(serialize_response(&err_response));
            }
        };

        // Notifications (no id) don't get responses in JSON-RPC
        let is_notification = request.id.is_none();

        let response = route(&mut self.state, &request.method, request.params, request.id);

        if is_notification {
            return None;
        }

        Some(serialize_response(&response))
    }

    pub fn state(&self) -> &McpState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut McpState {
        &mut self.state
    }
}

fn serialize_response(response: &JsonRpcResponse) -> String {
    serde_json::to_string(response).expect("JSON-RPC response serialization cannot fail")
}
