use specforge_mcp::McpServer;
use specforge_registry::ManifestV2;
use serde_json::{json, Value};
use tempfile::TempDir;

fn init_server(project_dir: &std::path::Path) -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());
    server.state_mut().project_root = Some(project_dir.to_path_buf());
    server.state_mut().manifests = vec![rust_manifest(), typescript_manifest()];
    server
}

fn rust_manifest() -> ManifestV2 {
    ManifestV2 {
        name: "@specforge/rust".into(),
        version: "1.0.0".into(),
        manifest_version: 2,
        wasm_path: String::new(),
        contributes: specforge_registry::ExtensionContributions::default(),
        entity_kinds: vec![],
        edge_types: vec![],
        validation_rules: vec![],
        verify_kinds: vec![],
        fields: vec![],
        incremental: None,
        reserved_keywords: vec![],
        migration_hook: None,
        peer_dependencies: vec![],
        sandbox_policy: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        grammar_contributions: vec![],
        body_parser_contributions: vec![],
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        analyzer_contributions: vec![specforge_registry::AnalyzerContribution {
            language: "rust".into(),
            file_extensions: vec![".rs".into()],
            excluded_dirs: vec!["target".into()],
            scan_export: "scan__rust".into(),
            classify_export: "classify__rust".into(),
            map_export: "map__rust".into(),
            description: None,
        }],
        surfaces: None,
    }
}

fn typescript_manifest() -> ManifestV2 {
    ManifestV2 {
        name: "@specforge/typescript".into(),
        version: "1.0.0".into(),
        manifest_version: 2,
        wasm_path: String::new(),
        contributes: specforge_registry::ExtensionContributions::default(),
        entity_kinds: vec![],
        edge_types: vec![],
        validation_rules: vec![],
        verify_kinds: vec![],
        fields: vec![],
        incremental: None,
        reserved_keywords: vec![],
        migration_hook: None,
        peer_dependencies: vec![],
        sandbox_policy: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        grammar_contributions: vec![],
        body_parser_contributions: vec![],
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        analyzer_contributions: vec![specforge_registry::AnalyzerContribution {
            language: "typescript".into(),
            file_extensions: vec![".ts".into(), ".tsx".into(), ".js".into(), ".jsx".into()],
            excluded_dirs: vec!["node_modules".into(), "dist".into()],
            scan_export: "scan__typescript".into(),
            classify_export: "classify__typescript".into(),
            map_export: "map__typescript".into(),
            description: None,
        }],
        surfaces: None,
    }
}

fn call_tool(server: &mut McpServer, tool_name: &str, args: Value) -> Value {
    let req = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": tool_name, "arguments": args }
    });
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"].as_str().unwrap().to_string()
}

fn setup_project_with_sources(dir: &std::path::Path) {
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src.join("lib.rs"), "pub fn hello() {}").unwrap();
}

// --- specforge.infer_progress ---

#[test]
fn infer_progress_returns_summary_for_empty_project() {
    let tmp = TempDir::new().unwrap();
    let mut server = init_server(tmp.path());

    let resp = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["summary"]["files_total"], 0);
    assert_eq!(parsed["summary"]["files_analyzed"], 0);
    assert_eq!(parsed["summary"]["entities_produced"], 0);
}

#[test]
fn infer_progress_discovers_source_files() {
    let tmp = TempDir::new().unwrap();
    setup_project_with_sources(tmp.path());

    // Write a manifest with source_roots pointing to src/
    let manifest = json!({
        "version": 1,
        "source_roots": ["src"],
        "source_index": []
    });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());
    let resp = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["summary"]["files_total"], 2);
    assert_eq!(parsed["summary"]["files_analyzed"], 0);
    let unanalyzed = parsed["unanalyzed"].as_array().unwrap();
    assert_eq!(unanalyzed.len(), 2);
}

#[test]
fn infer_progress_shows_analyzed_file() {
    let tmp = TempDir::new().unwrap();
    setup_project_with_sources(tmp.path());

    let manifest = json!({
        "version": 1,
        "source_roots": ["src"],
        "source_index": [{
            "path": "src/main.rs",
            "content_hash": "abc123",
            "entities_produced": ["app_entrypoint"],
            "analyzed_at": "1700000000Z"
        }]
    });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());
    let resp = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["summary"]["files_total"], 2);
    assert_eq!(parsed["summary"]["files_analyzed"], 1);
    assert_eq!(parsed["summary"]["entities_produced"], 1);
    let unanalyzed = parsed["unanalyzed"].as_array().unwrap();
    assert_eq!(unanalyzed.len(), 1);
    assert_eq!(unanalyzed[0], "src/lib.rs");
}

// --- specforge.infer_session ---

#[test]
fn infer_session_start_creates_active_session() {
    let tmp = TempDir::new().unwrap();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());
    let resp = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "start",
        "agent": "claude"
    }));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["status"], "active");
    assert!(parsed["session_id"].as_str().unwrap().starts_with("sess_"));
}

