//! Inference progress and gaps: `specforge infer-status`, the MCP
//! `specforge.infer_progress` and `specforge.infer_gaps` tools, and the
//! infer prompt's plan read the project here.
//!
//! Progress compares the source files the enabled analyzers would scan
//! with `specforge-infer.json`'s index; gaps scans those files for public
//! items and keeps the ones no graph entity names.

use crate::OpError;
use serde_json::{Value, json};
use specforge_common::AnalyzerConfig;
use specforge_common::inference::{
    self, InferenceManifest, InferenceSummary, SourceItem, discovery::SourceDiscoveryConfig,
};
use specforge_graph::Graph;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_wasm::runtime::WasmRuntime;
use std::collections::BTreeMap;
use std::path::Path;

/// `specforge-infer.json` exists but cannot be read.
pub const MANIFEST_UNREADABLE: &str = "infer_manifest_unreadable";
/// `specforge-infer.json` is not a valid inference manifest.
pub const MANIFEST_INVALID: &str = "infer_manifest_invalid";

/// How far inference has come: every source file the analyzers would
/// scan, against the manifest's index.
#[derive(Debug, Clone)]
pub struct Progress {
    pub summary: InferenceSummary,
    /// Source files the manifest has not indexed.
    pub unanalyzed: Vec<String>,
    /// Indexed files whose content changed since they were analyzed.
    pub stale: Vec<String>,
    /// Indexed files no longer on disk.
    pub deleted: Vec<String>,
}

/// Public source items no graph entity names, by directory.
#[derive(Debug, Clone)]
pub struct Gaps {
    pub total_pub_items: usize,
    pub covered_items: usize,
    /// Directory (`.` for the root) to its uncovered items, sorted.
    pub by_directory: BTreeMap<String, Vec<SourceItem>>,
    /// Whether the counts are approximate: a regex fallback found the
    /// items (no scanner extension), or a scanner failed on a file.
    pub approximate: bool,
    pub scanners_used: Vec<String>,
    /// The files a scanner failed on (E028 each): their items are unknown.
    pub scan_failures: Vec<crate::scan::ScanFailure>,
}

/// The inference manifest at `root`: an empty one when there is none.
fn manifest(root: &Path) -> Result<InferenceManifest, OpError> {
    inference::load_inference_manifest(root).map_err(|message| {
        let code = if message.starts_with("failed to read") {
            MANIFEST_UNREADABLE
        } else {
            MANIFEST_INVALID
        };
        OpError::new(code, message)
    })
}

/// The source files the enabled analyzers would scan under `manifest`'s
/// source roots.
fn source_files(
    root: &Path,
    declarations: &[ExtensionDeclaration],
    manifest: &InferenceManifest,
) -> Vec<String> {
    let analyzers: Vec<AnalyzerConfig> = declarations
        .iter()
        .flat_map(|d| d.analyzers.iter())
        .map(|ac| AnalyzerConfig {
            language: ac.language.clone(),
            file_extensions: ac.file_extensions.clone(),
            excluded_dirs: ac.excluded_dirs.clone(),
        })
        .collect();
    let discovery = SourceDiscoveryConfig::from_analyzer_configs(&analyzers);
    inference::discover_source_files(root, &manifest.source_roots, &discovery)
}

/// Inference progress for the project at `root`, whose enabled extensions
/// are `declarations`.
pub fn progress(root: &Path, declarations: &[ExtensionDeclaration]) -> Result<Progress, OpError> {
    Ok(progress_under(root, declarations, &manifest(root)?))
}

/// [`progress`], counting from scratch when `specforge-infer.json` cannot
/// be read: the infer prompt plans a fresh inference then.
pub fn progress_or_fresh(root: &Path, declarations: &[ExtensionDeclaration]) -> Progress {
    progress_under(root, declarations, &manifest(root).unwrap_or_default())
}

fn progress_under(
    root: &Path,
    declarations: &[ExtensionDeclaration],
    manifest: &InferenceManifest,
) -> Progress {
    let files = source_files(root, declarations, manifest);
    let index = manifest.source_index_map();
    let unanalyzed = files
        .iter()
        .filter(|f| !index.contains_key(f.as_str()))
        .cloned()
        .collect();
    let (stale, deleted) = inference::detect_stale_entries(root, manifest);
    Progress {
        summary: manifest.compute_summary(files.len()),
        unanalyzed,
        stale,
        deleted,
    }
}

