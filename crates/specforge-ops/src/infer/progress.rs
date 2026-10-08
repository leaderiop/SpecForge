//! Inference progress: the source files the enabled analyzers would scan
//! against `specforge-infer.json`'s index.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};
use specforge_protocol_types::ExtensionDeclaration;

use super::discovery::source_files;
use super::gaps::directory_of;
use super::manifest::{InferenceManifest, InferenceSummary, detect_stale_entries};
use crate::OpError;
use crate::view::ProjectView;

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

/// Inference progress for the project the view was compiled from: the
/// source files its loaded analyzers would scan at its root, against
/// `specforge-infer.json` there. Without a root: `no_project`.
pub fn progress(view: &ProjectView) -> Result<Progress, OpError> {
    let root = view.project_root()?;
    Ok(progress_under(
        root,
        view.registries().declarations(),
        &InferenceManifest::at(root)?,
    ))
}

/// [`progress`], counting from scratch when `specforge-infer.json` cannot
/// be read, and nothing without a root: what the infer prompt plans from.
pub fn progress_or_fresh(view: &ProjectView) -> Progress {
    let Some(root) = view.root() else {
        return Progress {
            summary: InferenceManifest::default().compute_summary(0),
            unanalyzed: Vec::new(),
            stale: Vec::new(),
            deleted: Vec::new(),
        };
    };
    progress_under(
        root,
        view.registries().declarations(),
        &InferenceManifest::at(root).unwrap_or_default(),
    )
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
    let (stale, deleted) = detect_stale_entries(root, manifest);
    Progress {
        summary: manifest.compute_summary(files.len()),
        unanalyzed,
        stale,
        deleted,
    }
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

#[cfg(test)]
mod tests {
    use super::super::gaps::gaps;
    use super::super::manifest::compute_content_hash;
    use super::*;
    use crate::view::testing::Fixture;
    use specforge_test_macros::test as specforge_test;

    /// A project with Rust sources under `src/`, one of them indexed,
    /// whose compile loaded a Rust analyzer.
    fn project() -> Fixture {
        let fixture = Fixture::new().declarations(rust_analyzer());
        let dir = &fixture.dir;
        std::fs::create_dir_all(dir.path().join("src/net")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "pub fn alpha() {}\n").unwrap();
        std::fs::write(dir.path().join("src/net/wire.rs"), "pub struct Wire;\n").unwrap();
        let hash = compute_content_hash(&dir.path().join("src/lib.rs")).unwrap();
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
        fixture
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
        let progress = progress(&dir.view()).unwrap();
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
        std::fs::write(dir.dir.path().join("src/lib.rs"), "pub fn beta() {}\n").unwrap();
        assert_eq!(progress(&dir.view()).unwrap().stale, ["src/lib.rs"]);
    }

    #[specforge_test(
        behavior = "provide_mcp_infer_progress_tool",
        verify = "graceful handling when specforge-infer.json is missing"
    )]
    fn progress_without_a_manifest_counts_every_file_unanalyzed() {
        let dir = project();
        std::fs::remove_file(dir.dir.path().join("specforge-infer.json")).unwrap();
        let progress = progress(&dir.view()).unwrap();
        assert_eq!(progress.summary.files_analyzed, 0);
        assert!(progress.unanalyzed.contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn an_invalid_manifest_is_its_own_error() {
        let dir = project();
        std::fs::write(dir.dir.path().join("specforge-infer.json"), "{ nope").unwrap();
        assert_eq!(
            progress(&dir.view()).unwrap_err().code,
            super::super::manifest::MANIFEST_INVALID
        );
    }

    #[test]
    fn progress_over_a_rootless_view_is_no_project_and_fresh_counts_nothing() {
        let dir = project();
        let rootless = dir.rootless_view();

        assert_eq!(progress(&rootless).unwrap_err().code, "no_project");
        let runtime = specforge_wasm::testing::InProcessRuntime::new();
        assert_eq!(gaps(&rootless, &runtime).unwrap_err().code, "no_project");
        let fresh = progress_or_fresh(&rootless);
        assert_eq!(fresh.summary.files_total, 0);
        assert_eq!(fresh.summary.files_analyzed, 0);
        assert!(fresh.unanalyzed.is_empty() && fresh.stale.is_empty() && fresh.deleted.is_empty());

        // Rooted, the same view counts the files at its root.
        assert_eq!(progress_or_fresh(&dir.view()).summary.files_total, 2);
    }
}
