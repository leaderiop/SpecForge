//! Inference: the manifest an agent's inference is recorded in
//! (`specforge-infer.json`), its sessions, its progress and gaps, and the
//! `inferred` lint. `specforge infer-status`, the MCP
//! `specforge.infer_progress`, `specforge.infer_gaps` and
//! `specforge.infer_session` tools and the infer prompt's plan read the
//! project here.
//!
//! Progress compares the source files the enabled analyzers would scan
//! with the manifest's index; gaps scans those files for public items and
//! keeps the ones no graph entity names; the lint reports analyzed files
//! that changed or are over-dense; [`session`] records one step of an
//! inference session.

mod discovery;
mod gaps;
mod lint;
mod manifest;
mod progress;
mod session;

pub use gaps::{Gaps, SourceItem, directory_of, gaps};
pub use lint::lint;
pub(crate) use manifest::read_manifest;
pub use manifest::{
    InferenceManifest, InferenceSession, InferenceSummary, MANIFEST_FILENAME,
    MANIFEST_WRITE_FAILED, SessionStatus, SourceFileEntry,
};
pub use progress::{Progress, progress};
pub use session::{
    END_STATUS, EndStatus, Recorded, SESSION_ACTION, SESSION_ACTIVE, SESSION_NOT_ACTIVE,
    SOURCE_OUTSIDE_ROOT, SOURCE_UNREADABLE, SessionAction, SessionOutcome, SessionStep,
    UNKNOWN_SESSION, session,
};
