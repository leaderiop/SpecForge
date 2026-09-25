use serde_json::json;
use specforge_registry::client::credentials::{credentials_path, read_credentials};
use specforge_wasm::{DoctorStatus, LockFile, read_lock_file, run_doctor_check};
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
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output).expect("serialize JSON output")
                    );
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
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).expect("serialize JSON output")
                );
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

    // Registry credential health: expired tokens and unreadable keyring
    // entries break add/publish flows — surface them here before they bite.
    use chrono::Utc as Now;
    let mut credential_lines: Vec<(String, String)> = Vec::new(); // (level, text)
    let mut credential_failures = 0usize;
    let store = read_credentials(&credentials_path()).unwrap_or_default();
    for alias in {
        let mut aliases: Vec<String> = store.registries.keys().cloned().collect();
        aliases.sort();
        aliases
    } {
        match store.get_credential_detail(&alias) {
            Err(diag) => {
                credential_failures += 1;
                credential_lines.push((
                    "error".to_string(),
                    format!("{} — {}", diag.message, diag.suggestion.unwrap_or_default()),
                ));
            }
            Ok(Some(_)) => {
                // Healthy; note an approaching expiry so re-login is not a surprise.
                if let Some(specforge_registry::client::credentials::CredentialEntry::Token {
                    expires_at: Some(expires_at),
                    ..
                }) = store.registries.get(&alias)
                {
                    match assess_expiry(&Some(expires_at.clone()), Now::now()) {
                        Some(TokenExpiry::Expired) => {
                            unreachable!("expired tokens already failed get_credential_detail")
                        }
                        Some(TokenExpiry::ExpiringSoon(days)) => {
                            credential_lines.push((
                                "warning".to_string(),
                                format!(
                                    "registry '{alias}': token expires in {days} day(s) — re-login soon"
                                ),
                            ));
                        }
                        other => {
                            let detail = match other {
                                Some(TokenExpiry::Valid) => String::new(),
                                _ => format!(" (expires {expires_at})"),
                            };
                            credential_lines.push((
                                "ok".to_string(),
                                format!("registry '{alias}': token ok{detail}"),
                            ));
                        }
                    }
                }
            }
            Ok(None) => {}
        }
    }
    let signing_key = specforge_registry::signing::signing_key_path();

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
                    DoctorStatus::StaleHash {
                        name,
                        expected,
                        actual,
                    } => json!({
                        "status": "stale_hash",
                        "name": name,
                        "expected": expected,
                        "actual": actual,
                    }),
                    DoctorStatus::PeerMismatch {
                        name,
                        peer,
                        required,
                    } => json!({
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
                "credentials_failures": credential_failures,
                "credentials": credential_lines
                    .iter()
                    .map(|(level, text)| json!({ "level": level, "text": text }))
                    .collect::<Vec<_>>(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        _ => {
            println!(
                "Extension health check ({} extension(s)):",
                lock.entries.len()
            );
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
                        DoctorStatus::StaleHash {
                            name,
                            expected,
                            actual,
                        } => {
                            println!(
                                "  [STALE] {} — hash mismatch (expected {}, got {})",
                                name,
                                &expected[..8.min(expected.len())],
                                &actual[..8.min(actual.len())]
                            );
                        }
                        DoctorStatus::PeerMismatch {
                            name,
                            peer,
                            required,
                        } => {
                            println!("  [PEER] {} — requires {} v{}", name, peer, required);
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

            if !credential_lines.is_empty() || signing_key.exists() {
                println!();
                println!("Registry credentials:");
                for (level, text) in &credential_lines {
                    let tag = match level.as_str() {
                        "error" => "ERROR",
                        "warning" => "WARN",
                        _ => "ok",
                    };
                    println!("  [{tag}] {text}");
                }
                if signing_key.exists() {
                    println!("  [ok] signing key present");
                } else {
                    println!("  [ok] signing key: not created yet (generated on first publish)");
                }
            }
        }
    }

    if all_healthy && credential_failures == 0 {
        0
    } else {
        1
    }
}

/// Expiry assessment for a stored token timestamp (RFC 3339).
#[derive(Debug, PartialEq)]
enum TokenExpiry {
    Valid,
    ExpiringSoon(i64),
    Expired,
}

/// Within 7 days of expiry (or past it) a token is worth flagging.
const EXPIRY_WARNING_DAYS: i64 = 7;

fn assess_expiry(
    expires_at: &Option<String>,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<TokenExpiry> {
    let deadline = chrono::DateTime::parse_from_rfc3339(expires_at.as_deref()?)
        .ok()?
        .with_timezone(&chrono::Utc);
    let days = (deadline - now).num_days();
    if days < 0 {
        Some(TokenExpiry::Expired)
    } else if days <= EXPIRY_WARNING_DAYS {
        Some(TokenExpiry::ExpiringSoon(days))
    } else {
        Some(TokenExpiry::Valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(days_from_now: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now() + chrono::Duration::days(days_from_now)
    }

    fn stamp(t: chrono::DateTime<chrono::Utc>) -> Option<String> {
        Some(t.to_rfc3339())
    }

    #[test]
    fn expiry_buckets() {
        assert_eq!(
            assess_expiry(&stamp(at(-1)), chrono::Utc::now()),
            Some(TokenExpiry::Expired)
        );
        // "expires now" is nanosecond-ambiguous — accept either side of the boundary.
        assert!(matches!(
            assess_expiry(&stamp(at(0)), chrono::Utc::now()),
            Some(TokenExpiry::Expired) | Some(TokenExpiry::ExpiringSoon(0))
        ));
        // num_days truncates elapsed sub-second precision, so allow ±1 day.
        assert!(matches!(
            assess_expiry(&stamp(at(3)), chrono::Utc::now()),
            Some(TokenExpiry::ExpiringSoon(2)) | Some(TokenExpiry::ExpiringSoon(3))
        ));
        assert_eq!(
            assess_expiry(&stamp(at(30)), chrono::Utc::now()),
            Some(TokenExpiry::Valid)
        );
    }

    #[test]
    fn expiry_handles_missing_or_invalid_timestamps() {
        assert_eq!(assess_expiry(&None, chrono::Utc::now()), None);
        assert_eq!(
            assess_expiry(&Some("not-a-date".to_string()), chrono::Utc::now()),
            None
        );
    }
}
