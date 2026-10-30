//! The citation index (D-r44-19): every upstream file that a `//!` header of `gsd/core/src` says it ports, mapped to the
//! files that say so. A changed upstream file is owned by the files that cite it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ROOTS: [&str; 8] = [
    "src/",
    "hooks/",
    "bin/",
    "agents/",
    "commands/",
    "gsd-core/",
    "capabilities/",
    "tests/",
];
const SUFFIXES: [&str; 8] = [".cts", ".cjs", ".js", ".mjs", ".sh", ".md", ".json", ".ts"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The upstream paths in one header line: a token that starts with an upstream root and ends in a source suffix,
/// with a `:20-73` line range dropped.
fn paths_of(line: &str) -> Vec<String> {
    line.split(|c: char| c.is_whitespace() || "`,;()".contains(c))
        .map(|t| t.trim_end_matches('.'))
        .map(|t| t.split_once(':').map_or(t, |(path, _lines)| path))
        .filter(|t| {
            ROOTS.iter().any(|r| t.starts_with(r)) && SUFFIXES.iter().any(|s| t.ends_with(s))
        })
        .map(String::from)
        .collect()
}

/// `upstream path -> the files of gsd/core (relative to it) whose header cites it`, from the `//!` lines of `src/`
/// that name a gsd-core release.
pub fn index(core: &Path) -> BTreeMap<String, Vec<String>> {
    let mut files = Vec::new();
    rust_files(&core.join("src"), &mut files);
    files.sort();
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("a source file");
        let name = file
            .strip_prefix(core)
            .expect("under gsd/core")
            .display()
            .to_string();
        for line in text
            .lines()
            .take_while(|l| l.starts_with("//!") || l.is_empty())
        {
            if !(line.contains("gsd-core v") || line.contains("Ported from")) {
                continue;
            }
            for path in paths_of(line) {
                let files = found.entry(path).or_default();
                if !files.contains(&name) {
                    files.push(name.clone());
                }
            }
        }
    }
    found
}
