use crate::OutputFormat;
use serde_json::json;
use specforge_wasm::{read_lock_file, uninstall_extension, write_lock_file};
use std::path::Path;

pub fn run(name: &str, path: &Path, force: bool, format: OutputFormat) -> i32 {
    let lock_path = path.join("specforge.lock");

    // A locked install of that name wins; otherwise a builtin name disables the builtin.
    let locked =
        read_lock_file(&lock_path).is_ok_and(|lock| lock.entries.iter().any(|e| e.name == name));
    if !locked && let Some(builtin) = crate::builtins::builtin_name(name) {
        return disable_builtin(builtin, path, format);
    }
    let extensions_dir = path.join(".specforge").join("extensions");

    // 1. Read lock file (missing lock file means nothing to remove)
    let mut lock = match read_lock_file(&lock_path) {
        Ok(lock) => lock,
        Err(_) => {
            match format {
                OutputFormat::Json => {
                    let output = json!({
                        "error": format!("extension '{}' is not installed (no lock file found)", name),
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output).expect("serialize JSON output")
                    );
                }
                OutputFormat::Human => {
                    eprintln!(
                        "error: extension '{}' is not installed (no lock file found)",
                        name
                    );
                }
            }
            return 1;
        }
    };

    // Check if extension is in the lock file
    if !lock.entries.iter().any(|e| e.name == name) {
        match format {
            OutputFormat::Json => {
                let output = json!({
                    "error": format!("extension '{}' is not installed", name),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).expect("serialize JSON output")
                );
            }
            OutputFormat::Human => {
                eprintln!("error: extension '{}' is not installed", name);
            }
        }
        return 1;
    }

    // 2. Uninstall (no manifests available for peer dep checks in CLI context)
    let installed_manifests = Vec::new();
    match uninstall_extension(
        name,
        &installed_manifests,
        &extensions_dir,
        &mut lock,
        force,
    ) {
        Ok(result) => {
            // 3. Write updated lock file
            if let Err(diag) = write_lock_file(&lock, &lock_path) {
                match format {
                    OutputFormat::Json => {
                        let output = json!({
                            "error": diag.message,
                        });
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&output).expect("serialize JSON output")
                        );
                    }
                    OutputFormat::Human => {
                        eprintln!("error: {}", diag.message);
                    }
                }
                return 1;
            }

            // 4. Report success
            match format {
                OutputFormat::Json => {
                    let output = json!({
                        "removed": result.name,
                        "version": result.version,
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output).expect("serialize JSON output")
                    );
                }
                OutputFormat::Human => {
                    println!("Removed extension '{}' (v{})", result.name, result.version);
                }
            }
            0
        }
        Err(diag) => {
            match format {
                OutputFormat::Json => {
                    let output = json!({
                        "error": diag.message,
                        "code": diag.code,
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output).expect("serialize JSON output")
                    );
                }
                OutputFormat::Human => {
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

/// Builtins are embedded in the binary: disabling one only edits specforge.json.
fn disable_builtin(name: &str, path: &Path, format: OutputFormat) -> i32 {
    let not_installed = format!("extension '{}' is not installed", name);
    let error = if !path.join("specforge.json").exists() {
        Some(format!("{not_installed} (no specforge.json found)"))
    } else {
        match crate::builtins::disable(path, name) {
            Ok(true) => None,
            Ok(false) => Some(not_installed),
            Err(message) => Some(message),
        }
    };
    match (error, format) {
        (Some(message), OutputFormat::Json) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"error": message}))
                    .expect("serialize JSON output")
            );
            1
        }
        (Some(message), OutputFormat::Human) => {
            eprintln!("error: {}", message);
            1
        }
        (None, OutputFormat::Json) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"removed": name, "source": "builtin"}))
                    .expect("serialize JSON output")
            );
            0
        }
        (None, OutputFormat::Human) => {
            println!("Disabled builtin extension '{}'", name);
            0
        }
    }
}
