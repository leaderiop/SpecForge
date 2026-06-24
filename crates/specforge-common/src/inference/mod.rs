pub mod anchors;
pub mod discovery;

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use discovery::{AnalyzerConfig, SourceDiscoveryConfig, discover_source_files};

const CURRENT_VERSION: u32 = 1;
const MANIFEST_FILENAME: &str = "specforge-infer.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceManifest {
    pub version: u32,
    pub source_roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_index: Vec<SourceFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFileEntry {
    pub path: String,
    pub content_hash: String,
    pub entities_produced: Vec<String>,
    pub analyzed_at: String,
}

#[derive(Debug, Clone)]
pub struct InferenceSummary {
    pub files_total: usize,
    pub files_analyzed: usize,
    pub entities_produced: usize,
}

impl Default for InferenceManifest {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            source_roots: Vec::new(),
            source_index: Vec::new(),
        }
    }
}

impl InferenceManifest {
    pub fn source_index_map(&self) -> HashMap<&str, &SourceFileEntry> {
        self.source_index
            .iter()
            .map(|e| (e.path.as_str(), e))
            .collect()
    }

    pub fn upsert_source_entry(&mut self, entry: SourceFileEntry) {
        if let Some(existing) = self.source_index.iter_mut().find(|e| e.path == entry.path) {
            *existing = entry;
        } else {
            self.source_index.push(entry);
        }
        self.source_index.sort_by(|a, b| a.path.cmp(&b.path));
    }

    pub fn compute_summary(&self, total_source_files: usize) -> InferenceSummary {
        let entities_produced = self
            .source_index
            .iter()
            .map(|e| e.entities_produced.len())
            .sum();

        InferenceSummary {
            files_total: total_source_files,
            files_analyzed: self.source_index.len(),
            entities_produced,
        }
    }
}

pub fn load_inference_manifest(project_root: &Path) -> Result<InferenceManifest, String> {
    let path = project_root.join(MANIFEST_FILENAME);
    if !path.exists() {
        return Ok(InferenceManifest::default());
    }

    let content =
        fs::read_to_string(&path).map_err(|e| format!("failed to read {MANIFEST_FILENAME}: {e}"))?;
    let manifest: InferenceManifest = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse {MANIFEST_FILENAME}: {e}"))?;

    if manifest.version != CURRENT_VERSION {
        return Err(format!(
            "unsupported {MANIFEST_FILENAME} version: {} (expected {CURRENT_VERSION})",
            manifest.version
        ));
    }

    Ok(manifest)
}

pub fn save_inference_manifest(
    project_root: &Path,
    manifest: &InferenceManifest,
) -> Result<(), String> {
    let path = project_root.join(MANIFEST_FILENAME);

    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize {MANIFEST_FILENAME}: {e}"))?;

    let tmp_path = path.with_extension("json.tmp");
    let mut file = fs::File::create(&tmp_path)
        .map_err(|e| format!("failed to create temp file: {e}"))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("failed to write temp file: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("failed to sync temp file: {e}"))?;
    drop(file);

    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("failed to rename temp file to {MANIFEST_FILENAME}: {e}"))?;

    Ok(())
}

pub fn compute_content_hash(file_path: &Path) -> Result<String, String> {
    let content =
        fs::read(file_path).map_err(|e| format!("failed to read {}: {e}", file_path.display()))?;
    let hash = Sha256::digest(&content);
    Ok(format!("{:x}", hash))
}

pub fn detect_stale_entries(
    project_root: &Path,
    manifest: &InferenceManifest,
) -> (Vec<String>, Vec<String>) {
    let mut stale = Vec::new();
    let mut deleted = Vec::new();

    for entry in &manifest.source_index {
        let abs_path = project_root.join(&entry.path);
        if !abs_path.exists() {
            deleted.push(entry.path.clone());
            continue;
        }
        match compute_content_hash(&abs_path) {
            Ok(hash) if hash != entry.content_hash => {
                stale.push(entry.path.clone());
            }
            _ => {}
        }
    }

    (stale, deleted)
}

