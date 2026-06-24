use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

const ANCHORS_FILENAME: &str = "specforge-anchors.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorManifest {
    pub version: u32,
    #[serde(default)]
    pub anchors: Vec<SourceAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAnchor {
    pub entity_id: String,
    pub file: String,
    pub line: usize,
    pub symbol_name: String,
    pub item_kind: String,
    pub scanner: String,
    #[serde(default)]
    pub mapping_strategy: Option<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
}

impl Default for AnchorManifest {
    fn default() -> Self {
        Self {
            version: 1,
            anchors: Vec::new(),
        }
    }
}

impl AnchorManifest {
    pub fn entity_to_sources(&self) -> HashMap<&str, Vec<&SourceAnchor>> {
        let mut map: HashMap<&str, Vec<&SourceAnchor>> = HashMap::new();
        for anchor in &self.anchors {
            map.entry(anchor.entity_id.as_str())
                .or_default()
                .push(anchor);
        }
        map
    }

    pub fn file_to_entities(&self) -> HashMap<&str, Vec<&SourceAnchor>> {
        let mut map: HashMap<&str, Vec<&SourceAnchor>> = HashMap::new();
        for anchor in &self.anchors {
            map.entry(anchor.file.as_str())
                .or_default()
                .push(anchor);
        }
        map
    }

    pub fn upsert(&mut self, anchor: SourceAnchor) {
        if let Some(existing) = self
            .anchors
            .iter_mut()
            .find(|a| a.entity_id == anchor.entity_id && a.file == anchor.file)
        {
            *existing = anchor;
        } else {
            self.anchors.push(anchor);
        }
        self.anchors.sort_by(|a, b| {
            a.file.cmp(&b.file).then(a.line.cmp(&b.line))
        });
    }
}

pub fn load_anchor_manifest(project_root: &Path) -> Result<AnchorManifest, String> {
    let path = project_root.join(ANCHORS_FILENAME);
    if !path.exists() {
        return Ok(AnchorManifest::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {ANCHORS_FILENAME}: {e}"))?;
    let manifest: AnchorManifest = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse {ANCHORS_FILENAME}: {e}"))?;

    Ok(manifest)
}

pub fn save_anchor_manifest(
    project_root: &Path,
    manifest: &AnchorManifest,
) -> Result<(), String> {
    let path = project_root.join(ANCHORS_FILENAME);

    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize {ANCHORS_FILENAME}: {e}"))?;

    let tmp_path = path.with_extension("json.tmp");
    let mut file =
        fs::File::create(&tmp_path).map_err(|e| format!("failed to create temp file: {e}"))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("failed to write temp file: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("failed to sync temp file: {e}"))?;
    drop(file);

    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("failed to rename temp file to {ANCHORS_FILENAME}: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_anchor(entity: &str, file: &str, line: usize) -> SourceAnchor {
        SourceAnchor {
            entity_id: entity.into(),
            file: file.into(),
            line,
            symbol_name: entity.into(),
            item_kind: "function".into(),
            scanner: "@specforge/rust".into(),
            mapping_strategy: Some("exact_snake_case".into()),
            confidence: Some(0.8),
        }
    }

    #[test]
    fn default_manifest_is_empty() {
        let m = AnchorManifest::default();
        assert_eq!(m.version, 1);
        assert!(m.anchors.is_empty());
    }

    #[test]
    fn round_trip_serialization() {
        let mut m = AnchorManifest::default();
        m.upsert(sample_anchor("handle_login", "src/auth.rs", 10));

        let json = serde_json::to_string_pretty(&m).unwrap();
        let loaded: AnchorManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.anchors.len(), 1);
        assert_eq!(loaded.anchors[0].entity_id, "handle_login");
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let mut m = AnchorManifest::default();
        m.upsert(sample_anchor("process_order", "src/orders.rs", 25));
        m.upsert(sample_anchor("validate_input", "src/validation.rs", 5));

        save_anchor_manifest(dir.path(), &m).unwrap();
        let loaded = load_anchor_manifest(dir.path()).unwrap();
        assert_eq!(loaded.anchors.len(), 2);
    }

    #[test]
    fn load_returns_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let m = load_anchor_manifest(dir.path()).unwrap();
        assert!(m.anchors.is_empty());
    }

    #[test]
    fn upsert_replaces_existing() {
        let mut m = AnchorManifest::default();
        m.upsert(sample_anchor("hello", "src/lib.rs", 10));
        m.upsert(SourceAnchor {
            entity_id: "hello".into(),
            file: "src/lib.rs".into(),
            line: 15,
            symbol_name: "hello".into(),
            item_kind: "function".into(),
            scanner: "@specforge/rust".into(),
            mapping_strategy: None,
            confidence: None,
        });

        assert_eq!(m.anchors.len(), 1);
        assert_eq!(m.anchors[0].line, 15);
    }

    #[test]
    fn entity_to_sources_lookup() {
        let mut m = AnchorManifest::default();
        m.upsert(sample_anchor("auth", "src/auth.rs", 10));
        m.upsert(sample_anchor("auth", "src/auth_v2.rs", 5));
        m.upsert(sample_anchor("orders", "src/orders.rs", 1));

        let map = m.entity_to_sources();
        assert_eq!(map["auth"].len(), 2);
        assert_eq!(map["orders"].len(), 1);
    }

    #[test]
    fn file_to_entities_lookup() {
        let mut m = AnchorManifest::default();
        m.upsert(sample_anchor("handle_login", "src/auth.rs", 10));
        m.upsert(sample_anchor("handle_logout", "src/auth.rs", 30));
        m.upsert(sample_anchor("process_order", "src/orders.rs", 5));

        let map = m.file_to_entities();
        assert_eq!(map["src/auth.rs"].len(), 2);
        assert_eq!(map["src/orders.rs"].len(), 1);
    }

    #[test]
    fn anchors_sorted_by_file_then_line() {
        let mut m = AnchorManifest::default();
        m.upsert(sample_anchor("c", "src/z.rs", 50));
        m.upsert(sample_anchor("a", "src/a.rs", 10));
        m.upsert(sample_anchor("b", "src/a.rs", 5));

        assert_eq!(m.anchors[0].entity_id, "b");
        assert_eq!(m.anchors[0].file, "src/a.rs");
        assert_eq!(m.anchors[0].line, 5);
        assert_eq!(m.anchors[1].entity_id, "a");
        assert_eq!(m.anchors[1].file, "src/a.rs");
        assert_eq!(m.anchors[1].line, 10);
        assert_eq!(m.anchors[2].entity_id, "c");
        assert_eq!(m.anchors[2].file, "src/z.rs");
    }
}
