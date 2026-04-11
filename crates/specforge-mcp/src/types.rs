use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEvent {
    pub name: String,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapabilities {
    pub tools: Vec<McpToolDescriptor>,
    pub resources: Vec<McpResourceDescriptor>,
    pub prompts: Vec<McpPromptDescriptor>,
    pub subscriptions: bool,
    pub server_name: String,
    pub server_version: String,
    pub graph_protocol_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceDescriptor {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptDescriptor {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<McpPromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}
