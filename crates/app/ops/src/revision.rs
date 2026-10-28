//! A file's revision (ADR 0289): `sha256:<64 hex>` of its bytes as the compile read them, or `absent` for a
//! file the compile holds no text of. A writing command receives the revision of every source and names it on
//! each op, so the host can refuse a plan made over a text that has since changed (`stale_revision`).

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use specforge_project::CompiledProject;

/// The revision of a file the compile holds no text of.
pub const ABSENT: &str = "absent";

/// The revision of `text`.
pub fn of_text(text: &str) -> String {
    of_bytes(text.as_bytes())
}

/// The revision of a file holding `bytes`.
pub fn of_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("sha256:{hex}")
}

/// The revision of `path` (relative to the spec root) in `project`.
pub fn of_file(project: &CompiledProject, path: &str) -> String {
    project
        .source_text(path)
        .map_or_else(|| ABSENT.to_string(), |text| of_text(&text))
}

/// The revision of every source `project` holds text of, by key.
pub fn of_project(project: &CompiledProject) -> BTreeMap<String, String> {
    project
        .source_texts()
        .into_iter()
        .map(|(key, text)| (key, of_text(&text)))
        .collect()
}
