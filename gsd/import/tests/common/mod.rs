//! Calling `import` in process, as the host would, on a `.planning/` folder on disk.
#![allow(dead_code, reason = "each test crate uses part of it")]

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::testing::{CommandFeed, call_every_command_with};

/// A parity fixture's directory.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity/fixtures")
        .join(name)
}

/// One served file: `path` with `text`, as a glob with text serves it.
pub fn fact(path: &str, text: &str) -> Value {
    json!({
        "path": path,
        "exists": true,
        "bytes": text.len(),
        "lines": text.split('\n').count(),
        "text": text,
    })
}

/// Every file under `dir/.planning`, at any depth, as the facts of the `planning` read, sorted by path.
pub fn planning_facts(dir: &Path) -> Vec<Value> {
    let root = dir.join(".planning");
    let mut files = Vec::new();
    let mut dirs = vec![root.clone()];
    while let Some(dir) = dirs.pop() {
        let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                dirs.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    files
        .iter()
        .map(|path| {
            let rel = path
                .strip_prefix(&root)
                .expect("under .planning")
                .to_string_lossy()
                .replace('\\', "/");
            let text =
                std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            fact(&format!(".planning/{rel}"), &text)
        })
        .collect()
}

/// The feed of `import`: `planning` (facts, and whether the read stopped early) and the manifest's text.
pub fn feed(planning: Vec<Value>, truncated: bool, manifest: Option<&str>) -> CommandFeed {
    let manifest: Vec<Value> = manifest
        .map(|text| fact("specforge.json", text))
        .into_iter()
        .collect();
    CommandFeed {
        files: serde_json::from_value(json!({
            "planning": { "facts": planning, "truncated": truncated },
            "manifest": { "facts": manifest },
        }))
        .expect("the facts are FileFacts"),
        ..Default::default()
    }
}

/// A manifest naming the spec root `spec` and the extensions `loaded`.
pub fn manifest(loaded: &[&str]) -> String {
    json!({ "name": "p", "version": "1.0.0", "spec_root": "spec", "extensions": loaded })
        .to_string()
}

/// `import` over `graph`, fed `feed`.
pub fn import(graph: &CommandGraph, feed: &CommandFeed) -> CommandOutput {
    let outputs =
        call_every_command_with(&gsd_import::build(), graph, "2026-11-24", feed, |_, _| {
            String::new()
        });
    let (_, out) = outputs
        .into_iter()
        .find(|(id, _)| id == "import")
        .expect("@gsd/import declares import");
    out
}

/// The fixture `name`'s `.planning/` imported into an empty graph.
pub fn import_fixture(name: &str) -> CommandOutput {
    let dir = fixture(name);
    let manifest =
        std::fs::read_to_string(dir.join("specforge.json")).expect("the fixture's manifest");
    import(
        &CommandGraph::default(),
        &feed(planning_facts(&dir), false, Some(&manifest)),
    )
}

/// The JSON the command printed.
pub fn payload(out: &CommandOutput) -> Value {
    serde_json::from_str(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", out.stdout))
}