/// The public items of the project's source files that no entity of
/// `graph` names, scanned through the extensions' scanners on `runtime`.
pub fn gaps(
    root: &Path,
    declarations: &[ExtensionDeclaration],
    graph: &Graph,
    runtime: &dyn WasmRuntime,
) -> Result<Gaps, OpError> {
    let files = source_files(root, declarations, &manifest(root)?);
    let scanned = crate::scan::scan_source_files(runtime, declarations, root, &files);
    let entity_ids: Vec<&str> = graph
        .nodes()
        .into_iter()
        .map(|n| n.id.raw.as_str())
        .collect();
    let report = inference::compute_gap_report(scanned.items, &entity_ids, scanned.scanners_used);
    let mut by_directory: BTreeMap<String, Vec<SourceItem>> = BTreeMap::new();
    for gap in report.gaps {
        by_directory
            .entry(directory_of(&gap.file).to_string())
            .or_default()
            .push(gap);
    }
    Ok(Gaps {
        total_pub_items: report.total_pub_items,
        covered_items: report.covered_items,
        by_directory,
        approximate: report.approximate || !scanned.failures.is_empty(),
        scanners_used: report.scanners_used,
        scan_failures: scanned.failures,
    })
}

/// The directory part of a project-relative path, `.` at the root.
pub fn directory_of(path: &str) -> &str {
    path.rfind('/').map_or(".", |i| &path[..i])
}

impl Progress {
    /// The unanalyzed files grouped by directory, sorted.
    pub fn unanalyzed_by_directory(&self) -> BTreeMap<&str, Vec<&str>> {
        let mut by_dir: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for file in &self.unanalyzed {
            by_dir.entry(directory_of(file)).or_default().push(file);
        }
        by_dir
    }

    /// The document both surfaces answer with:
    /// `{summary, unanalyzed, stale, deleted}`.
    pub fn to_json(&self) -> Value {
        json!({
            "summary": {
                "files_total": self.summary.files_total,
                "files_analyzed": self.summary.files_analyzed,
                "entities_produced": self.summary.entities_produced,
            },
            "unanalyzed": self.unanalyzed,
            "stale": self.stale,
            "deleted": self.deleted,
        })
    }
}

impl Gaps {
    /// How many items no entity names.
    pub fn gap_count(&self) -> usize {
        self.by_directory.values().map(Vec::len).sum()
    }

