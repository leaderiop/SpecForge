use crate::OutputFormat;
use serde_json::json;
use specforge_wasm::{LockFile, read_lock_file};
use std::path::Path;

pub fn run(path: &Path, format: OutputFormat) -> i32 {
    let lock_path = path.join("specforge.lock");

    // Read lock file — missing lock file means no installed (downloaded) extensions
    let lock: LockFile = read_lock_file(&lock_path).unwrap_or_default();

    // Sort entries alphabetically by name
    let mut entries = lock.entries.clone();
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    // Builtins enabled in specforge.json ship with the binary: no lock entry.
    let mut builtins = crate::builtins::enabled(path);
    builtins.sort_unstable();
    builtins.dedup();

    match format {
        OutputFormat::Json => {
            let items: Vec<serde_json::Value> = builtins
                .iter()
                .map(|name| {
                    json!({
                        "name": name,
                        "version": env!("CARGO_PKG_VERSION"),
                        "source": "builtin",
                    })
                })
                .chain(entries.iter().map(|e| {
                    json!({
                        "name": e.name,
                        "version": e.version,
                        "source": e.source,
                    })
                }))
                .collect();
            let output = json!({
                "extensions": items,
                "count": items.len(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            if entries.is_empty() && builtins.is_empty() {
                println!("No extensions installed.");
                println!();
                println!("Install one with: specforge add <extension>");
            } else {
                println!("Installed extensions:");
                println!();
                for name in &builtins {
                    println!("  {} (builtin)", name);
                }
                for entry in &entries {
                    println!("  {} v{} ({})", entry.name, entry.version, entry.source);
                }
                println!();
                println!("{} extension(s) installed.", builtins.len() + entries.len());
            }
        }
    }

    0
}
