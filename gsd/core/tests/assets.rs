//! The embedded workflow assets (D-r42-2): every file under `assets/` that is not an agent body is in the table
//! `src/prompts/assets.rs`, and the reverse; each Markdown file names its upstream path on its first line, and each
//! JSON file (which cannot carry the line) is byte-equal to the vendored copy.

use std::path::{Path, PathBuf};

const TABLE: &str = include_str!("../src/prompts/assets.rs");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The paths the table's `asset!("…")` lines name.
fn table() -> Vec<String> {
    TABLE
        .lines()
        .filter_map(|l| l.trim().strip_prefix("asset!(\""))
        .filter_map(|l| l.strip_suffix("\"),"))
        .map(String::from)
        .collect()
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("a folder").flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(base, &path, out);
        } else {
            let relative = path.strip_prefix(base).expect("under base");
            out.push(relative.to_string_lossy().into_owned());
        }
    }
}

#[test]
fn assets_embed_and_attribute() {
    let base = root().join("assets");
    let mut on_disk = Vec::new();
    walk(&base, &base, &mut on_disk);
    on_disk.retain(|p| !p.starts_with("agents/"));
    on_disk.sort();
    let table = table();
    assert_eq!(table, on_disk, "the table and the assets/ folder differ");
    let mut sorted = table.clone();
    sorted.sort();
    assert_eq!(
        table, sorted,
        "the table must stay sorted: `get` searches it"
    );
    for path in &table {
        let text = std::fs::read_to_string(base.join(path)).expect("an asset");
        if path.ends_with(".json") {
            let vendored = root().join("../parity/upstream/v1.18.0").join(path);
            assert_eq!(
                text,
                std::fs::read_to_string(&vendored).expect("the vendored copy"),
                "{path}"
            );
        } else {
            let first = text.lines().next().unwrap_or_default();
            assert_eq!(
                first,
                format!(
                    "<!-- Ported from gsd-core v1.18.0 {path}, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->"
                ),
                "{path}"
            );
        }
    }
}
