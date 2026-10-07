use crate::support::*;
use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_test::prelude::*;

/// An initialized server over `project`, which enables the rust and
/// typescript analyzers, in that order, both served in process: their
/// scanners find nothing.
fn init_server(project: TestProject) -> Served {
    project.serve(&[
        TestExtension::named("@specforge/rust").declaring(rust),
        TestExtension::named("@specforge/typescript").declaring(typescript),
    ])
}

/// The rust analyzer, declared as the builtin declares it.
fn rust(c: &mut ContributionsBuilder) {
    c.analyzer("rust", |a| {
        a.file_extensions(&[".rs"])
            .excluded_dirs(&["target"])
            .scan(|_| ScanResponse {
                items: Vec::new(),
                language: None,
            });
    });
}

/// The typescript analyzer, declared as the builtin declares it.
fn typescript(c: &mut ContributionsBuilder) {
    c.analyzer("typescript", |a| {
        a.file_extensions(&[".ts", ".tsx", ".js", ".jsx"])
            .excluded_dirs(&["node_modules", "dist"])
            .scan(|_| ScanResponse {
                items: Vec::new(),
                language: None,
            });
    });
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
    let tmp = TestProject::new();
    let mut server = init_server(tmp);

    let resp = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["summary"]["files_total"], 0);
    assert_eq!(parsed["summary"]["files_analyzed"], 0);
    assert_eq!(parsed["summary"]["entities_produced"], 0);
}

#[test]
fn infer_progress_discovers_source_files() {
    let tmp = TestProject::new();
    setup_project_with_sources(tmp.root());

    // Write a manifest with source_roots pointing to src/
    let manifest = json!({
        "version": 1,
        "source_roots": ["src"],
        "source_index": []
    });
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);
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
    let tmp = TestProject::new();
    setup_project_with_sources(tmp.root());

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
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);
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
    let tmp = TestProject::new();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);
    let resp = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "start",
            "agent": "claude"
        }),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["status"], "active");
    assert!(parsed["session_id"].is_string(), "{parsed}");
}

#[test]
fn infer_session_rejects_second_active() {
    let tmp = TestProject::new();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);

    // First session succeeds
    let resp1 = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "start",
            "agent": "claude"
        }),
    );
    assert!(resp1.get("error").is_none());

    // Second session fails
    let resp2 = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "start",
            "agent": "claude"
        }),
    );
    assert!(
        resp2.get("error").is_some() || {
            let text = tool_text(&resp2);
            text.contains("already active")
        }
    );
}

#[test]
fn infer_session_mark_analyzed_records_entry() {
    let tmp = TestProject::new();
    let src = tmp.root().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();

    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);

    let resp = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "mark_analyzed",
            "source_file": "src/main.rs",
            "entities_produced": ["app_main"]
        }),
    );
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
    let tmp = TestProject::new();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);

    // Start session
    let start_resp = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "start",
            "agent": "claude"
        }),
    );
    let start_text = tool_text(&start_resp);
    let start_parsed: Value = serde_json::from_str(&start_text).unwrap();
    let session_id = start_parsed["session_id"].as_str().unwrap().to_string();

    // End session
    let end_resp = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "end",
            "session_id": session_id,
            "status": "completed"
        }),
    );
    let end_text = tool_text(&end_resp);
    let end_parsed: Value = serde_json::from_str(&end_text).unwrap();
    assert_eq!(end_parsed["status"], "completed");

    // Can start a new session now
    let new_resp = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "start",
            "agent": "claude"
        }),
    );
    assert!(new_resp.get("error").is_none());
}

#[test]
fn infer_session_end_rejects_unknown_session() {
    let tmp = TestProject::new();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);

    let resp = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "end",
            "session_id": "sess_nonexistent"
        }),
    );
    // unknown session is a tool execution error (isError result), not a
    // protocol error
    assert!(resp["result"]["isError"] == true);
}

#[test]
fn infer_session_missing_action_returns_error() {
    let tmp = TestProject::new();
    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);

    let resp = call_tool(&mut server, "specforge.infer_session", json!({}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "invalid_input", "{error}");
    assert_eq!(error["argument"], "action", "{error}");
}

