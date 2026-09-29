//! Writes each builtin's live `__handshake` and `__describe` payloads to
//! `extensions/<name>/src/{handshake,describe_<category>}.json`.
//!
//! Extensions that build their contributions in code (rather than serving
//! checked-in JSON through `raw_category`) use these snapshots as their
//! protocol fixtures; the conformance tests parse them against the protocol
//! types. Run after changing such an extension and rebuilding its blob:
//!
//!   cargo run -p xtask --bin snapshot-builtins -- testing

use specforge_component::ComponentRuntime;
use specforge_component::builtins::{BUILTIN_EXTENSIONS, load_builtins_for};
use specforge_wasm::protocol::{ProtocolHost, SUPPORTED_CATEGORIES};
use std::path::Path;

fn main() {
    let names: Vec<String> = std::env::args().skip(1).collect();
    if names.is_empty() {
        eprintln!("usage: snapshot-builtins <extension-dir-name>...   (e.g. testing)");
        std::process::exit(2);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the workspace root");

    let runtime = ComponentRuntime::new();
    let mut failed = false;
    for dir_name in &names {
        let extension = format!("@specforge/{dir_name}");
        if !BUILTIN_EXTENSIONS
            .iter()
            .any(|(name, _)| *name == extension)
        {
            eprintln!("[fail]   {dir_name}: {extension} is not a builtin");
            failed = true;
            continue;
        }
        if let Err(e) = load_builtins_for(&runtime, std::slice::from_ref(&extension)) {
            eprintln!("[fail]   {dir_name}: {e}");
            failed = true;
            continue;
        }
        let host = ProtocolHost::new(&runtime);
        let src = root.join("extensions").join(dir_name).join("src");

        let handshake = match host.handshake(&extension) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[fail]   {dir_name}: handshake: {e}");
                failed = true;
                continue;
            }
        };
        write_json(&src.join("handshake.json"), &handshake);

        for category in SUPPORTED_CATEGORIES {
            let path = src.join(format!("describe_{category}.json"));
            match host.describe(&extension, category) {
                // Empty categories get no fixture (and lose a stale one).
                Ok(response) if response.items.as_array().is_some_and(|a| a.is_empty()) => {
                    let _ = std::fs::remove_file(&path);
                }
                Ok(response) => write_json(&path, &response),
                Err(e) => eprintln!("[skip]   {dir_name}: describe {category}: {e}"),
            }
        }
        println!(
            "[ok]     {dir_name}: snapshots written to {}",
            src.display()
        );
    }
    if failed {
        std::process::exit(1);
    }
}

fn write_json(path: &Path, value: &impl serde::Serialize) {
    let json = serde_json::to_string_pretty(value).expect("protocol payloads serialize");
    std::fs::write(path, json + "\n")
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}
