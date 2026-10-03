mod diagnostic;
pub mod discovery;
pub mod inference;
mod interner;
mod present;
mod project;
mod slug;
mod span;
pub mod suggest;

pub use diagnostic::{Diagnostic, DiagnosticData, DiagnosticsExt, Severity};
pub use discovery::{SKIP_DIRS, discover_spec_files, is_discovered, is_excluded};
pub use inference::anchors::{
    AnchorManifest, SourceAnchor, load_anchor_manifest, save_anchor_manifest,
};
pub use inference::{
    AnalyzerConfig, GapReport, InferenceManifest, InferenceSummary, SourceDiscoveryConfig,
    SourceFileEntry, SourceItem, compute_content_hash, compute_gap_report,
    compute_inference_diagnostics, detect_stale_entries, discover_source_files,
    load_inference_manifest, save_inference_manifest,
};
pub use interner::Sym;
pub use present::{
    DiagnosticJson, MAX_DIAGNOSTICS, compute_exit_code, diagnostics_json, format_diagnostic,
    serialize_diagnostics, truncate_diagnostics,
};
pub use project::{
    InferenceConfig, ProjectConfig, extension_entry_name, find_project_root, load_project_config,
    validate_project_name,
};
pub use slug::slug;
pub use span::SourceSpan;
pub use suggest::find_close_match;