pub fn compute_inference_diagnostics(
    project_root: &Path,
    manifest: &InferenceManifest,
    density_threshold: f64,
) -> Vec<crate::Diagnostic> {
    let mut diagnostics = Vec::new();

    let (stale, _deleted) = detect_stale_entries(project_root, manifest);
    for path in &stale {
        diagnostics.push(crate::Diagnostic {
            code: "I200".to_string(),
            message: format!(
                "Source file '{}' has changed since it was analyzed — inferred entities may be stale",
                path
            ),
            severity: crate::Severity::Info,
            span: None,
            suggestion: Some("Re-analyze this file to update inferred entities".to_string()),
        });
    }

    for entry in &manifest.source_index {
        if entry.entities_produced.is_empty() {
            continue;
        }
        let abs_path = project_root.join(&entry.path);
        if !abs_path.exists() {
            continue;
        }
        let line_count = match fs::read_to_string(&abs_path) {
            Ok(content) => content.lines().count().max(1),
            Err(_) => continue,
        };
        let density = entry.entities_produced.len() as f64 / line_count as f64;
        if density > density_threshold {
            diagnostics.push(crate::Diagnostic {
                code: "I202".to_string(),
                message: format!(
                    "High inference density in '{}': {} entities from {} lines ({:.1} entities/100 lines, threshold: {:.1})",
                    entry.path,
                    entry.entities_produced.len(),
                    line_count,
                    density * 100.0,
                    density_threshold * 100.0,
                ),
                severity: crate::Severity::Info,
                span: None,
                suggestion: Some("Consider whether some inferred entities should be merged or removed".to_string()),
            });
        }
    }

    diagnostics
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    pub name: String,
    pub item_kind: String,
    pub file: String,
    pub line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scanner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapReport {
    pub total_pub_items: usize,
    pub covered_items: usize,
    pub gaps: Vec<SourceItem>,
    pub approximate: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scanners_used: Vec<String>,
}

pub fn compute_gap_report(
    scanned_items: Vec<SourceItem>,
    entity_ids: &[&str],
    scanners_used: Vec<String>,
) -> GapReport {
    let total = scanned_items.len();
    let entity_set: std::collections::HashSet<&str> = entity_ids.iter().copied().collect();

    let gaps: Vec<SourceItem> = scanned_items
        .into_iter()
        .filter(|item| !entity_set.contains(item.name.as_str()))
        .collect();

    GapReport {
        total_pub_items: total,
        covered_items: total - gaps.len(),
        scanners_used,
        gaps,
        approximate: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_manifest_has_current_version() {
        let m = InferenceManifest::default();
        assert_eq!(m.version, CURRENT_VERSION);
        assert!(m.source_roots.is_empty());
        assert!(m.source_index.is_empty());
    }

    #[test]
    fn round_trip_serialization() {
        let mut m = InferenceManifest {
            source_roots: vec!["src/".to_string()],
            ..Default::default()
        };
        m.upsert_source_entry(SourceFileEntry {
            path: "src/main.rs".to_string(),
            content_hash: "abc123".to_string(),
            entities_produced: vec!["my_behavior".to_string()],
            analyzed_at: "2026-04-24T10:00:00Z".to_string(),
        });

        let json = serde_json::to_string_pretty(&m).unwrap();
        let loaded: InferenceManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.version, CURRENT_VERSION);
        assert_eq!(loaded.source_roots, vec!["src/"]);
        assert_eq!(loaded.source_index.len(), 1);
        assert_eq!(loaded.source_index[0].path, "src/main.rs");
        assert_eq!(loaded.source_index[0].entities_produced, vec!["my_behavior"]);
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let m = load_inference_manifest(dir.path()).unwrap();
        assert_eq!(m.version, CURRENT_VERSION);
        assert!(m.source_index.is_empty());
    }

    #[test]
    fn load_rejects_unsupported_version() {
        let dir = TempDir::new().unwrap();
        let content = r#"{"version": 999, "source_roots": [], "source_index": []}"#;
        fs::write(dir.path().join(MANIFEST_FILENAME), content).unwrap();

        let result = load_inference_manifest(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported"));
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let mut m = InferenceManifest {
            source_roots: vec!["crates/my-crate/src".to_string()],
            ..Default::default()
        };
        m.upsert_source_entry(SourceFileEntry {
            path: "crates/my-crate/src/lib.rs".to_string(),
            content_hash: "deadbeef".to_string(),
            entities_produced: vec!["a".to_string(), "b".to_string()],
            analyzed_at: "2026-04-24T12:00:00Z".to_string(),
        });

        save_inference_manifest(dir.path(), &m).unwrap();
        let loaded = load_inference_manifest(dir.path()).unwrap();

        assert_eq!(loaded.source_roots, m.source_roots);
        assert_eq!(loaded.source_index.len(), 1);
        assert_eq!(loaded.source_index[0].entities_produced.len(), 2);
    }

    #[test]
    fn source_index_sorted_after_upsert() {
        let mut m = InferenceManifest::default();
        let entry = |path: &str| SourceFileEntry {
            path: path.to_string(),
            content_hash: "h".to_string(),
            entities_produced: vec![],
            analyzed_at: "t".to_string(),
        };

        m.upsert_source_entry(entry("z.rs"));
        m.upsert_source_entry(entry("a.rs"));
        m.upsert_source_entry(entry("m.rs"));

        let paths: Vec<&str> = m.source_index.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["a.rs", "m.rs", "z.rs"]);
    }

    #[test]
    fn upsert_replaces_existing_entry() {
        let mut m = InferenceManifest::default();
        m.upsert_source_entry(SourceFileEntry {
            path: "src/lib.rs".to_string(),
            content_hash: "old".to_string(),
            entities_produced: vec!["a".to_string()],
            analyzed_at: "t1".to_string(),
        });
        m.upsert_source_entry(SourceFileEntry {
            path: "src/lib.rs".to_string(),
            content_hash: "new".to_string(),
            entities_produced: vec!["a".to_string(), "b".to_string()],
            analyzed_at: "t2".to_string(),
        });

        assert_eq!(m.source_index.len(), 1);
        assert_eq!(m.source_index[0].content_hash, "new");
        assert_eq!(m.source_index[0].entities_produced.len(), 2);
    }

    #[test]
    fn compute_summary_counts() {
        let mut m = InferenceManifest::default();
        m.upsert_source_entry(SourceFileEntry {
            path: "a.rs".to_string(),
            content_hash: "h".to_string(),
            entities_produced: vec!["e1".to_string(), "e2".to_string()],
            analyzed_at: "t".to_string(),
        });
        m.upsert_source_entry(SourceFileEntry {
            path: "b.rs".to_string(),
            content_hash: "h".to_string(),
            entities_produced: vec!["e3".to_string()],
            analyzed_at: "t".to_string(),
        });

        let summary = m.compute_summary(10);
        assert_eq!(summary.files_total, 10);
        assert_eq!(summary.files_analyzed, 2);
        assert_eq!(summary.entities_produced, 3);
    }

    #[test]
    fn content_hash_is_sha256() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "fn main() {}").unwrap();

        let hash = compute_content_hash(&file).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn detect_stale_and_deleted() {
        let dir = TempDir::new().unwrap();
        let file_a = dir.path().join("a.rs");
        fs::write(&file_a, "original").unwrap();
        let hash_a = compute_content_hash(&file_a).unwrap();

        let mut m = InferenceManifest::default();
        m.upsert_source_entry(SourceFileEntry {
            path: "a.rs".to_string(),
            content_hash: hash_a,
            entities_produced: vec![],
            analyzed_at: "t".to_string(),
        });
        m.upsert_source_entry(SourceFileEntry {
            path: "deleted.rs".to_string(),
            content_hash: "whatever".to_string(),
            entities_produced: vec![],
            analyzed_at: "t".to_string(),
        });

        let (stale, deleted) = detect_stale_entries(dir.path(), &m);
        assert!(stale.is_empty());
        assert_eq!(deleted, vec!["deleted.rs"]);

        fs::write(&file_a, "modified").unwrap();
        let (stale, deleted) = detect_stale_entries(dir.path(), &m);
        assert_eq!(stale, vec!["a.rs"]);
        assert_eq!(deleted, vec!["deleted.rs"]);
    }

    #[test]
    fn empty_source_index_not_serialized() {
        let m = InferenceManifest::default();
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("source_index"));
    }

    #[test]
    fn gap_report_identifies_uncovered_items() {
        let items = vec![
            SourceItem { name: "hello".into(), item_kind: "function".into(), file: "src/lib.rs".into(), line: 1, scanner: Some("rust".into()) },
            SourceItem { name: "config".into(), item_kind: "struct".into(), file: "src/lib.rs".into(), line: 2, scanner: Some("rust".into()) },
        ];
        let report = compute_gap_report(items, &["hello"], vec!["rust".into()]);
        assert_eq!(report.total_pub_items, 2);
        assert_eq!(report.covered_items, 1);
        assert_eq!(report.gaps.len(), 1);
        assert_eq!(report.gaps[0].name, "config");
        assert!(!report.approximate);
        assert_eq!(report.scanners_used, vec!["rust"]);
    }

    #[test]
    fn gap_report_all_covered() {
        let items = vec![
            SourceItem { name: "hello".into(), item_kind: "function".into(), file: "src/lib.rs".into(), line: 1, scanner: Some("rust".into()) },
        ];
        let report = compute_gap_report(items, &["hello"], vec!["rust".into()]);
        assert_eq!(report.gaps.len(), 0);
        assert_eq!(report.covered_items, 1);
    }

    #[test]
    fn gap_report_empty_items() {
        let report = compute_gap_report(vec![], &["hello"], vec![]);
        assert_eq!(report.total_pub_items, 0);
        assert_eq!(report.gaps.len(), 0);
    }

    #[test]
    fn i200_stale_source_anchor() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("main.rs");
        fs::write(&file, "fn main() {}").unwrap();

        let hash = compute_content_hash(&file).unwrap();
        let mut manifest = InferenceManifest::default();
        manifest.upsert_source_entry(SourceFileEntry {
            path: "main.rs".to_string(),
            content_hash: hash,
            entities_produced: vec!["app_main".to_string()],
            analyzed_at: "t".to_string(),
        });

        let diags = compute_inference_diagnostics(dir.path(), &manifest, 0.05);
        assert!(diags.iter().all(|d| d.code != "I200"));

        fs::write(&file, "fn main() { changed }").unwrap();
        let diags = compute_inference_diagnostics(dir.path(), &manifest, 0.05);
        assert!(diags.iter().any(|d| d.code == "I200"));
    }

    #[test]
    fn i202_high_density() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("tiny.rs");
        fs::write(&file, "pub fn a() {}\npub fn b() {}").unwrap();

        let hash = compute_content_hash(&file).unwrap();
        let mut manifest = InferenceManifest::default();
        manifest.upsert_source_entry(SourceFileEntry {
            path: "tiny.rs".to_string(),
            content_hash: hash,
            entities_produced: vec![
                "e1".into(), "e2".into(), "e3".into(), "e4".into(), "e5".into(),
            ],
            analyzed_at: "t".to_string(),
        });

        let diags = compute_inference_diagnostics(dir.path(), &manifest, 0.05);
        assert!(diags.iter().any(|d| d.code == "I202"));
    }

    #[test]
    fn i202_below_threshold_no_diagnostic() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("big.rs");
        let content = (0..200).map(|i| format!("pub fn func_{i}() {{}}\n")).collect::<String>();
        fs::write(&file, &content).unwrap();

        let hash = compute_content_hash(&file).unwrap();
        let mut manifest = InferenceManifest::default();
        manifest.upsert_source_entry(SourceFileEntry {
            path: "big.rs".to_string(),
            content_hash: hash,
            entities_produced: vec!["one".into()],
            analyzed_at: "t".to_string(),
        });

        let diags = compute_inference_diagnostics(dir.path(), &manifest, 0.05);
        assert!(diags.iter().all(|d| d.code != "I202"));
    }
}
