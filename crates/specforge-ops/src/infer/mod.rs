//! Inference: the manifest an agent's inference is recorded in
//! (`specforge-infer.json`), its progress and gaps, and the `inferred`
//! lint. `specforge infer-status`, the MCP `specforge.infer_progress`,
//! `specforge.infer_gaps` and `specforge.infer_session` tools and the
//! infer prompt's plan read the project here.
//!
//! Progress compares the source files the enabled analyzers would scan
//! with the manifest's index; gaps scans those files for public items and
//! keeps the ones no graph entity names; the lint reports analyzed files
//! that changed or are over-dense.

mod discovery;
mod gaps;
mod lint;
mod manifest;
mod progress;

pub use gaps::{Gaps, SourceItem, directory_of, gaps};
pub use lint::lint;
pub use manifest::{
    InferenceManifest, InferenceSummary, MANIFEST_FILENAME, MANIFEST_INVALID, MANIFEST_UNREADABLE,
    SourceFileEntry, compute_content_hash, load_inference_manifest, save_inference_manifest,
};
pub use progress::{Progress, progress, progress_or_fresh};
