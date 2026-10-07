use serde_json::{Value, json};

use specforge_common::inference::{self, InferenceManifest, SourceFileEntry};

use specforge_ops::Writes;

use crate::mutation::{Mutated, Written};
use crate::state::McpState;
use crate::target::Call;
use crate::tool::{ErrorCode, McpError, ToolOutcome};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceSession {
    pub session_id: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    pub agent: String,
    pub status: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "crate::args::lenient")]
    action: Option<String>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    agent: Option<String>,
    #[serde(default, deserialize_with = "crate::args::some_strings")]
    source_roots: Option<Vec<String>>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    source_file: Option<String>,
    #[serde(default, deserialize_with = "crate::args::strings")]
    entities_produced: Vec<String>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    session_id: Option<String>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    status: Option<String>,
}

/// `specforge.infer_session`: each action rewrites `specforge-infer.json`
/// (a mutation that names it); a refusal writes nothing.
pub fn call(call: &mut Call<'_>, args: Args) -> Mutated {
    let Some(project_root) = call.root().map(std::path::Path::to_path_buf) else {
        return Mutated::refused(crate::target::no_project(crate::target::Reach::Served));
    };
    let state = &*call.state;

    let action = match args.action.as_deref() {
        Some(a) => a,
        None => {
            return Mutated::refused(ToolOutcome::invalid_input(
                "action",
                "Missing required parameter: action (start | mark_analyzed | end)",
            ));
        }
    };

    match action {
        "start" => handle_start(state, &args, &project_root),
        "mark_analyzed" => handle_mark_analyzed(state, &args, &project_root),
        "end" => handle_end(state, &args, &project_root),
        _ => Mutated::refused(ToolOutcome::invalid_input(
            "action",
            format!(
                "Unknown action: '{}'. Expected: start, mark_analyzed, end",
                action
            ),
        )),
    }
}

fn handle_start(_state: &McpState, args: &Args, project_root: &std::path::Path) -> Mutated {
    let mut manifest = match inference::load_inference_manifest(project_root) {
        Ok(m) => m,
        Err(e) => return Mutated::refused(super::manifest_error(e)),
    };

    let agent = args.agent.clone().unwrap_or_else(|| "unknown".to_string());

    let source_roots = args.source_roots.clone();

    if let Some(roots) = source_roots {
        manifest.source_roots = roots;
    }

    let session_id = generate_session_id();
    let now = now_rfc3339();

    let session = InferenceSession {
        session_id: session_id.clone(),
        started_at: now,
        ended_at: None,
        agent,
        status: "active".to_string(),
    };

    let sessions_json = read_sessions_from_manifest(project_root);
    if sessions_json.iter().any(|s| s.status == "active") {
        return Mutated::refused(ToolOutcome::error(
            ErrorCode::Conflict,
            "Another inference session is already active. End it first.",
        ));
    }

    let mut sessions = sessions_json;
    sessions.push(session);

    let written = match write_sessions_to_manifest(project_root, &manifest, &sessions) {
        Ok(written) => written,
        Err(e) => return Mutated::refused(ToolOutcome::error(ErrorCode::InternalError, e)),
    };

    Mutated::wrote(
        ToolOutcome::ok(json!({
            "session_id": session_id,
            "status": "active"
        })),
        Written::files(written),
    )
}

