//! Inference sessions: one agent's run of inference, recorded in the
//! inference manifest. Its steps (start, mark a source file analyzed, end)
//! are one management operation, [`session`], which reads the manifest
//! once, applies the step and writes it once (ADR 0022 "Inference
//! sessions").

use std::path::Path;

use super::manifest::{
    InferenceManifest, InferenceSession, SessionStatus, SourceFileEntry, compute_content_hash,
};
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind, Writes};

/// A start while a session is active (`Conflict`).
pub const SESSION_ACTIVE: &str = "session_active";
/// An end of a session that is not active (`Conflict`).
pub const SESSION_NOT_ACTIVE: &str = "session_not_active";
/// An end of a session the manifest does not record (`InvalidInput`).
pub const UNKNOWN_SESSION: &str = "unknown_session";
/// A marked file that cannot be read (`OpErrorKind::of_io`: `FileNotFound`
/// when missing).
pub const SOURCE_UNREADABLE: &str = "source_unreadable";

/// How a session ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndStatus {
    Completed,
    Paused,
}

impl EndStatus {
    fn status(self) -> SessionStatus {
        match self {
            EndStatus::Completed => SessionStatus::Completed,
            EndStatus::Paused => SessionStatus::Paused,
        }
    }
}

/// One step of an inference session.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionStep<'a> {
    /// Begin a session for `agent` (`unknown` when unnamed); replace the
    /// manifest's source roots when `source_roots` is given.
    Start {
        agent: Option<&'a str>,
        source_roots: Option<&'a [String]>,
    },
    /// Record `source_file` (a path under the root) as analyzed now,
    /// producing `entities`; its SHA-256 is read from disk.
    MarkAnalyzed {
        source_file: &'a str,
        entities: &'a [String],
    },
    /// End the active session `session_id` as `status`, now.
    End {
        session_id: &'a str,
        status: EndStatus,
    },
}

/// What a step recorded.
#[derive(Debug, Clone, PartialEq)]
pub enum Recorded {
    Started {
        session_id: String,
    },
    /// `source_file` as recorded.
    Marked {
        source_file: String,
        entities: Vec<String>,
    },
    Ended {
        session_id: String,
        status: SessionStatus,
    },
}

/// A recorded step and the files it wrote (the manifest).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionOutcome {
    pub recorded: Recorded,
    pub writes: Writes,
}

/// Record `step` in the inference manifest at the view's root: one read,
/// the step applied, one write, the writes returned (ADR 0022 D1). A
/// refusal writes nothing. In order: `no_project` without a root; E071 for
/// a manifest that cannot be used; then the step's own refusal
/// ([`SESSION_ACTIVE`], [`UNKNOWN_SESSION`], [`SESSION_NOT_ACTIVE`],
/// [`SOURCE_UNREADABLE`]). Marking a file does not need an active session.
pub fn session(view: &ProjectView<'_>, step: SessionStep<'_>) -> Result<SessionOutcome, OpError> {
    let root = view.project_root()?;
    let mut manifest = InferenceManifest::at(root)?;
    let recorded = apply(&mut manifest, root, step, &now(), new_session_id)?;
    let writes = manifest.write(root)?;
    Ok(SessionOutcome { recorded, writes })
}

/// `step` applied to `manifest`, at time `now`: the rules, without disk
/// (apart from the hash of a marked file).
fn apply(
    manifest: &mut InferenceManifest,
    root: &Path,
    step: SessionStep<'_>,
    now: &str,
    new_id: impl FnOnce() -> String,
) -> Result<Recorded, OpError> {
    match step {
        SessionStep::Start {
            agent,
            source_roots,
        } => {
            if manifest.active_session().is_some() {
                return Err(OpError::new(
                    OpErrorKind::Conflict,
                    SESSION_ACTIVE,
                    "Another inference session is already active. End it first.",
                ));
            }
            if let Some(roots) = source_roots {
                manifest.source_roots = roots.to_vec();
            }
            let session_id = new_id();
            manifest.sessions.push(InferenceSession {
                session_id: session_id.clone(),
                started_at: now.to_string(),
                ended_at: None,
                agent: agent.unwrap_or("unknown").to_string(),
                status: SessionStatus::Active,
                unknown: Default::default(),
            });
            Ok(Recorded::Started { session_id })
        }
        SessionStep::MarkAnalyzed {
            source_file,
            entities,
        } => {
            // The file is named as the agent did: the absolute path would
            // leak where the server's project lives.
            let content_hash = compute_content_hash(&root.join(source_file)).map_err(|e| {
                OpError::new(
                    OpErrorKind::of_io(&e),
                    SOURCE_UNREADABLE,
                    format!("failed to read {source_file}"),
                )
            })?;
            manifest.upsert_source_entry(SourceFileEntry::new(
                source_file,
                content_hash,
                entities.to_vec(),
                now,
            ));
            Ok(Recorded::Marked {
                source_file: source_file.to_string(),
                entities: entities.to_vec(),
            })
        }
        SessionStep::End { session_id, status } => {
            let Some(session) = manifest
                .sessions
                .iter_mut()
                .find(|s| s.session_id == session_id)
            else {
                return Err(OpError::new(
                    OpErrorKind::InvalidInput,
                    UNKNOWN_SESSION,
                    format!("Unknown session_id: '{session_id}'"),
                ));
            };
            if session.status != SessionStatus::Active {
                return Err(OpError::new(
                    OpErrorKind::Conflict,
                    SESSION_NOT_ACTIVE,
                    format!("Session '{session_id}' is not active"),
                ));
            }
            session.status = status.status();
            session.ended_at = Some(now.to_string());
            Ok(Recorded::Ended {
                session_id: session_id.to_string(),
                status: session.status,
            })
        }
    }
}

/// A new session's ID: a random (version 4) UUID, as `InferenceSession`'s
/// `session_id` is (`start_inference_session`: "a generated UUID").
fn new_session_id() -> String {
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
fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
