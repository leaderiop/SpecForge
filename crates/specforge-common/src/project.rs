use std::path::{Path, PathBuf};

/// Locate the project root by walking from `start` upward to the filesystem root.
///
/// At each directory level, checks for `specforge.json` (preferred) then `specforge.spec`.
/// The first directory containing either file wins (closest-wins semantics).
/// Symlinks are resolved before comparison to avoid infinite loops.
///
/// Returns `None` if neither file is found in any ancestor.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.canonicalize().ok()?;

    loop {
        if current.join("specforge.json").exists() || current.join("specforge.spec").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Parsed project configuration from specforge.json.
#[derive(Debug, Clone, Default)]
pub struct ProjectConfig {
    pub name: Option<String>,
    pub version: Option<String>,
    pub spec_root: Option<String>,
    pub extensions: Vec<String>,
    pub raw: Option<serde_json::Value>,
}

/// Load project configuration from specforge.json in the given directory.
/// Returns a default config if the file doesn't exist.
pub fn load_project_config(project_root: &Path) -> ProjectConfig {
    let config_path = project_root.join("specforge.json");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return ProjectConfig::default(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return ProjectConfig::default(),
    };

    let name = value.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let version = value.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
    let spec_root = value.get("spec_root").and_then(|v| v.as_str()).map(|s| s.to_string());
    let extensions = value
        .get("extensions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    ProjectConfig {
        name,
        version,
        spec_root,
        extensions,
        raw: Some(value),
    }
}
