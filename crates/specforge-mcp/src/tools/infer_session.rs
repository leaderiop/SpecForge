use serde_json::json;

use specforge_ops::infer::{
    self, InferenceManifest, InferenceSession, SessionStatus, SourceFileEntry,
};

use crate::args::Arguments;
use crate::mutation::{Mutated, Written};
use crate::target::Call;
use crate::tool::{ErrorCode, McpError, ToolOutcome};

/// The actions a session takes, in the order the listing states them.
const ACTIONS: &[&str] = &["start", "mark_analyzed", "end"];

/// The states a session ends in.
const END_STATUSES: &[&str] = &["completed", "paused"];

/// `specforge.infer_session`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Session action to perform
    #[arg(names = ACTIONS)]
    action: String,
    /// Agent identifier (for start)
    agent: Option<String>,
    /// Source directories to scan (for start)
    source_roots: Option<Vec<String>>,
    /// Relative path to analyzed file (for mark_analyzed)
    source_file: Option<String>,
    /// Entity IDs produced from the file (for mark_analyzed)
    entities_produced: Vec<String>,
    /// Session ID to end (for end)
    session_id: Option<String>,
    /// Final status (for end, default: completed)
    #[arg(names = END_STATUSES)]
    status: Option<String>,
}

/// `specforge.infer_session`: each action rewrites `specforge-infer.json`
/// (a mutation that names it); a refusal writes nothing.
pub fn call(call: &mut Call<'_>, args: Args) -> Mutated {
    let Some(project_root) = call.root().map(std::path::Path::to_path_buf) else {
        return Mutated::refused(crate::target::no_project(crate::target::Reach::Served));
    };

    let action = args.action.as_str();

    match action {
        "start" => handle_start(&args, &project_root),
        "mark_analyzed" => handle_mark_analyzed(&args, &project_root),
        "end" => handle_end(&args, &project_root),
        _ => Mutated::refused(ToolOutcome::invalid_input(
            "action",
            format!(
                "Unknown action: '{}'. Expected: {}",
                action,
                ACTIONS.join(", ")
            ),
        )),
    }
}

fn handle_start(args: &Args, project_root: &std::path::Path) -> Mutated {
    let mut manifest = match InferenceManifest::at(project_root) {
        Ok(m) => m,
        Err(e) => return Mutated::refused_after(false, e),
    };

    if manifest.active_session().is_some() {
        return Mutated::refused(ToolOutcome::error(
            ErrorCode::Conflict,
            "Another inference session is already active. End it first.",
        ));
    }

    if let Some(roots) = args.source_roots.clone() {
        manifest.source_roots = roots;
    }

    let session_id = generate_session_id();
    manifest.sessions.push(InferenceSession {
        session_id: session_id.clone(),
        started_at: now_rfc3339(),
        ended_at: None,
        agent: args.agent.clone().unwrap_or_else(|| "unknown".to_string()),
        status: SessionStatus::Active,
        unknown: Default::default(),
    });

    let written = match manifest.write(project_root) {
        Ok(written) => written,
        Err(e) => return Mutated::refused_after(false, e),
    };

    Mutated::wrote(
        ToolOutcome::ok(json!({
            "session_id": session_id,
            "status": SessionStatus::Active.name()
        })),
        Written::files(written),
    )
}

fn handle_mark_analyzed(args: &Args, project_root: &std::path::Path) -> Mutated {
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

    let mut manifest = match InferenceManifest::at(project_root) {
        Ok(m) => m,
        Err(e) => return Mutated::refused_after(false, e),
    };

    let abs_path = project_root.join(&source_file);
    let content_hash = match infer::compute_content_hash(&abs_path) {
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

    manifest.upsert_source_entry(SourceFileEntry::new(
        source_file.clone(),
        content_hash,
        entities.clone(),
        now_rfc3339(),
    ));

    let written = match manifest.write(project_root) {
        Ok(written) => written,
        Err(e) => return Mutated::refused_after(false, e),
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

fn handle_end(args: &Args, project_root: &std::path::Path) -> Mutated {
    let session_id = match args.session_id.as_deref() {
        Some(s) => s.to_string(),
        None => {
            return Mutated::refused(ToolOutcome::invalid_input(
                "session_id",
                "Missing required parameter: session_id",
            ));
        }
    };

    let status = args.status.as_deref().unwrap_or("completed");
    let ended = match status {
        "completed" => SessionStatus::Completed,
        "paused" => SessionStatus::Paused,
        _ => {
            return Mutated::refused(ToolOutcome::invalid_input(
                "status",
                format!(
                    "Invalid status: '{}'. Expected: {}",
                    status,
                    END_STATUSES.join(", ")
                ),
            ));
        }
    };

    let mut manifest = match InferenceManifest::at(project_root) {
        Ok(m) => m,
        Err(e) => return Mutated::refused_after(false, e),
    };

    match manifest
        .sessions
        .iter_mut()
        .find(|s| s.session_id == session_id)
    {
        Some(s) if s.status == SessionStatus::Active => {
            s.status = ended;
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

    let written = match manifest.write(project_root) {
        Ok(written) => written,
        Err(e) => return Mutated::refused_after(false, e),
    };

    Mutated::wrote(
        ToolOutcome::ok(json!({
            "session_id": session_id,
            "status": ended.name()
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