    /// The document both surfaces answer with (the spec's
    /// `InferenceGapReport`): totals, then each directory's items.
    pub fn to_json(&self) -> Value {
        let by_directory: Vec<Value> = self
            .by_directory
            .iter()
            .map(|(dir, gaps)| {
                json!({
                    "directory": dir,
                    "count": gaps.len(),
                    "items": gaps.iter().map(|g| json!({
                        "name": g.name,
                        "item_kind": g.item_kind,
                        "file": g.file,
                        "line": g.line,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        json!({
            "total_pub_items": self.total_pub_items,
            "covered_items": self.covered_items,
            "gap_count": self.gap_count(),
            "approximate": self.approximate,
            "scanners_used": self.scanners_used,
            "scan_failures": self.scan_failures.iter().map(|failure| {
                let diagnostic = failure.error.diagnostic();
                json!({
                    "file": failure.file,
                    "code": diagnostic.code,
                    "message": diagnostic.message,
                })
            }).collect::<Vec<_>>(),
            "by_directory": by_directory,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    /// A project with Rust sources under `src/`, one of them indexed.
    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/net")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "pub fn alpha() {}\n").unwrap();
        std::fs::write(dir.path().join("src/net/wire.rs"), "pub struct Wire;\n").unwrap();
        let hash = inference::compute_content_hash(&dir.path().join("src/lib.rs")).unwrap();
        std::fs::write(
            dir.path().join("specforge-infer.json"),
            json!({
                "version": 1,
                "source_roots": ["src"],
                "source_index": [
                    {"path": "src/lib.rs", "content_hash": hash,
                     "entities_produced": ["alpha"], "analyzed_at": "2026-10-01T00:00:00Z"},
                    {"path": "src/gone.rs", "content_hash": "00",
                     "entities_produced": [], "analyzed_at": "2026-10-01T00:00:00Z"}
                ]
            })
            .to_string(),
        )
        .unwrap();
        dir
    }

    /// An extension analyzing Rust files.
    fn rust_analyzer() -> Vec<ExtensionDeclaration> {
        vec![ExtensionDeclaration {
            handshake: specforge_protocol_types::HandshakeResponse {
                name: "@acme/rust".into(),
                version: "1.0.0".into(),
                ..Default::default()
            },
            analyzers: vec![specforge_protocol_types::AnalyzerDescriptor {
                language: "rust".into(),
                file_extensions: vec![".rs".into()],
                scan_export: "scan".into(),
                classify_export: "classify".into(),
                map_export: "map".into(),
                ..Default::default()
            }],
            ..Default::default()
        }]
    }

    #[specforge_test(
        behavior = "provide_mcp_infer_progress_tool",
        verify = "returns summary with unanalyzed files"
    )]
    fn progress_lists_unindexed_and_deleted_files() {
        let dir = project();
        let progress = progress(dir.path(), &rust_analyzer()).unwrap();
        assert_eq!(progress.unanalyzed, ["src/net/wire.rs"]);
        assert_eq!(progress.deleted, ["src/gone.rs"]);
        assert!(progress.stale.is_empty(), "{progress:?}");
        assert_eq!(progress.summary.files_total, 2);
        assert_eq!(progress.summary.entities_produced, 1);
        assert_eq!(
            progress.unanalyzed_by_directory(),
            BTreeMap::from([("src/net", vec!["src/net/wire.rs"])])
        );
    }

    #[specforge_test(
        behavior = "provide_mcp_infer_progress_tool",
        verify = "detects stale files by content hash"
    )]
    fn progress_detects_a_changed_file() {
        let dir = project();
        std::fs::write(dir.path().join("src/lib.rs"), "pub fn beta() {}\n").unwrap();
        assert_eq!(
            progress(dir.path(), &rust_analyzer()).unwrap().stale,
            ["src/lib.rs"]
        );
    }

    #[specforge_test(
        behavior = "provide_mcp_infer_progress_tool",
        verify = "graceful handling when specforge-infer.json is missing"
    )]
    fn progress_without_a_manifest_counts_every_file_unanalyzed() {
        let dir = project();
        std::fs::remove_file(dir.path().join("specforge-infer.json")).unwrap();
        let progress = progress(dir.path(), &rust_analyzer()).unwrap();
        assert_eq!(progress.summary.files_analyzed, 0);
        assert!(progress.unanalyzed.contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn an_invalid_manifest_is_its_own_error() {
        let dir = project();
        std::fs::write(dir.path().join("specforge-infer.json"), "{ nope").unwrap();
        assert_eq!(
            progress(dir.path(), &rust_analyzer()).unwrap_err().code,
            MANIFEST_INVALID
        );
    }

    #[test]
    fn gaps_document_groups_items_by_directory() {
        let item = |name: &str, file: &str| SourceItem {
            name: name.into(),
            item_kind: "fn".into(),
            file: file.into(),
            line: 1,
            scanner: None,
        };
        let gaps = Gaps {
            total_pub_items: 3,
            covered_items: 1,
            by_directory: BTreeMap::from([
                (".".into(), vec![item("main", "main.rs")]),
                ("src".into(), vec![item("beta", "src/b.rs")]),
            ]),
            approximate: true,
            scanners_used: Vec::new(),
            scan_failures: Vec::new(),
        };
        let doc = gaps.to_json();
        assert_eq!(doc["gap_count"], 2);
        assert_eq!(doc["by_directory"][1]["directory"], "src");
        assert_eq!(doc["by_directory"][1]["items"][0]["file"], "src/b.rs");
        assert_eq!(directory_of("main.rs"), ".");
    }
}
