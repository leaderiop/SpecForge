//! Every ported asset names its upstream file, version and licence, and the licence is upstream's.

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn every_asset_is_attributed() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let mut all = Vec::new();
    files(&assets, &mut all);
    assert!(all.len() >= 25, "{} assets", all.len());
    for path in all {
        let relative = path.strip_prefix(&assets).expect("under assets");
        let upstream = relative.to_string_lossy();
        if upstream.ends_with(".json") {
            continue; // a JSON file cannot carry the line: `assets.rs` and `tests/assets.rs` cover it
        }
        let text = std::fs::read_to_string(&path).expect("a text asset");
        let first = text.lines().next().unwrap_or_default();
        assert_eq!(
            first,
            format!(
                "<!-- Ported from gsd-core v1.18.0 {upstream}, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->"
            ),
            "{}",
            path.display()
        );
    }
}

#[test]
fn the_licence_is_upstreams() {
    let licence = include_str!("../LICENSE-GSD");
    assert!(licence.starts_with("MIT License\n"), "{licence}");
    assert!(licence.contains("Copyright (c) 2026 Open GSD"));
    assert!(licence.contains("Permission is hereby granted, free of charge"));
    let source = include_str!("../../parity/upstream/v1.18.0/SOURCE.md");
    assert!(
        source.contains("MIT, Copyright (c) 2026 Open GSD"),
        "SOURCE.md names the licence"
    );
}

/// A Rust file ported from upstream says so on its first line, with the version and the licence.
#[test]
fn every_ported_source_is_attributed() {
    let ported = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/complexity");
    let mut all = Vec::new();
    files(&ported, &mut all);
    assert!(all.len() >= 4, "{} ported files", all.len());
    for path in all {
        let text = std::fs::read_to_string(&path).expect("a source");
        let first = text.lines().next().unwrap_or_default();
        assert!(
            first.starts_with("//! Ported from gsd-core v1.18.0 ")
                && first.contains("MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD"),
            "{}: {first}",
            path.display()
        );
    }
}

/// The vendored scorer and its tests are upstream's, where the port's tests say they come from.
#[test]
fn the_vendored_scorer_is_the_file_the_port_names() {
    let upstream = Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/upstream/v1.18.0");
    for file in [
        "src/complexity-trigger.cts",
        "tests/complexity-trigger.test.cjs",
    ] {
        let text = std::fs::read_to_string(upstream.join(file)).expect("a vendored file");
        assert!(text.contains("analyzeSource"), "{file}");
    }
}