#[test]
fn infer_session_full_lifecycle() {
    let tmp = TestProject::new();
    let src = tmp.root().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src.join("lib.rs"), "pub fn hello() {}").unwrap();

    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);

    // Check initial progress
    let p0 = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let p0_parsed: Value = serde_json::from_str(&tool_text(&p0)).unwrap();
    assert_eq!(p0_parsed["summary"]["files_total"], 2);
    assert_eq!(p0_parsed["summary"]["files_analyzed"], 0);

    // Start session
    let start = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "start",
            "agent": "claude",
            "source_roots": ["src"]
        }),
    );
    let start_parsed: Value = serde_json::from_str(&tool_text(&start)).unwrap();
    let sid = start_parsed["session_id"].as_str().unwrap().to_string();

    // Mark first file
    call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "mark_analyzed",
            "source_file": "src/main.rs",
            "entities_produced": ["app_main"]
        }),
    );

    // Check mid-progress
    let p1 = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let p1_parsed: Value = serde_json::from_str(&tool_text(&p1)).unwrap();
    assert_eq!(p1_parsed["summary"]["files_analyzed"], 1);
    assert_eq!(p1_parsed["unanalyzed"].as_array().unwrap().len(), 1);

    // Mark second file
    call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "mark_analyzed",
            "source_file": "src/lib.rs",
            "entities_produced": ["hello_behavior", "hello_type"]
        }),
    );

    // Check final progress
    let p2 = call_tool(&mut server, "specforge.infer_progress", json!({}));
    let p2_parsed: Value = serde_json::from_str(&tool_text(&p2)).unwrap();
    assert_eq!(p2_parsed["summary"]["files_total"], 2);
    assert_eq!(p2_parsed["summary"]["files_analyzed"], 2);
    assert_eq!(p2_parsed["summary"]["entities_produced"], 3);
    assert_eq!(p2_parsed["unanalyzed"].as_array().unwrap().len(), 0);

    // End session
    let end = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({
            "action": "end",
            "session_id": sid,
            "status": "completed"
        }),
    );
    let end_parsed: Value = serde_json::from_str(&tool_text(&end)).unwrap();
    assert_eq!(end_parsed["status"], "completed");
}

// --- Mixed language discovery ---

#[test]
fn infer_progress_discovers_both_rust_and_typescript() {
    let tmp = TestProject::new();
    let src = tmp.root().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("lib.rs"), "pub fn hello() {}").unwrap();
    std::fs::write(src.join("app.ts"), "export function handleRequest() {}").unwrap();
    std::fs::write(src.join("readme.md"), "# Hello").unwrap();

    let manifest = json!({ "version": 1, "source_roots": ["src"] });
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let mut server = init_server(tmp);
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

/// Whether `id` is a version 4 UUID: 8-4-4-4-12 lowercase hex digits,
/// version nibble 4, variant 8, 9, a or b.
fn is_uuid_v4(id: &str) -> bool {
    let groups: Vec<&str> = id.split('-').collect();
    groups.iter().map(|g| g.len()).eq([8, 4, 4, 4, 12])
        && groups.iter().all(|g| {
            g.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        })
        && groups[2].starts_with('4')
        && groups[3].starts_with(['8', '9', 'a', 'b'])
}

/// The sessions specforge-infer.json records.
fn recorded_sessions(root: &std::path::Path) -> Vec<Value> {
    let text = std::fs::read_to_string(root.join("specforge-infer.json")).unwrap();
    let manifest: Value = serde_json::from_str(&text).unwrap();
    manifest["sessions"].as_array().cloned().unwrap_or_default()
}

#[specforge_test(
    behavior = "start_inference_session",
    verify = "start assigns unique session ID"
)]
fn infer_session_ids_are_unique_uuids() {
    let tmp = TestProject::new();
    let mut server = init_server(tmp);
    let mut ids = Vec::new();
    for _ in 0..2 {
        let started = call_tool(
            &mut server,
            "specforge.infer_session",
            json!({"action": "start", "agent": "claude"}),
        );
        let id = serde_json::from_str::<Value>(&tool_text(&started)).unwrap()["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(is_uuid_v4(&id), "not a UUID: {id}");
        call_tool(
            &mut server,
            "specforge.infer_session",
            json!({"action": "end", "session_id": id}),
        );
        ids.push(id);
    }
    assert_ne!(ids[0], ids[1]);
}

#[specforge_test(
    behavior = "end_inference_session",
    verify = "end sets ended_at timestamp"
)]
fn infer_session_timestamps_are_rfc_3339() {
    let tmp = TestProject::new();
    std::fs::create_dir_all(tmp.root().join("src")).unwrap();
    std::fs::write(tmp.root().join("src/lib.rs"), "pub fn f() {}\n").unwrap();
    let mut server = init_server(tmp);
    let started = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({"action": "start"}),
    );
    let id = serde_json::from_str::<Value>(&tool_text(&started)).unwrap()["session_id"].clone();
    call_tool(
        &mut server,
        "specforge.infer_session",
        json!({"action": "mark_analyzed", "source_file": "src/lib.rs"}),
    );
    call_tool(
        &mut server,
        "specforge.infer_session",
        json!({"action": "end", "session_id": id}),
    );

    let session = recorded_sessions(server.root()).pop().unwrap();
    let text = std::fs::read_to_string(server.root().join("specforge-infer.json")).unwrap();
    let manifest: Value = serde_json::from_str(&text).unwrap();
    let analyzed_at = &manifest["source_index"][0]["analyzed_at"];
    for stamp in [&session["started_at"], &session["ended_at"], analyzed_at] {
        let stamp = stamp
            .as_str()
            .unwrap_or_else(|| panic!("no timestamp: {text}"));
        assert!(
            chrono::DateTime::parse_from_rfc3339(stamp).is_ok(),
            "not RFC 3339: {stamp}"
        );
    }
}
