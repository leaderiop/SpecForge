//! Escape hatch for the fresh-clone bootstrap: builds any missing builtin
//! extension wasm on demand. The normal `cargo build` already does this via
//! `crates/specforge-extism/build.rs`; this bin exists so humans (and the
//! wayfinder decision in map #1 / ticket #4) have a named, explicit command.

use std::path::{Path, PathBuf};
use std::process::Command;

const EXTENSIONS: &[(&str, &str)] = &[
    ("product", "specforge_ext_product.wasm"),
    ("software", "specforge_ext_software.wasm"),
    ("governance", "specforge_ext_governance.wasm"),
    ("formal", "specforge_ext_formal.wasm"),
];

const TARGET: &str = "wasm32-unknown-unknown";

fn main() {
    let force = std::env::args().any(|a| a == "--force");
    let mut failed = false;
    let root = std::env::var("CARGO_MANIFEST_DIR")
        .map(|m| {
            Path::new(&m)
                .join("..")
                .canonicalize()
                .expect("repo root resolves")
        })
        .expect("CARGO_MANIFEST_DIR is set when run via cargo xtask");
    for (name, artifact) in EXTENSIONS {
        let blob: PathBuf = root.join(format!(
            "extensions/{name}/target/{TARGET}/release/{artifact}"
        ));
        if blob.exists() && !force {
            println!("[skip] {name}: {} exists", blob.display());
            continue;
        }
        println!(
            "[build] {name} for {TARGET}{}",
            if force { " (forced)" } else { "" }
        );
        let status = Command::new("cargo")
            .args(["build", "--release", "--target", TARGET])
            .current_dir(root.join(format!("extensions/{name}")))
            .status()
            .unwrap_or_else(|e| {
                panic!("failed to spawn cargo while building builtin '{name}': {e}")
            });
        if status.success() && blob.exists() {
            println!("[ok]    {name}");
        } else {
            eprintln!(
                "[fail]  {name}: build did not produce {}\nIf the target is missing, run: rustup target add {TARGET}",
                blob.display()
            );
            failed = true;
        }
    }
    std::process::exit(u8::from(failed).into());
}
