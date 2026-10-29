//! The ported workflows, references, templates and command files against the vendored upstream files (gsd-core
//! v1.18.0, D-r42-4): line for line, a line may differ only where upstream's names `.planning`, `gsd-tools`,
//! `gsd_run` or `gsd-sdk`, or where a link left the tree (`REWRITTEN`).

use std::path::{Path, PathBuf};

use regex::Regex;

#[path = "common/port.rs"]
mod rule;
use rule::check_lines;

/// Body lines (0-based) that differ although upstream's names no ported tool: a relative link that leaves the tree
/// became the absolute URL of the file at the tag, or where a rewrite fixed a line that names no ported tool but still
/// told the agent something false.
const REWRITTEN: &[(&str, usize)] = &[
    ("gsd-core/references/agent-skills-bootstrap.md", 9),
    ("gsd-core/references/loop-hook-dispatch.md", 133),
    ("gsd-core/references/loop-hook-dispatch.md", 142),
    ("gsd-core/references/mvp-concepts.md", 4),
    // REVIEW-r42 C1: upstream deletes REQUIREMENTS.md after archiving it; the requirement entities stay.
    ("gsd-core/workflows/complete-milestone.md", 23),
    ("gsd-core/workflows/complete-milestone.md", 279),
    ("gsd-core/workflows/complete-milestone.md", 544),
    ("gsd-core/workflows/complete-milestone.md", 555),
    ("gsd-core/workflows/complete-milestone.md", 556),
    ("gsd-core/workflows/complete-milestone.md", 663),
    ("gsd-core/workflows/complete-milestone.md", 666),
    ("gsd-core/workflows/complete-milestone.md", 669),
    ("gsd-core/workflows/complete-milestone.md", 743),
    ("gsd-core/workflows/complete-milestone.md", 744),
    // REVIEW-r42 M4: a shell line whose path had been swapped for prose.
    ("gsd-core/workflows/complete-milestone.md", 220),
    ("gsd-core/workflows/complete-milestone.md", 254),
    ("gsd-core/workflows/transition.md", 222),
];

/// `(path, first, last)`: runs of body lines (0-based, inclusive) rewritten because the thing they describe does not
/// exist here: a state table, a directory, a batch router. Each run opens with a line that says what replaced it.
const REWRITTEN_SPANS: &[(&str, usize, usize)] = &[
    ("commands/gsd/pause-work.md", 11, 11),
    ("commands/gsd/pause-work.md", 31, 31),
    ("commands/gsd/pause-work.md", 33, 33),
    ("commands/gsd/pause-work.md", 6, 6),
    ("commands/gsd/resume-work.md", 6, 7),
    ("gsd-core/workflows/autonomous.md", 166, 167),
    ("gsd-core/workflows/autonomous.md", 292, 292),
    ("gsd-core/workflows/autonomous.md", 613, 615),
    ("gsd-core/workflows/autonomous.md", 676, 676),
    ("gsd-core/workflows/fast.md", 78, 82),
    ("gsd-core/workflows/fast.md", 85, 85),
    ("gsd-core/workflows/fast.md", 87, 89),
    ("gsd-core/workflows/fast.md", 92, 92),
    ("gsd-core/workflows/pause-work.md", 103, 113),
    ("gsd-core/workflows/pause-work.md", 116, 116),
    ("gsd-core/workflows/pause-work.md", 118, 118),
    ("gsd-core/workflows/pause-work.md", 139, 139),
    ("gsd-core/workflows/pause-work.md", 223, 223),
    ("gsd-core/workflows/pause-work.md", 239, 239),
    ("gsd-core/workflows/pause-work.md", 259, 259),
    ("gsd-core/workflows/pause-work.md", 78, 101),
    ("gsd-core/workflows/quick-batch/steps/batch-init.md", 21, 21),
    ("gsd-core/workflows/quick.md", 141, 142),
    ("gsd-core/workflows/quick.md", 254, 254),
    ("gsd-core/workflows/quick.md", 265, 265),
    ("gsd-core/workflows/quick.md", 637, 637),
    ("gsd-core/workflows/quick.md", 639, 639),
    ("gsd-core/workflows/quick.md", 641, 641),
    ("gsd-core/workflows/quick.md", 643, 643),
    ("gsd-core/workflows/quick.md", 645, 645),
    ("gsd-core/workflows/quick.md", 647, 647),
    ("gsd-core/workflows/quick.md", 649, 651),
    ("gsd-core/workflows/quick.md", 653, 655),
    ("gsd-core/workflows/quick.md", 657, 659),
    ("gsd-core/workflows/quick.md", 661, 663),
    ("gsd-core/workflows/quick.md", 667, 667),
    ("gsd-core/workflows/quick.md", 670, 674),
    ("gsd-core/workflows/quick.md", 676, 677),
    ("gsd-core/workflows/quick.md", 679, 679),
    ("gsd-core/workflows/quick.md", 681, 682),
    ("gsd-core/workflows/quick.md", 684, 684),
    ("gsd-core/workflows/quick.md", 686, 690),
    ("gsd-core/workflows/quick.md", 692, 692),
    ("gsd-core/workflows/quick.md", 694, 694),
    ("gsd-core/workflows/quick.md", 696, 699),
    ("gsd-core/workflows/quick.md", 790, 790),
    ("gsd-core/workflows/resume-project.md", 103, 106),
    ("gsd-core/workflows/resume-project.md", 115, 115),
    ("gsd-core/workflows/resume-project.md", 117, 124),
    ("gsd-core/workflows/resume-project.md", 126, 126),
    ("gsd-core/workflows/resume-project.md", 129, 129),
    ("gsd-core/workflows/resume-project.md", 158, 158),
    ("gsd-core/workflows/resume-project.md", 186, 192),
    ("gsd-core/workflows/resume-project.md", 198, 200),
    ("gsd-core/workflows/resume-project.md", 202, 202),
    ("gsd-core/workflows/resume-project.md", 304, 304),
    ("gsd-core/workflows/resume-project.md", 306, 306),
    ("gsd-core/workflows/resume-project.md", 308, 309),
    ("gsd-core/workflows/resume-project.md", 311, 314),
    ("gsd-core/workflows/resume-project.md", 316, 316),
    ("gsd-core/workflows/resume-project.md", 322, 322),
    ("gsd-core/workflows/resume-project.md", 324, 324),
    ("gsd-core/workflows/resume-project.md", 326, 328),
    ("gsd-core/workflows/resume-project.md", 330, 330),
    ("gsd-core/workflows/resume-project.md", 332, 332),
    ("gsd-core/workflows/resume-project.md", 334, 334),
    ("gsd-core/workflows/resume-project.md", 336, 337),
    ("gsd-core/workflows/resume-project.md", 353, 353),
    ("gsd-core/workflows/resume-project.md", 358, 358),
    ("gsd-core/workflows/resume-project.md", 82, 83),
    ("gsd-core/workflows/resume-project.md", 85, 87),
    ("gsd-core/workflows/resume-project.md", 90, 91),
    ("gsd-core/workflows/resume-project.md", 93, 96),
    ("gsd-core/workflows/resume-project.md", 98, 99),
    ("gsd-core/workflows/session-report.md", 57, 57),
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
                || REWRITTEN_SPANS
                    .iter()
                    .any(|(p, a, b)| *p == path && (*a..=*b).contains(&n))
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
