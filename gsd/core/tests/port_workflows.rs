//! The ported workflows, references, templates and command files against the vendored upstream files (gsd-core
//! v1.18.0, D-r42-4): line for line, a line may differ only where upstream's names `.planning`, `gsd-tools`,
//! `gsd_run` or `gsd-sdk`, or where a link left the tree (`REWRITTEN`).

use std::path::{Path, PathBuf};

use regex::Regex;

#[path = "common/port.rs"]
mod rule;
use rule::check_lines;

/// Body lines (0-based) that differ although upstream's names no ported tool: a relative link that leaves the tree
/// became the absolute URL of the file at the tag.
const REWRITTEN: &[(&str, usize)] = &[
    ("gsd-core/references/agent-skills-bootstrap.md", 9),
    ("gsd-core/references/loop-hook-dispatch.md", 133),
    ("gsd-core/references/loop-hook-dispatch.md", 142),
    ("gsd-core/references/mvp-concepts.md", 4),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn vendored_root() -> PathBuf {
    root().join("../parity/upstream/v1.18.0")
}

/// The files under `dir` as paths relative to `base`, sorted.
fn files(base: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(base, &path, out);
        } else {
            let relative = path.strip_prefix(base).expect("under base");
            out.push(relative.to_string_lossy().into_owned());
        }
    }
    out.sort();
}

/// The embedded files that are not agent bodies.
pub fn assets() -> Vec<String> {
    let base = root().join("assets");
    let mut all = Vec::new();
    files(&base, &base, &mut all);
    all.retain(|p| !p.starts_with("agents/"));
    all
}

/// The vendored copy of an asset: upstream's own name, or `<name>.txt` for a file whose links leave the tree.
fn vendored(path: &str) -> PathBuf {
    let plain = vendored_root().join(path);
    if plain.is_file() {
        plain
    } else {
        vendored_root().join(format!("{path}.txt"))
    }
}

/// The body of an upstream file: a command file loses its frontmatter, which the slash binding carries.
fn body(path: &str) -> Vec<String> {
    let file = vendored(path);
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    if path.starts_with("commands/") {
        assert_eq!(lines[0], "---", "{path} has no frontmatter");
        let end = lines[1..]
            .iter()
            .position(|l| l == "---")
            .expect("a closing ---")
            + 1;
        lines.drain(..=end);
    }
    lines
}

#[test]
fn workflow_assets_diff_only_in_ported_passages() {
    let rx = Regex::new(r"\.planning|gsd-tools|gsd_run|gsd-sdk").expect("a regex");
    let mut checked = 0;
    for path in assets().into_iter().filter(|p| p.ends_with(".md")) {
        let asset = std::fs::read_to_string(root().join("assets").join(&path)).expect("an asset");
        check_lines(&path, &path, &asset, &body(&path), &rx, &|n| {
            REWRITTEN.contains(&(path.as_str(), n))
        });
        checked += 1;
    }
    assert!(checked > 0, "no workflow asset was checked");
}

#[test]
fn the_vendored_tree_lists_every_asset() {
    let source = std::fs::read_to_string(vendored_root().join("SOURCE.md")).expect("SOURCE.md");
    let mut vendored_files = Vec::new();
    for dir in ["gsd-core", "commands"] {
        let base = vendored_root();
        files(&base, &base.join(dir), &mut vendored_files);
    }
    let named: Vec<String> = vendored_files
        .iter()
        .map(|p| p.strip_suffix(".txt").unwrap_or(p).to_string())
        .collect();
    let assets = assets();
    for path in &assets {
        assert!(named.contains(path), "{path} has no vendored copy");
        assert!(
            source.contains(path.as_str()),
            "SOURCE.md does not list {path}"
        );
    }
    for path in &named {
        assert!(assets.contains(path), "{path} is vendored but not an asset");
    }
}
