mod diagnostic;
pub mod inference;
mod interner;
mod project;
mod span;
mod suggest;

pub use diagnostic::{Diagnostic, DiagnosticsExt, Severity};
pub use inference::{
    GapReport, InferenceManifest, InferenceSummary, SourceFileEntry, SourceItem,
    compute_content_hash, compute_gap_report, compute_inference_diagnostics,
    detect_stale_entries, load_inference_manifest, save_inference_manifest,
    AnalyzerConfig, SourceDiscoveryConfig, discover_source_files,
};
pub use inference::anchors::{
    AnchorManifest, SourceAnchor, load_anchor_manifest, save_anchor_manifest,
};
pub use interner::Sym;
pub use project::{find_project_root, load_project_config, InferenceConfig, ProjectConfig};
pub use span::SourceSpan;
pub use suggest::find_close_match;