fn handle_mark_analyzed(_state: &McpState, args: &Args, project_root: &std::path::Path) -> Mutated {
    let source_file = match args.source_file.as_deref() {
        Some(f) => f.to_string(),
        None => {
            return Mutated::refused(ToolOutcome::invalid_input(
                "source_file",
                "Missing required parameter: source_file",
            ));
        }
    };

    let entities: Vec<String> = args.entities_produced.clone();

    let mut manifest = match inference::load_inference_manifest(project_root) {
        Ok(m) => m,
        Err(e) => return Mutated::refused(super::manifest_error(e)),
    };

    let abs_path = project_root.join(&source_file);
    let content_hash = match inference::compute_content_hash(&abs_path) {
        Ok(h) => h,
        // Name the file as the agent did: the absolute path would leak
        // where the server's project lives.
        Err(_) => {
            return Mutated::refused(
                McpError::new(
                    ErrorCode::FileNotFound,
                    format!("failed to read {source_file}"),
                )
                .with_argument("source_file"),
            );
        }
    };

    manifest.upsert_source_entry(SourceFileEntry {
        path: source_file.clone(),
        content_hash,
        entities_produced: entities.clone(),
        analyzed_at: now_rfc3339(),
    });

    let sessions = read_sessions_from_manifest(project_root);
    let written = match write_sessions_to_manifest(project_root, &manifest, &sessions) {
        Ok(written) => written,
        Err(e) => return Mutated::refused(ToolOutcome::error(ErrorCode::InternalError, e)),
    };

    Mutated::wrote(
        ToolOutcome::ok(json!({
            "source_file": source_file,
            "entities_produced": entities,
            "status": "recorded"
        })),
        Written::files(written).with_entities(entities),
    )
}

fn handle_end(_state: &McpState, args: &Args, project_root: &std::path::Path) -> Mutated {
    let session_id = match args.session_id.as_deref() {
        Some(s) => s.to_string(),
        None => {
            return Mutated::refused(ToolOutcome::invalid_input(
                "session_id",
                "Missing required parameter: session_id",
            ));
        }
    };

    let status = args.status.as_deref().unwrap_or("completed").to_string();

    if status != "completed" && status != "paused" {
        return Mutated::refused(ToolOutcome::invalid_input(
            "status",
            format!("Invalid status: '{}'. Expected: completed, paused", status),
        ));
    }

    let manifest = match inference::load_inference_manifest(project_root) {
        Ok(m) => m,
        Err(e) => return Mutated::refused(super::manifest_error(e)),
    };

    let mut sessions = read_sessions_from_manifest(project_root);
    let session = sessions.iter_mut().find(|s| s.session_id == session_id);
    match session {
        Some(s) if s.status == "active" => {
            s.status = status.clone();
            s.ended_at = Some(now_rfc3339());
        }
        Some(_) => {
            return Mutated::refused(ToolOutcome::error(
                ErrorCode::Conflict,
                format!("Session '{session_id}' is not active"),
            ));
        }
        None => {
            return Mutated::refused(ToolOutcome::invalid_input(
                "session_id",
                format!("Unknown session_id: '{session_id}'"),
            ));
        }
    }

    let written = match write_sessions_to_manifest(project_root, &manifest, &sessions) {
        Ok(written) => written,
        Err(e) => return Mutated::refused(ToolOutcome::error(ErrorCode::InternalError, e)),
    };

    Mutated::wrote(
        ToolOutcome::ok(json!({
            "session_id": session_id,
            "status": status
        })),
        Written::files(written),
    )
}

/// A new session's ID: a random (version 4) UUID, as `InferenceSession`'s
/// `session_id` is (`start_inference_session`: "a generated UUID").
fn generate_session_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("OS entropy unavailable");
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // the RFC 9562 variant
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// The current time as an RFC 3339 UTC timestamp, as the session and
/// source-file records store it.
fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
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

/// Write the manifest with `sessions`; what it wrote (the manifest).
fn write_sessions_to_manifest(
    project_root: &std::path::Path,
    manifest: &InferenceManifest,
    sessions: &[InferenceSession],
) -> Result<Writes, String> {
    let path = sessions_path(project_root);
    let mut value = serde_json::to_value(manifest).unwrap_or(json!({}));
    if let Value::Object(ref mut map) = value {
        map.insert(
            "sessions".to_string(),
            serde_json::to_value(sessions).unwrap_or(json!([])),
        );
    }
    let json =
        serde_json::to_string_pretty(&value).map_err(|e| format!("Failed to serialize: {}", e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| format!("Failed to write: {}", e))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Failed to rename: {}", e))?;
    Ok(Writes::from_iter([path]))
}
