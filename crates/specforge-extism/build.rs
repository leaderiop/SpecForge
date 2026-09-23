//! Fresh-clone bootstrap for the four embedded builtin extension blobs.
//!
//! `builtins.rs` `include_bytes!`s artifacts from
//! `extensions/<name>/target/wasm32-unknown-unknown/release/` — paths that only
//! exist after each extension crate has been cross-compiled. This guard builds
//! any missing blob during the normal `cargo build`, so a fresh clone compiles
//! without a manual bootstrap step.
//!
//! Decision (wayfinder map #1, ticket #4):
//! - skip blobs that already exist (no incremental rebuild of extensions here;
//!   CI's clean-checkout job is what keeps the guard honest);
//! - never auto-install toolchains: a missing target fails with the exact
//!   `rustup` command to run.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const EXTENSIONS: &[(&str, &str)] = &[
    ("product", "specforge_ext_product.wasm"),
    ("software", "specforge_ext_software.wasm"),
    ("governance", "specforge_ext_governance.wasm"),
    ("formal", "specforge_ext_formal.wasm"),
];

const TARGET: &str = "wasm32-unknown-unknown";

fn workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo");
    Path::new(&manifest_dir)
        .join("../../")
        .canonicalize()
        .expect("workspace root resolves")
}

fn blob_path(root: &Path, name: &str, artifact: &str) -> PathBuf {
    root.join(format!(
        "extensions/{name}/target/{TARGET}/release/{artifact}"
    ))
}

fn build_extension(root: &Path, name: &str) {
    println!("cargo:warning=building builtin extension '{name}' for {TARGET} (first build only)");
    let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "--release", "--target", TARGET])
        .current_dir(root.join(format!("extensions/{name}")))
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn cargo while building builtin '{name}': {e}"));
    if !status.success() {
        panic!(
            "builtin extension '{name}' failed to build for {TARGET}.\n\
             If the target is missing, run: rustup target add {TARGET}"
        );
    }
}

fn main() {
    let root = workspace_root();
    for (name, artifact) in EXTENSIONS {
        let blob = blob_path(&root, name, artifact);
        if blob.exists() {
            continue;
        }
        build_extension(&root, name);
        assert!(
            blob.exists(),
            "builtin '{name}' reported success but {} is still missing",
            blob.display()
        );
    }
}
