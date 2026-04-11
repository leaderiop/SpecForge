#[path = "e2e_support/lifecycle.rs"]
mod lifecycle;
#[path = "e2e_support/navigation.rs"]
mod navigation;
#[path = "e2e_support/hover.rs"]
mod hover;
#[path = "e2e_support/completion.rs"]
mod completion;
#[path = "e2e_support/symbols.rs"]
mod symbols;
#[path = "e2e_support/editing.rs"]
mod editing;
#[path = "e2e_support/code_actions.rs"]
mod code_actions;
#[path = "e2e_support/semantic_tokens.rs"]
mod semantic_tokens;
#[path = "e2e_support/formatting.rs"]
mod formatting;
#[path = "e2e_support/integration.rs"]
mod integration;

use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tower_lsp::{LspService, Server};

use specforge_lsp::backend::Backend;

/// An in-process LSP client that communicates via JSON-RPC over memory streams.
pub struct LspClient {
    writer: DuplexStream,
    reader: DuplexStream,
    next_id: Arc<Mutex<i64>>,
    server_task: JoinHandle<()>,
}

impl Drop for LspClient {
    fn drop(&mut self) {
        self.server_task.abort();
    }
}

impl LspClient {
    async fn write_message(&mut self, msg: &Value) {
        let body = serde_json::to_string(msg).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.writer.write_all(header.as_bytes()).await.unwrap();
        self.writer.write_all(body.as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();
    }

    async fn read_message(&mut self) -> Value {
        let mut header = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            self.reader.read_exact(&mut byte).await.unwrap();
            header.push(byte[0]);
            if header.len() >= 4 && &header[header.len() - 4..] == b"\r\n\r\n" {
                break;
            }
        }
        let header_str = String::from_utf8(header).unwrap();
        let content_length: usize = header_str
            .lines()
            .find(|l| l.starts_with("Content-Length:"))
            .unwrap()
            .split(':')
            .nth(1)
            .unwrap()
            .trim()
            .parse()
            .unwrap();

        let mut body = vec![0u8; content_length];
        self.reader.read_exact(&mut body).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    async fn send_request(&mut self, method: &str, params: Value) -> Value {
        let id = {
            let mut next = self.next_id.lock().await;
            let id = *next;
            *next += 1;
            id
        };
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        if !params.is_null() {
            msg["params"] = params;
        }
        self.write_message(&msg).await;
        self.read_response_with_id(id).await
    }

    async fn send_notification(&mut self, method: &str, params: Value) {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_message(&msg).await;
    }

    /// If msg is a server→client request (has method + id), send a success response.
    /// Returns true if it was a server request (and was responded to).
    async fn auto_respond_if_server_request(&mut self, msg: &Value) -> bool {
        if msg.get("method").is_some() && msg.get("id").is_some() {
            let req_id = msg["id"].clone();
            self.write_message(&json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": null,
            }))
            .await;
            true
        } else {
            false
        }
    }

    async fn read_response_with_id(&mut self, id: i64) -> Value {
        loop {
            let msg = self.read_message().await;
            self.auto_respond_if_server_request(&msg).await;
            if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
                return msg;
            }
        }
    }

    async fn wait_for_notification(&mut self, method: &str, timeout_ms: u64) -> Option<Value> {
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match tokio::time::timeout(remaining, self.read_message()).await {
                Ok(msg) => {
                    // Auto-respond to server→client requests, but still
                    // check if the method matches what we're waiting for.
                    self.auto_respond_if_server_request(&msg).await;
                    let method_match = msg
                        .get("method")
                        .and_then(|v: &Value| v.as_str())
                        .map(|m| m == method)
                        .unwrap_or(false);
                    if method_match {
                        return Some(msg);
                    }
                }
                Err(_) => return None,
            }
        }
    }

    pub async fn initialize(&mut self, root_uri: Option<&str>) -> Value {
        let root = root_uri.map(|r| {
            tower_lsp::lsp_types::Url::from_file_path(r)
                .unwrap()
                .to_string()
        });
        let params = json!({
            "processId": null,
            "rootUri": root,
            "capabilities": {},
        });
        self.send_request("initialize", params).await
    }

    pub async fn initialized(&mut self) {
        self.send_notification("initialized", json!({})).await;
    }

    pub async fn shutdown(&mut self) -> Value {
        self.send_request("shutdown", json!(null)).await
    }

    pub async fn did_open(&mut self, uri: &str, language_id: &str, text: &str) {
        self.send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                }
            }),
        )
        .await;
    }

    pub async fn did_change(&mut self, uri: &str, version: i32, changes: Vec<Value>) {
        self.send_notification(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": uri,
                    "version": version,
                },
                "contentChanges": changes,
            }),
        )
        .await;
    }

    pub async fn did_close(&mut self, uri: &str) {
        self.send_notification(
            "textDocument/didClose",
            json!({
                "textDocument": { "uri": uri }
            }),
        )
        .await;
    }

    pub async fn hover(&mut self, uri: &str, line: u32, character: u32) -> Value {
        self.send_request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        )
        .await
    }

    pub async fn goto_definition(&mut self, uri: &str, line: u32, character: u32) -> Value {
        self.send_request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        )
        .await
    }

    pub async fn completion(&mut self, uri: &str, line: u32, character: u32) -> Value {
        self.send_request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        )
        .await
    }

    pub async fn references(&mut self, uri: &str, line: u32, character: u32) -> Value {
        self.send_request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": true },
            }),
        )
        .await
    }

    pub async fn prepare_rename(&mut self, uri: &str, line: u32, character: u32) -> Value {
        self.send_request(
            "textDocument/prepareRename",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        )
        .await
    }

    pub async fn rename(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Value {
        self.send_request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "newName": new_name,
            }),
        )
        .await
    }

    pub async fn code_action(
        &mut self,
        uri: &str,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    ) -> Value {
        self.send_request(
            "textDocument/codeAction",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": start_line, "character": start_char },
                    "end": { "line": end_line, "character": end_char },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .await
    }

    pub async fn document_symbol(&mut self, uri: &str) -> Value {
        self.send_request(
            "textDocument/documentSymbol",
            json!({
                "textDocument": { "uri": uri },
            }),
        )
        .await
    }

    pub async fn workspace_symbol(&mut self, query: &str) -> Value {
        self.send_request("workspace/symbol", json!({ "query": query }))
            .await
    }

    pub async fn semantic_tokens_full(&mut self, uri: &str) -> Value {
        self.send_request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": { "uri": uri },
            }),
        )
        .await
    }

    pub async fn formatting(&mut self, uri: &str, tab_size: u32) -> Value {
        self.send_request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": uri },
                "options": { "tabSize": tab_size, "insertSpaces": true },
            }),
        )
        .await
    }

    pub async fn range_formatting(
        &mut self,
        uri: &str,
        tab_size: u32,
        start_line: u32,
        end_line: u32,
    ) -> Value {
        self.send_request(
            "textDocument/rangeFormatting",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": start_line, "character": 0 },
                    "end": { "line": end_line, "character": 0 },
                },
                "options": { "tabSize": tab_size, "insertSpaces": true },
            }),
        )
        .await
    }
}

pub async fn start_server(root_uri: Option<&str>) -> LspClient {
    let (client_to_server, server_stdin) = tokio::io::duplex(1024 * 64);
    let (server_stdout, server_to_client) = tokio::io::duplex(1024 * 64);

    let (service, socket) = LspService::new(Backend::new);

    let server_task = tokio::spawn(async move {
        Server::new(server_stdin, server_stdout, socket)
            .serve(service)
            .await;
    });

    let mut client = LspClient {
        writer: client_to_server,
        reader: server_to_client,
        next_id: Arc::new(Mutex::new(1)),
        server_task,
    };

    client.initialize(root_uri).await;
    client.initialized().await;

    client
}

pub async fn start_server_with_doc(
    root_uri: Option<&str>,
    file_name: &str,
    text: &str,
) -> (LspClient, String) {
    let mut client = start_server(root_uri).await;
    let uri = format!("file:///test/{file_name}");
    client.did_open(&uri, "specforge", text).await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;
    (client, uri)
}
