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

    // What each extension provides: its entity kinds (from the compiled
    // KindRegistry) and how many of the project's entities use them.
    let ctx = (!builtins.is_empty() || !entries.is_empty()).then(|| crate::pipeline::compile(path));
    let provides = |name: &str| -> (Vec<String>, usize) {
        let Some(ctx) = &ctx else {
            return (Vec::new(), 0);
        };
        let mut kinds: Vec<String> = ctx
            .kind_registry
            .iter()
            .filter(|(_, entry)| entry.source_extension == name)
            .map(|(kind, _)| kind.clone())
            .collect();
        kinds.sort();
        let count = ctx
            .graph
            .nodes()
            .iter()
            .filter(|n| kinds.iter().any(|k| k.as_str() == n.kind.raw.as_str()))
            .count();
        (kinds, count)
    };

    match format {
        OutputFormat::Json => {
            let items: Vec<serde_json::Value> = builtins
                .iter()
                .map(|name| {
                    let (kinds, count) = provides(name);
                    json!({
                        "name": name,
                        "version": env!("CARGO_PKG_VERSION"),
                        "source": "builtin",
                        "entity_kinds": kinds,
                        "entity_count": count,
                    })
                })
                .chain(entries.iter().map(|e| {
                    let (kinds, count) = provides(&e.name);
                    json!({
                        "name": e.name,
                        "version": e.version,
                        "source": e.source,
                        "entity_kinds": kinds,
                        "entity_count": count,
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
                let describe = |name: &str| {
                    let (kinds, count) = provides(name);
                    if kinds.is_empty() {
                        String::new()
                    } else {
                        format!(": {count} entities ({})", kinds.join(", "))
                    }
                };
                for name in &builtins {
                    println!("  {} (builtin){}", name, describe(name));
                }
                for entry in &entries {
                    println!(
                        "  {} v{} ({}){}",
                        entry.name,
                        entry.version,
                        entry.source,
                        describe(&entry.name)
                    );
                }
                println!();
                println!("{} extension(s) installed.", builtins.len() + entries.len());
            }
        }
    }

    0
}
