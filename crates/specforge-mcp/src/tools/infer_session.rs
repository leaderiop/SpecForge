use serde_json::{json, Value};

use specforge_common::inference::{
    self, InferenceManifest, SourceFileEntry,
};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceSession {
    pub session_id: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    pub agent: String,
    pub status: String,
}

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let project_root = match &state.project_root {
        Some(p) => p.clone(),
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_REQUEST,
                "No project root available",
            );
        }
    };

    let action = match args.get("action").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Missing required parameter: action (start | mark_analyzed | end)",
            );
        }
    };

    match action {
        "start" => handle_start(state, &args, &project_root, id),
        "mark_analyzed" => handle_mark_analyzed(state, &args, &project_root, id),
        "end" => handle_end(state, &args, &project_root, id),
        _ => JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            format!("Unknown action: '{}'. Expected: start, mark_analyzed, end", action),
        ),
    }
}

fn handle_start(
    _state: &McpState,
    args: &Value,
    project_root: &std::path::Path,
    id: Option<Value>,
) -> JsonRpcResponse {
    let mut manifest = match inference::load_inference_manifest(project_root) {
        Ok(m) => m,
        Err(e) => return JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e),
    };

    let agent = args
        .get("agent")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let source_roots = args
        .get("source_roots")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        });

    if let Some(roots) = source_roots {
        manifest.source_roots = roots;
    }

    let session_id = generate_session_id();
    let now = now_iso8601();

    let session = InferenceSession {
        session_id: session_id.clone(),
        started_at: now,
        ended_at: None,
        agent,
        status: "active".to_string(),
    };

    let sessions_json = read_sessions_from_manifest(project_root);
    if sessions_json.iter().any(|s| s.status == "active") {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_REQUEST,
            "Another inference session is already active. End it first.",
        );
    }

    let mut sessions = sessions_json;
    sessions.push(session);

    if let Err(e) = write_sessions_to_manifest(project_root, &manifest, &sessions) {
        return JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e);
    }

    JsonRpcResponse::success(id, json!({
        "content": [{ "type": "text", "text": json!({
            "session_id": session_id,
            "status": "active"
        }).to_string() }]
    }))
}

fn handle_mark_analyzed(
    _state: &McpState,
    args: &Value,
    project_root: &std::path::Path,
    id: Option<Value>,
) -> JsonRpcResponse {
    let source_file = match args.get("source_file").and_then(|v| v.as_str()) {
        Some(f) => f.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Missing required parameter: source_file",
            );
        }
    };

    let entities: Vec<String> = args
        .get("entities_produced")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let mut manifest = match inference::load_inference_manifest(project_root) {
        Ok(m) => m,
        Err(e) => return JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e),
    };

    let abs_path = project_root.join(&source_file);
    let content_hash = match inference::compute_content_hash(&abs_path) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e),
    };

    manifest.upsert_source_entry(SourceFileEntry {
        path: source_file.clone(),
        content_hash,
        entities_produced: entities.clone(),
        analyzed_at: now_iso8601(),
    });

    let sessions = read_sessions_from_manifest(project_root);
    if let Err(e) = write_sessions_to_manifest(project_root, &manifest, &sessions) {
        return JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e);
    }

    JsonRpcResponse::success(id, json!({
        "content": [{ "type": "text", "text": json!({
            "source_file": source_file,
            "entities_produced": entities,
            "status": "recorded"
        }).to_string() }]
    }))
}

fn handle_end(
    _state: &McpState,
    args: &Value,
    project_root: &std::path::Path,
    id: Option<Value>,
) -> JsonRpcResponse {
    let session_id = match args.get("session_id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Missing required parameter: session_id",
            );
        }
    };

    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("completed")
        .to_string();

    if status != "completed" && status != "paused" {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            format!("Invalid status: '{}'. Expected: completed, paused", status),
        );
    }

    let manifest = match inference::load_inference_manifest(project_root) {
        Ok(m) => m,
        Err(e) => return JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e),
    };

    let mut sessions = read_sessions_from_manifest(project_root);
    let session = sessions.iter_mut().find(|s| s.session_id == session_id);
    match session {
        Some(s) if s.status == "active" => {
            s.status = status.clone();
            s.ended_at = Some(now_iso8601());
        }
        Some(_) => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_REQUEST,
                format!("Session '{}' is not active", session_id),
            );
        }
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                format!("Unknown session_id: '{}'", session_id),
            );
        }
    }

    if let Err(e) = write_sessions_to_manifest(project_root, &manifest, &sessions) {
        return JsonRpcResponse::error(id, error_codes::INTERNAL_ERROR, e);
    }

    JsonRpcResponse::success(id, json!({
        "content": [{ "type": "text", "text": json!({
            "session_id": session_id,
            "status": status
        }).to_string() }]
    }))
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("sess_{:x}", ts)
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    format!("{}Z", secs)
}

fn sessions_path(project_root: &std::path::Path) -> std::path::PathBuf {
    project_root.join("specforge-infer.json")
}

fn read_sessions_from_manifest(project_root: &std::path::Path) -> Vec<InferenceSession> {
    let path = sessions_path(project_root);
    if !path.exists() {
        return Vec::new();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let value: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    value
        .get("sessions")
        .and_then(|v| serde_json::from_value::<Vec<InferenceSession>>(v.clone()).ok())
        .unwrap_or_default()
}

fn write_sessions_to_manifest(
    project_root: &std::path::Path,
    manifest: &InferenceManifest,
    sessions: &[InferenceSession],
) -> Result<(), String> {
    let path = sessions_path(project_root);
    let mut value = serde_json::to_value(manifest).unwrap_or(json!({}));
    if let Value::Object(ref mut map) = value {
        map.insert(
            "sessions".to_string(),
            serde_json::to_value(sessions).unwrap_or(json!([])),
        );
    }
    let json = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| format!("Failed to write: {}", e))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Failed to rename: {}", e))?;
    Ok(())
}
