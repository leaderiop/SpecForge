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