#[test]
fn infer_session_rejects_second_active() {
    let tmp = TempDir::new().unwrap();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());

    // First session succeeds
    let resp1 = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "start",
        "agent": "claude"
    }));
    assert!(resp1.get("error").is_none());

    // Second session fails
    let resp2 = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "start",
        "agent": "claude"
    }));
    assert!(resp2.get("error").is_some() || {
        let text = tool_text(&resp2);
        text.contains("already active")
    });
}

#[test]
fn infer_session_mark_analyzed_records_entry() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();

    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());

    let resp = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "mark_analyzed",
        "source_file": "src/main.rs",
        "entities_produced": ["app_main"]
    }));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["status"], "recorded");
    assert_eq!(parsed["source_file"], "src/main.rs");

    // Verify it shows up in progress
    let progress = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let ptext = tool_text(&progress);
    let pparsed: Value = serde_json::from_str(&ptext).unwrap();
    assert_eq!(pparsed["summary"]["files_analyzed"], 1);
    assert_eq!(pparsed["summary"]["entities_produced"], 1);
}

#[test]
fn infer_session_end_completes_session() {
    let tmp = TempDir::new().unwrap();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());

    // Start session
    let start_resp = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "start",
        "agent": "claude"
    }));
    let start_text = tool_text(&start_resp);
    let start_parsed: Value = serde_json::from_str(&start_text).unwrap();
    let session_id = start_parsed["session_id"].as_str().unwrap().to_string();

    // End session
    let end_resp = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "end",
        "session_id": session_id,
        "status": "completed"
    }));
    let end_text = tool_text(&end_resp);
    let end_parsed: Value = serde_json::from_str(&end_text).unwrap();
    assert_eq!(end_parsed["status"], "completed");

    // Can start a new session now
    let new_resp = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "start",
        "agent": "claude"
    }));
    assert!(new_resp.get("error").is_none());
}

#[test]
fn infer_session_end_rejects_unknown_session() {
    let tmp = TempDir::new().unwrap();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());

    let resp = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "end",
        "session_id": "sess_nonexistent"
    }));
    assert!(resp.get("error").is_some());
}

#[test]
fn infer_session_missing_action_returns_error() {
    let tmp = TempDir::new().unwrap();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());

    let resp = call_tool(&mut server, "specforge.infer_session", json!({}));
    assert!(resp.get("error").is_some());
}

#[test]
fn infer_session_full_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src.join("lib.rs"), "pub fn hello() {}").unwrap();

    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());

    // Check initial progress
    let p0 = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let p0_parsed: Value = serde_json::from_str(&tool_text(&p0)).unwrap();
    assert_eq!(p0_parsed["summary"]["files_total"], 2);
    assert_eq!(p0_parsed["summary"]["files_analyzed"], 0);

    // Start session
    let start = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "start",
        "agent": "claude",
        "source_roots": ["src"]
    }));
    let start_parsed: Value = serde_json::from_str(&tool_text(&start)).unwrap();
    let sid = start_parsed["session_id"].as_str().unwrap().to_string();

    // Mark first file
    call_tool(&mut server, "specforge.infer_session", json!({
        "action": "mark_analyzed",
        "source_file": "src/main.rs",
        "entities_produced": ["app_main"]
    }));

    // Check mid-progress
    let p1 = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let p1_parsed: Value = serde_json::from_str(&tool_text(&p1)).unwrap();
    assert_eq!(p1_parsed["summary"]["files_analyzed"], 1);
    assert_eq!(p1_parsed["unanalyzed"].as_array().unwrap().len(), 1);

    // Mark second file
    call_tool(&mut server, "specforge.infer_session", json!({
        "action": "mark_analyzed",
        "source_file": "src/lib.rs",
        "entities_produced": ["hello_behavior", "hello_type"]
    }));

    // Check final progress
    let p2 = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let p2_parsed: Value = serde_json::from_str(&tool_text(&p2)).unwrap();
    assert_eq!(p2_parsed["summary"]["files_total"], 2);
    assert_eq!(p2_parsed["summary"]["files_analyzed"], 2);
    assert_eq!(p2_parsed["summary"]["entities_produced"], 3);
    assert_eq!(p2_parsed["unanalyzed"].as_array().unwrap().len(), 0);

    // End session
    let end = call_tool(&mut server, "specforge.infer_session", json!({
        "action": "end",
        "session_id": sid,
        "status": "completed"
    }));
    let end_parsed: Value = serde_json::from_str(&tool_text(&end)).unwrap();
    assert_eq!(end_parsed["status"], "completed");
}

// --- Mixed language discovery ---

#[test]
fn infer_progress_discovers_both_rust_and_typescript() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("lib.rs"), "pub fn hello() {}").unwrap();
    std::fs::write(src.join("app.ts"), "export function handleRequest() {}").unwrap();
    std::fs::write(src.join("readme.md"), "# Hello").unwrap();

    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.path().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ).unwrap();

    let mut server = init_server(tmp.path());
    let resp = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["summary"]["files_total"], 2);
    let unanalyzed = parsed["unanalyzed"].as_array().unwrap();
    assert_eq!(unanalyzed.len(), 2);
    let files: Vec<&str> = unanalyzed.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(files.contains(&"src/lib.rs"));
    assert!(files.contains(&"src/app.ts"));
}
