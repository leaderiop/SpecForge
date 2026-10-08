//! The source anchors manifest: `<root>/specforge-anchors.json`, which
//! source item each entity is anchored to.

use std::fs;
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

pub fn load_anchor_manifest(project_root: &Path) -> Result<AnchorManifest, String> {
    let path = project_root.join(ANCHORS_FILENAME);
    if !path.exists() {
        return Ok(AnchorManifest::default());
    }

    let content =
        fs::read_to_string(&path).map_err(|e| format!("failed to read {ANCHORS_FILENAME}: {e}"))?;
    let manifest: AnchorManifest = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse {ANCHORS_FILENAME}: {e}"))?;

    Ok(manifest)
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
        let m = AnchorManifest {
            anchors: vec![sample_anchor("handle_login", "src/auth.rs", 10)],
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&m).unwrap();
        let loaded: AnchorManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.anchors.len(), 1);
        assert_eq!(loaded.anchors[0].entity_id, "handle_login");
    }

    #[test]
    fn load_reads_the_file() {
        let dir = TempDir::new().unwrap();
        let m = AnchorManifest {
            anchors: vec![
                sample_anchor("process_order", "src/orders.rs", 25),
                sample_anchor("validate_input", "src/validation.rs", 5),
            ],
            ..Default::default()
        };
        fs::write(
            dir.path().join(ANCHORS_FILENAME),
            serde_json::to_string_pretty(&m).unwrap(),
        )
        .unwrap();

        let loaded = load_anchor_manifest(dir.path()).unwrap();
        assert_eq!(loaded.anchors.len(), 2);
    }

    #[test]
    fn load_returns_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let m = load_anchor_manifest(dir.path()).unwrap();
        assert!(m.anchors.is_empty());
    }
}
