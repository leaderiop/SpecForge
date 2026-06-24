use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[host_fn]
extern "ExtismHost" {
    fn host_emit_diagnostic(input: Vec<u8>) -> Vec<u8>;
    fn host_read_file(input: Vec<u8>) -> Vec<u8>;
    fn host_query_graph(input: Vec<u8>) -> Vec<u8>;
}

#[derive(Deserialize)]
struct HandshakeRequest {
    #[allow(dead_code)]
    host_version: String,
    #[allow(dead_code)]
    supported_categories: Vec<String>,
}

#[derive(Serialize)]
struct HandshakeResponse {
    protocol_version: String,
    name: String,
    version: String,
    contribution_flags: ContributionFlags,
    peer_dependencies: Vec<serde_json::Value>,
    sandbox_policy: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ContributionFlags {
    entities: bool,
    validators: bool,
    renderers: bool,
    providers: bool,
    collectors: bool,
    prompts: bool,
    parsers: bool,
    grammars: bool,
    body_parsers: bool,
}

#[derive(Deserialize)]
struct DescribeRequest {
    category: String,
}

#[derive(Serialize)]
struct DescribeResponse {
    category: String,
    items: serde_json::Value,
}

#[plugin_fn]
pub fn __handshake(_input: Vec<u8>) -> FnResult<Vec<u8>> {
    let response = HandshakeResponse {
        protocol_version: "1.0.0".to_string(),
        name: "@test/hello".to_string(),
        version: "0.1.0".to_string(),
        contribution_flags: ContributionFlags {
            entities: true,
            validators: false,
            renderers: false,
            providers: false,
            collectors: false,
            prompts: false,
            parsers: false,
            grammars: false,
            body_parsers: false,
        },
        peer_dependencies: vec![],
        sandbox_policy: None,
    };
    Ok(serde_json::to_vec(&response)?)
}

#[plugin_fn]
pub fn __describe(input: Vec<u8>) -> FnResult<Vec<u8>> {
    let request: DescribeRequest = serde_json::from_slice(&input)?;
    let items = match request.category.as_str() {
        "entities" => serde_json::json!([{
            "name": "widget",
            "fields": [
                {"name": "title", "field_type": "string", "required": true},
                {"name": "status", "field_type": "string", "required": false}
            ],
            "testable": false,
            "singleton": false,
            "supports_verify": false
        }]),
        _ => serde_json::json!([]),
    };
    let response = DescribeResponse {
        category: request.category,
        items,
    };
    Ok(serde_json::to_vec(&response)?)
}

#[plugin_fn]
pub fn test_emit_diagnostic(_input: Vec<u8>) -> FnResult<Vec<u8>> {
    let diag = serde_json::json!({
        "code": "W100",
        "severity": "Warning",
        "message": "test warning from wasm",
        "span": null,
        "suggestion": null
    });
    let diag_bytes = serde_json::to_vec(&diag)?;
    let _response = unsafe { host_emit_diagnostic(diag_bytes)? };
    Ok(serde_json::to_vec(&serde_json::json!({"ok": true}))?)
}

#[plugin_fn]
pub fn test_emit_malformed_diagnostic(_input: Vec<u8>) -> FnResult<Vec<u8>> {
    let bad_bytes = b"not valid json".to_vec();
    let _response = unsafe { host_emit_diagnostic(bad_bytes)? };
    Ok(serde_json::to_vec(&serde_json::json!({"ok": true}))?)
}

#[plugin_fn]
pub fn test_read_file(input: Vec<u8>) -> FnResult<Vec<u8>> {
    let response = unsafe { host_read_file(input)? };
    Ok(response)
}

#[plugin_fn]
pub fn test_query_graph(_input: Vec<u8>) -> FnResult<Vec<u8>> {
    let response = unsafe { host_query_graph(vec![])? };
    Ok(response)
}
