//! The build cache: the explicit, opt-in record of one build's entity
//! statuses (`specforge-cache.json` at the project root), so history rules
//! such as status transitions compare against a declared input, never
//! hidden state. `specforge check --cache` writes it (behavior
//! `write_build_cache`); every compile that runs a check-phase pass reads
//! it and hands it to the passes as `previous` (behavior
//! `read_build_cache`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, Severity};
use specforge_graph::Graph;
use specforge_parser::FieldValue;

/// The cache's file name, beside `specforge.json`.
pub const BUILD_CACHE_FILE: &str = "specforge-cache.json";

/// The format this SpecForge writes and reads.
pub const BUILD_CACHE_FORMAT: u32 = 1;

/// One build's statuses:
/// `{"format": 1, "statuses": {"<entity id>": {"kind", "status"}}}`,
/// entities sorted by id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildCache {
    pub format: u32,
    pub statuses: BTreeMap<String, CachedStatus>,
}

/// An entity's kind and status in the build that wrote the cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedStatus {
    pub kind: String,
    pub status: String,
}

impl BuildCache {
    /// The statuses of `graph`: every entity that declares a `status`.
    pub fn of(graph: &Graph) -> Self {
        let statuses = graph
            .nodes()
            .into_iter()
            .filter_map(|node| {
                let status = match node.fields.get("status")? {
                    FieldValue::Identifier(s) | FieldValue::String(s) => s.clone(),
                    _ => return None,
                };
                Some((
                    node.id.raw.to_string(),
                    CachedStatus {
                        kind: node.kind.raw.to_string(),
                        status,
                    },
                ))
            })
            .collect();
        BuildCache {
            format: BUILD_CACHE_FORMAT,
            statuses,
        }
    }

    /// The cache file's path in the project at `root`.
    pub fn path(root: &Path) -> PathBuf {
        root.join(BUILD_CACHE_FILE)
    }

    /// The file's text: pretty-printed JSON with a final newline, the same
    /// bytes for the same statuses.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// Read the cache of the project at `root`: `Ok(None)` without the
    /// file; W144 when it cannot be read, does not parse, or declares
    /// another format.
    pub fn read(root: &Path) -> Result<Option<Self>, Diagnostic> {
        let path = Self::path(root);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(invalid_cache(&format!("cannot be read: {e}"))),
        };
        let cache: BuildCache = serde_json::from_str(&text)
            .map_err(|e| invalid_cache(&format!("does not parse: {e}")))?;
        if cache.format != BUILD_CACHE_FORMAT {
            return Err(invalid_cache(&format!(
                "declares format {}, but this SpecForge reads format {BUILD_CACHE_FORMAT}",
                cache.format
            )));
        }
        Ok(Some(cache))
    }

    /// Write the cache to the project at `root`, replacing the file
    /// atomically (written beside it, then renamed).
    pub fn write(&self, root: &Path) -> std::io::Result<()> {
        let path = Self::path(root);
        let staging = root.join(format!(".{BUILD_CACHE_FILE}.tmp"));
        std::fs::write(&staging, self.to_json())?;
        std::fs::rename(&staging, &path).inspect_err(|_| {
            let _ = std::fs::remove_file(&staging);
        })
    }
}

/// `specforge check --cache`: write the statuses of `graph` to the project
/// at `root`, unless what check `reported` has an error (a check that
/// fails is not a baseline). Whether the file was written.
pub fn record_build_cache(
    root: &Path,
    graph: &Graph,
    reported: &[Diagnostic],
) -> std::io::Result<bool> {
    if reported.iter().any(|d| d.severity == Severity::Error) {
        return Ok(false);
    }
    BuildCache::of(graph).write(root)?;
    Ok(true)
}

fn invalid_cache(problem: &str) -> Diagnostic {
    Diagnostic::warning(
        "W144",
        format!("build cache '{BUILD_CACHE_FILE}' {problem}; status history is ignored"),
    )
    .with_suggestion(format!(
        "rewrite it with `specforge check --cache`, or delete {BUILD_CACHE_FILE}"
    ))
}
