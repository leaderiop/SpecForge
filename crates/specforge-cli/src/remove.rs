use serde_json::json;
use specforge_wasm::{read_lock_file, uninstall_extension, write_lock_file};
use std::path::Path;

pub fn run(name: &str, path: &Path, force: bool, format: &str) -> i32 {
    let lock_path = path.join("specforge.lock");
    let extensions_dir = path.join(".specforge").join("extensions");
    let cache_dir = path.join(".specforge").join("cache");

    // 1. Read lock file (missing lock file means nothing to remove)
    let mut lock = match read_lock_file(&lock_path) {
        Ok(lock) => lock,
        Err(_) => {
            match format {
                "json" => {
                    let output = json!({
                        "error": format!("extension '{}' is not installed (no lock file found)", name),
                    });
                    println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
                }
                _ => {
                    eprintln!("error: extension '{}' is not installed (no lock file found)", name);
                }
            }
            return 1;
        }
    };

    // Check if extension is in the lock file
    if !lock.entries.iter().any(|e| e.name == name) {
        match format {
            "json" => {
                let output = json!({
                    "error": format!("extension '{}' is not installed", name),
                });
                println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
            }
            _ => {
                eprintln!("error: extension '{}' is not installed", name);
            }
        }
        return 1;
    }

    // 2. Uninstall (no manifests available for peer dep checks in CLI context)
    let installed_manifests = Vec::new();
    match uninstall_extension(name, &installed_manifests, &extensions_dir, &cache_dir, &mut lock, force) {
        Ok(result) => {
            // 3. Write updated lock file
            if let Err(diag) = write_lock_file(&lock, &lock_path) {
                match format {
                    "json" => {
                        let output = json!({
                            "error": diag.message,
                        });
                        println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
                    }
                    _ => {
                        eprintln!("error: {}", diag.message);
                    }
                }
                return 1;
            }

            // 4. Report success
            match format {
                "json" => {
                    let output = json!({
                        "removed": result.name,
                        "version": result.version,
                        "cache_invalidated": result.cache_invalidated,
                    });
                    println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
                }
                _ => {
                    println!("Removed extension '{}' (v{})", result.name, result.version);
                    if result.cache_invalidated {
                        println!("  AOT cache invalidated");
                    }
                }
            }
            0
        }
        Err(diag) => {
            match format {
                "json" => {
                    let output = json!({
                        "error": diag.message,
                        "code": diag.code,
                    });
                    println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
                }
                _ => {
                    eprintln!("error: {}", diag.message);
                    if let Some(suggestion) = &diag.suggestion {
                        eprintln!("  hint: {}", suggestion);
                    }
                }
            }
            1
        }
    }
}
