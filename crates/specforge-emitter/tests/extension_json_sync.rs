//! Drift guard: the committed `extensions/<name>/src/{handshake,describe_*}.json`
//! files are GENERATED from the in-process builtin extensions
//! (`crates/specforge-emitter/src/builtins/*.rs`) by
//! `xtask/src/extract_extension_json.rs`. They are then embedded into the
//! `.wasm` extensions that the CLI/LSP/MCP load at runtime.
//!
//! Because the source of truth (Rust) and the runtime artifact (JSON → wasm)
//! are separate, they can silently drift: editing a builtin without
//! re-extracting leaves the JSON — and therefore the wasm — describing a stale
//! contract. This test regenerates every describe payload in-memory and asserts
//! it byte-matches the committed JSON, turning silent divergence into a loud
//! failure with a clear remedy.
//!
//! If this test fails after an intentional builtin change, run:
//!   cargo run -p xtask --bin extract-extension-json
//! then rebuild the affected wasm:
//!   (cd extensions/<name> && cargo build --release --target wasm32-unknown-unknown)

use specforge_emitter::builtins::runtime_for_extensions;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};
use std::path::PathBuf;

/// Repo root = two levels up from this crate's manifest dir (crates/specforge-emitter).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Pretty-print a JSON byte payload exactly as the xtask does, so byte
/// comparison against the committed (pretty-printed) files is meaningful.
fn pretty(bytes: &[u8]) -> String {
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("describe output is valid JSON");
    serde_json::to_string_pretty(&value).expect("re-serialize JSON")
}

#[test]
fn committed_extension_json_matches_builtin_source() {
    let extensions = [
        "@specforge/product",
        "@specforge/software",
        "@specforge/governance",
        "@specforge/formal",
    ];
    let categories = [
        "entities",
        "edges",
        "fields",
        "shared_fields",
        "enhancements",
        "validation_rules",
        "surfaces",
        "passes",
        "feature_flags",
    ];

    let ext_names: Vec<String> = extensions.iter().map(|s| s.to_string()).collect();
    let runtime = runtime_for_extensions(&ext_names);
    let root = repo_root();

    let mut drift: Vec<String> = Vec::new();

    for ext_name in &extensions {
        let dir_name = ext_name.strip_prefix("@specforge/").unwrap();
        let src_dir = root.join("extensions").join(dir_name).join("src");

        // Handshake
        if let WasmCallResult::Ok(bytes) = runtime.call_export(ext_name, "__handshake", &[]) {
            let path = src_dir.join("handshake.json");
            let committed = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("missing committed file: {}", path.display()));
            if pretty(&bytes) != committed.trim_end() {
                drift.push(format!("{} handshake.json", dir_name));
            }
        }

        // Describe payloads
        for cat in &categories {
            let input = format!(r#"{{"category":"{}"}}"#, cat);
            if let WasmCallResult::Ok(bytes) =
                runtime.call_export(ext_name, "__describe", input.as_bytes())
            {
                let path = src_dir.join(format!("describe_{}.json", cat));
                let committed = std::fs::read_to_string(&path)
                    .unwrap_or_else(|_| panic!("missing committed file: {}", path.display()));
                if pretty(&bytes) != committed.trim_end() {
                    drift.push(format!("{} describe_{}.json", dir_name, cat));
                }
            }
        }
    }

    assert!(
        drift.is_empty(),
        "committed extension JSON is out of sync with the builtin source for: {}\n\
         Regenerate with:  cargo run -p xtask --bin extract-extension-json\n\
         then rebuild the affected wasm:  (cd extensions/<name> && cargo build --release --target wasm32-unknown-unknown)",
        drift.join(", ")
    );
}
