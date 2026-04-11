#![allow(clippy::result_large_err)]

pub mod error_codes;
pub mod router;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }

    pub fn error_with_data(id: Option<Value>, code: i64, message: impl Into<String>, data: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: Some(data),
            }),
        }
    }
}

pub fn parse_request(input: &str) -> Result<JsonRpcRequest, JsonRpcResponse> {
    let value: Value = serde_json::from_str(input).map_err(|_| {
        JsonRpcResponse::error(None, error_codes::PARSE_ERROR, "Parse error")
    })?;

    // Validate jsonrpc field
    if value.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        let id = value.get("id").cloned();
        return Err(JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Invalid Request: missing or invalid jsonrpc version"));
    }

    // Validate method field
    if value.get("method").and_then(|v| v.as_str()).is_none() {
        let id = value.get("id").cloned();
        return Err(JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Invalid Request: missing method"));
    }

    serde_json::from_value(value).map_err(|e| {
        JsonRpcResponse::error(None, error_codes::INVALID_REQUEST, format!("Invalid Request: {}", e))
    })
}
