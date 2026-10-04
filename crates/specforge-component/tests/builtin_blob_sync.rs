//! Fast local drift guard for the vendored builtin wasm blobs.
//!
//! `extensions/<name>/wasm/<blob>.wasm` is committed to the repo and
//! embedded into `specforge-component` via `include_bytes!`. The blobs embed
//! each extension's `describe_*.json` payloads verbatim, so a manifest change
//! without a re-vendor fails here. Rust source drift is invisible to this
//! test; CI's `build-builtins --check` covers every input.
//!
//! Refresh flow: `cargo run -p xtask --bin build-builtins -- --install`.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

const EXTENSIONS: &[&str] = &[
    "product",
    "software",
    "governance",
    "formal",
    "rust",
    "typescript",
];

#[test]
fn vendored_builtin_blobs_embed_current_manifest_payloads() {
    let root = repo_root();
    let mut checked = 0usize;

    for ext in EXTENSIONS {
        let blob_path = root.join(format!("extensions/{ext}/wasm/specforge_ext_{ext}.wasm"));
        let blob = std::fs::read(&blob_path)
            .unwrap_or_else(|e| panic!("vendored blob missing: {}: {e}", blob_path.display()));

        let src = root.join(format!("extensions/{ext}/src"));
        let mut jsons = Vec::new();
        for entry in std::fs::read_dir(&src).expect("guest src dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                jsons.push(path);
            }
        }
        assert!(
            !jsons.is_empty(),
            "{}: no describe payloads found",
            src.display()
        );

        // handshake.json is re-serialized by the guest's SDK macro (never
        // embedded verbatim), and the formal pilot describes its passes from
        // `#[compiler_pass]` registrations rather than the static file.
        let exempt: &[&str] = match *ext {
            "formal" => &["handshake.json", "describe_passes.json"],
            _ => &["handshake.json"],
        };
        for json_path in jsons {
            if exempt.contains(&json_path.file_name().unwrap().to_str().unwrap()) {
                continue;
            }
            let payload = std::fs::read(&json_path).expect("read describe payload");
            assert!(
                contains_subslice(&blob, &payload),
                "{}: vendored blob is stale — it does not embed the current {}. \
                 Re-vendor: cargo run -p xtask --bin build-builtins -- --install",
                blob_path.display(),
                json_path.display(),
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 33,
        "expected >= 33 payload checks (8 per existing guest, formal -2, \
         rust/typescript +1 each; surfaces are declared in code), got {checked}"
    );
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}
