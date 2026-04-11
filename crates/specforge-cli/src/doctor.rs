use serde_json::json;
use specforge_wasm::{read_lock_file, run_doctor_check, DoctorStatus, LockFile};
use std::collections::HashMap;
use std::path::Path;

pub fn run(path: &Path, format: &str) -> i32 {
    let lock_path = path.join("specforge.lock");
    let extensions_dir = path.join(".specforge").join("extensions");

    // Read lock file — missing lock file means nothing to check
    let lock: LockFile = match read_lock_file(&lock_path) {
        Ok(lock) => lock,
        Err(_) => {
            match format {
                "json" => {
                    let output = json!({
                        "status": "healthy",
                        "issues": [],
                        "message": "no lock file found — no extensions to check",
                    });
                    println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
                }
                _ => {
                    println!("No lock file found — no extensions to check.");
                }
            }
            return 0;
        }
    };

    if lock.entries.is_empty() {
        match format {
            "json" => {
                let output = json!({
                    "status": "healthy",
                    "issues": [],
                    "message": "no extensions installed",
                });
                println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
            }
            _ => {
                println!("No extensions installed — nothing to check.");
            }
        }
        return 0;
    }

    // Build installed versions map from lock file entries
    let installed_versions: HashMap<String, String> = lock
        .entries
        .iter()
        .map(|e| (e.name.clone(), e.version.clone()))
        .collect();

    // Run doctor checks with a simple hash function (read file and compute sha256)
    let compute_hash = |wasm_path: &Path| -> Option<String> {
        let bytes = std::fs::read(wasm_path).ok()?;
        Some(specforge_wasm::hex_sha256(&bytes))
    };

    let results = run_doctor_check(&lock, &extensions_dir, compute_hash, &installed_versions);

    // Separate healthy from issues
    let issues: Vec<&DoctorStatus> = results
        .iter()
        .filter(|r| !matches!(r, DoctorStatus::Healthy))
        .collect();

    let all_healthy = issues.is_empty();

    match format {
        "json" => {
            let issue_items: Vec<serde_json::Value> = issues
                .iter()
                .map(|status| match status {
                    DoctorStatus::Healthy => json!({"status": "healthy"}),
                    DoctorStatus::MissingBinary { name } => json!({
                        "status": "missing_binary",
                        "name": name,
                    }),
                    DoctorStatus::StaleHash { name, expected, actual } => json!({
                        "status": "stale_hash",
                        "name": name,
                        "expected": expected,
                        "actual": actual,
                    }),
                    DoctorStatus::PeerMismatch { name, peer, required } => json!({
                        "status": "peer_mismatch",
                        "name": name,
                        "peer": peer,
                        "required": required,
                    }),
                })
                .collect();

            let output = json!({
                "status": if all_healthy { "healthy" } else { "issues_found" },
                "extensions_checked": lock.entries.len(),
                "issues": issue_items,
            });
            println!("{}", serde_json::to_string_pretty(&output).expect("serialize JSON output"));
        }
        _ => {
            println!("Extension health check ({} extension(s)):", lock.entries.len());
            println!();

            if all_healthy {
                println!("  All extensions healthy.");
            } else {
                for status in &issues {
                    match status {
                        DoctorStatus::Healthy => {}
                        DoctorStatus::MissingBinary { name } => {
                            println!("  [MISSING] {} — .wasm binary not found", name);
                        }
                        DoctorStatus::StaleHash { name, expected, actual } => {
                            println!(
                                "  [STALE] {} — hash mismatch (expected {}, got {})",
                                name,
                                &expected[..8.min(expected.len())],
                                &actual[..8.min(actual.len())]
                            );
                        }
                        DoctorStatus::PeerMismatch { name, peer, required } => {
                            println!(
                                "  [PEER] {} — requires {} v{}",
                                name, peer, required
                            );
                        }
                    }
                }
            }

            println!();
            if all_healthy {
                println!("No issues found.");
            } else {
                println!("{} issue(s) found.", issues.len());
            }
        }
    }

    if all_healthy { 0 } else { 1 }
}
