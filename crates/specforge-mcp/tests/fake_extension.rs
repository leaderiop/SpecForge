//! A stand-in Wasm runtime hosting one extension, `@test/cmds`, that speaks
//! the extension protocol (`__handshake`, `__describe`) and contributes
//! surfaces: two CLI commands (`report`, `check`), an explicit MCP tool
//! named `specforge.cmds.check` (the name `check` would be auto-promoted
//! to), and one MCP resource. Every other export call is recorded and
//! answered from the configured outputs, so a test sees which export a tool
//! call reached and with what input.

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub const EXT: &str = "@test/cmds";

/// One recorded export call: `(extension, export, input as JSON)`.
pub type Call = (String, String, Value);

pub struct FakeExtension {
    outputs: HashMap<String, Vec<u8>>,
    /// Exports whose call panics, as a broken host function would.
    panics: Vec<String>,
    calls: Mutex<Vec<Call>>,
    /// The compiler passes `@test/cmds` declares (`__describe passes`).
    passes: Value,
}

impl FakeExtension {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
            panics: Vec::new(),
            calls: Mutex::new(Vec::new()),
            passes: json!([]),
        }
    }

    /// Declare the compiler passes named `names`; each runs as the export
    /// `__pass_<name>`, answered from the configured outputs.
    pub fn with_passes(mut self, names: &[&str]) -> Self {
        self.passes = names.iter().map(|name| json!({"name": name})).collect();
        self
    }

    /// Answer calls to `export` with `output`.
    pub fn with_output(mut self, export: &str, output: Value) -> Self {
        self.outputs
            .insert(export.into(), serde_json::to_vec(&output).unwrap());
        self
    }

    /// Make calls to `export` panic.
    pub fn with_panic(mut self, export: &str) -> Self {
        self.panics.push(export.into());
        self
    }

    /// Every non-protocol export call so far, oldest first.
    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    /// The surfaces `@test/cmds` declares.
    pub fn surfaces() -> Value {
        json!({
            "commands": [
                {
                    "id": "report",
                    "title": "Report",
                    "description": "Write a coverage report",
                    "export": "cmd__report",
                    "args": [
                        {"name": "format", "arg_type": {"enum": {"values": ["md", "json"]}}, "required": true, "description": "Output format"},
                        {"name": "verbose", "arg_type": "bool"},
                        {"name": "limit", "arg_type": "integer"},
                        {"name": "out", "arg_type": "path"}
                    ]
                },
                {
                    "id": "check",
                    "title": "Check",
                    "description": "Check the project",
                    "export": "cmd__check",
                    "args": []
                }
            ],
            "mcp_tools": [
                {
                    "name": "specforge.cmds.check",
                    "description": "Explicit check tool",
                    "export": "mcp__check",
                    "input_schema": {"type": "object", "properties": {"strict": {"type": "boolean"}}},
                    "output_schema": {"type": "object", "properties": {"checked": {"type": "boolean"}}}
                }
            ],
            "mcp_resources": [
                {
                    "uri_template": "specforge://ext/cmds/summary",
                    "name": "cmds-summary",
                    "export": "mcp__summary",
                    "mime_type": "application/json"
                }
            ]
        })
    }
}

impl WasmRuntime for FakeExtension {
    fn load_module(&self, _wasm_path: &Path) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult {
        if extension_name != EXT {
            return trap("extension_not_found", export_name);
        }
        match export_name {
            "__handshake" => ok(json!({
                "protocol_version": "1.0.0",
                "name": EXT,
                "version": "0.1.0",
                "contribution_flags": {"entities": true},
                "peer_dependencies": [],
                "sandbox_policy": null,
            })),
            "__describe" => {
                let request: Value = serde_json::from_slice(input).unwrap();
                let category = request["category"].as_str().unwrap().to_string();
                let items = match category.as_str() {
                    "surfaces" => json!([Self::surfaces()]),
                    "passes" => self.passes.clone(),
                    _ => json!([]),
                };
                ok(json!({"category": category, "items": items}))
            }
            export if self.panics.iter().any(|p| p == export) => {
                panic!("{export} panicked")
            }
            export => {
                let input = serde_json::from_slice(input).unwrap_or(Value::Null);
                self.calls
                    .lock()
                    .unwrap()
                    .push((extension_name.into(), export.into(), input));
                match self.outputs.get(export) {
                    Some(output) => WasmCallResult::Ok(output.clone()),
                    None => trap("export_not_found", export),
                }
            }
        }
    }
}

fn ok(value: Value) -> WasmCallResult {
    WasmCallResult::Ok(serde_json::to_vec(&value).unwrap())
}

fn trap(kind: &str, export: &str) -> WasmCallResult {
    WasmCallResult::Trap(WasmTrapInfo {
        kind: kind.into(),
        message: format!("{kind}: {export}"),
        export_name: export.into(),
    })
}

/// A project on disk that enables `@test/cmds`, with one spec file.
pub fn project() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        json!({"name": "cmds", "version": "0.1.0", "extensions": [EXT]}).to_string(),
    )
    .unwrap();
    std::fs::write(dir.path().join("main.spec"), "").unwrap();
    dir
}

/// A server whose extensions run in `ext`; not yet initialized.
pub fn server_with(ext: &Arc<FakeExtension>) -> McpServer {
    let mut server = McpServer::new();
    server.state_mut().extension_runtime = Some(Arc::clone(ext) as Arc<dyn WasmRuntime>);
    server
}

/// A server initialized over [`project`] with `ext` as its runtime.
pub fn initialized(ext: FakeExtension) -> (McpServer, Arc<FakeExtension>, tempfile::TempDir) {
    let ext = Arc::new(ext);
    let dir = project();
    let mut server = server_with(&ext);
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"projectRoot": dir.path().to_str().unwrap()}});
    let resp: Value =
        serde_json::from_str(&server.handle_message(&req.to_string()).unwrap()).unwrap();
    assert!(resp["error"].is_null(), "{resp}");
    (server, ext, dir)
}
