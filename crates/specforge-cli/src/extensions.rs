use serde_json::json;
use specforge_wasm::{read_lock_file, LockFile};
use std::path::Path;

pub fn run(path: &Path, format: &str) -> i32 {
    let lock_path = path.join("specforge.lock");

    // Read lock file — missing lock file means no extensions installed
    let lock: LockFile = read_lock_file(&lock_path).unwrap_or_default();

    // Sort entries alphabetically by name
    let mut entries = lock.entries.clone();
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    match format {
        "json" => {
            let items: Vec<serde_json::Value> = entries
                .iter()
                .map(|e| {
                    json!({
                        "name": e.name,
                        "version": e.version,
                        "source": e.source,
                    })
                })
                .collect();
            let output = json!({
                "extensions": items,
                "count": items.len(),
            });
            println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
        }
        _ => {
            if entries.is_empty() {
                println!("No extensions installed.");
                println!();
                println!("Install one with: specforge add <extension>");
            } else {
                println!("Installed extensions:");
                println!();
                for entry in &entries {
                    println!("  {} v{} ({})", entry.name, entry.version, entry.source);
                }
                println!();
                println!("{} extension(s) installed.", entries.len());
            }
        }
    }

    0
}
