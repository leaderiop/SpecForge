//! `specforge doctor`: one project health report, shared by the CLI command
//! and the MCP `specforge.doctor` tool so both surfaces say the same thing.
//!
//! The report is built from what a compile already produced (the loaded
//! manifests and the diagnostics) plus the project's lock file on disk:
//! enabled extensions with their enhancement counts, enhancements grouped
//! by the entity kind they target, extension conflicts with a resolution
//! suggestion, keywords that shadow an entity kind, extensions that failed
//! to load, installed-binary integrity, and whether the z3 solver is on PATH.
//!
//! Registry credential health ([`credential_health`]) is the user's, not the
//! project's: `specforge doctor` reports it; the MCP tool, whose spec'd
//! report (`McpDoctorReport`) is about the project, does not.

use serde::Serialize;
use specforge_common::{Diagnostic, Severity};
use specforge_registry::ManifestV2;
use specforge_wasm::{DoctorStatus, read_lock_file, run_doctor_check};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Diagnostic codes that mean two contributions collide.
pub const CONFLICT_CODES: [&str; 6] = ["E017", "E018", "E023", "E026", "E057", "W018"];

/// Codes that mean a name shadows a grammar-level construct: E013 (a project
/// entity ID is a structural keyword or an extension's kind keyword), E023
/// (an extension's entity kind collides with a structural keyword) and E026
/// (a kind keyword is registered twice). E023 and E026 are also conflicts.
pub const SHADOWING_CODES: [&str; 3] = ["E013", "E023", "E026"];

/// Codes that mean an enabled extension did not load: E028 (not installed,
/// or its protocol load failed) and E033 (its installed binary no longer
/// matches the lock file's hash).
pub const LOAD_FAILURE_CODES: [&str; 2] = ["E028", "E033"];

/// Everything `specforge doctor` reports about a project.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    /// Every enabled extension: builtins from `specforge.json` and lock entries.
    pub extensions: Vec<ExtensionHealth>,
    /// Entity enhancements keyed by the entity kind they target (sorted).
    pub enhancements: BTreeMap<String, Vec<EnhancementEntry>>,
    /// Extension conflicts the compile reported, in diagnostic order.
    pub conflicts: Vec<Conflict>,
    /// Names shadowing a grammar-level construct (see [`SHADOWING_CODES`]).
    pub shadowed: Vec<ShadowedConstruct>,
    /// Enabled extensions the compile could not load (see
    /// [`LOAD_FAILURE_CODES`]), in diagnostic order.
    pub load_failures: Vec<LoadFailure>,
    /// Installed binaries that do not match the lock file.
    pub issues: Vec<BinaryIssue>,
    /// Lock entries whose binaries were checked.
    pub extensions_checked: usize,
    /// `stale` when an installed binary is missing or its hash drifted.
    pub cache_status: CacheStatus,
    /// Whether the z3 SMT solver is on PATH.
    pub z3_available: bool,
    /// Every problem above as one flat, remediable list.
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionHealth {
    pub name: String,
    pub version: String,
    /// `builtin` for extensions shipped in the binary, else the lock entry's source.
    pub source: String,
    pub enhancement_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnhancementEntry {
    /// The extension contributing the enhancement.
    pub extension: String,
    pub fields: Vec<String>,
    pub edge_types: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_kinds: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Conflict {
    pub code: String,
    pub severity: FindingStatus,
    pub message: String,
    /// The diagnostic's own suggestion, else the catalogue's explanation of
    /// its code.
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShadowedConstruct {
    pub keyword: String,
    pub code: String,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadFailure {
    pub code: String,
    pub message: String,
    /// The diagnostic's own suggestion, else the catalogue's explanation of
    /// its code.
    pub suggestion: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BinaryIssue {
    MissingBinary {
        name: String,
    },
    StaleHash {
        name: String,
        expected: String,
        actual: String,
    },
    PeerMismatch {
        name: String,
        peer: String,
        /// The range `name` requires.
        required: String,
        /// The version the lock records for `peer`; absent when it isn't
        /// installed.
        #[serde(skip_serializing_if = "Option::is_none")]
        installed: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheStatus {
    Ok,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub check: String,
    pub status: FindingStatus,
    pub code: String,
    pub remediation: String,
}

impl DoctorReport {
    /// Any error-level finding: doctor exits 1.
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.status == FindingStatus::Error)
    }

    /// Every installed binary is healthy and every enabled extension loaded.
    pub fn extensions_ok(&self) -> bool {
        self.issues.is_empty() && self.load_failures.is_empty()
    }

    /// Each conflict's message, in report order.
    pub fn conflict_messages(&self) -> Vec<&str> {
        self.conflicts.iter().map(|c| c.message.as_str()).collect()
    }
}

/// Build the report for the project at `project_root` from a compile's
/// loaded `manifests` and `diagnostics`.
pub fn diagnose(
    project_root: &Path,
    manifests: &[ManifestV2],
    diagnostics: &[Diagnostic],
) -> DoctorReport {
    diagnose_with(project_root, manifests, diagnostics, z3_on_path())
}

/// [`diagnose`] with the z3 probe supplied, so tests do not depend on PATH.
pub fn diagnose_with(
    project_root: &Path,
    manifests: &[ManifestV2],
    diagnostics: &[Diagnostic],
    z3_available: bool,
) -> DoctorReport {
    let lock = read_lock_file(&project_root.join("specforge.lock")).ok();
    let lock_entries = lock.as_ref().map(|l| l.entries.as_slice()).unwrap_or(&[]);

    // Extensions: loaded manifests first (declaration order), then lock
    // entries that did not load. A manifest without a lock entry is a builtin.
    let mut extensions: Vec<ExtensionHealth> = manifests
        .iter()
        .map(|m| ExtensionHealth {
            name: m.name.clone(),
            version: m.version.clone(),
            source: lock_entries
                .iter()
                .find(|e| e.name == m.name)
                .map_or_else(|| "builtin".to_string(), |e| e.source.clone()),
            enhancement_count: m.entity_enhancements.len(),
        })
        .collect();
    for entry in lock_entries {
        if !extensions.iter().any(|e| e.name == entry.name) {
            extensions.push(ExtensionHealth {
                name: entry.name.clone(),
                version: entry.version.clone(),
                source: entry.source.clone(),
                enhancement_count: 0,
            });
        }
    }

    let mut enhancements: BTreeMap<String, Vec<EnhancementEntry>> = BTreeMap::new();
    for manifest in manifests {
        for enhancement in &manifest.entity_enhancements {
            enhancements
                .entry(enhancement.target_kind.clone())
                .or_default()
                .push(EnhancementEntry {
                    extension: manifest.name.clone(),
                    fields: enhancement.fields.iter().map(|f| f.name.clone()).collect(),
                    edge_types: enhancement
                        .edge_types
                        .iter()
                        .map(|e| e.label.clone())
                        .collect(),
                    verify_kinds: enhancement.verify_kinds.clone(),
                });
        }
    }

    let mut findings = Vec::new();

    // Installed binaries against the lock file.
    let installed_versions: HashMap<String, String> = lock_entries
        .iter()
        .map(|e| (e.name.clone(), e.version.clone()))
        .collect();
    let compute_hash = |wasm_path: &Path| -> Option<String> {
        let bytes = std::fs::read(wasm_path).ok()?;
        Some(specforge_wasm::hex_sha256(&bytes))
    };
    let statuses = lock
        .as_ref()
        .map(|l| {
            run_doctor_check(
                l,
                &project_root.join(".specforge").join("extensions"),
                compute_hash,
                &installed_versions,
            )
        })
        .unwrap_or_default();
    // A local install reinstalls from its path; a registry one at the
    // version it is locked at, when that is a version a registry can serve.
    let reinstall = |name: &str| {
        let entry = lock_entries.iter().find(|e| e.name == name);
        let specifier = match entry {
            Some(e) if e.source.starts_with("local:") => e.source["local:".len()..].to_string(),
            Some(e) if semver::Version::parse(&e.version).is_ok() => {
                format!("{name}@{}", e.version)
            }
            _ => name.to_string(),
        };
        format!("run `specforge add {specifier}` to reinstall it")
    };
    let mut issues = Vec::new();
    for status in statuses {
        let (issue, finding) = match status {
            DoctorStatus::Healthy => continue,
            DoctorStatus::MissingBinary { name } => (
                BinaryIssue::MissingBinary { name: name.clone() },
                Finding {
                    check: format!("extension {name}"),
                    status: FindingStatus::Error,
                    code: "missing_binary".into(),
                    remediation: reinstall(&name),
                },
            ),
            DoctorStatus::StaleHash {
                name,
                expected,
                actual,
            } => (
                BinaryIssue::StaleHash {
                    name: name.clone(),
                    expected: expected.clone(),
                    actual: actual.clone(),
                },
                Finding {
                    check: format!("extension {name}: lock expects {expected}, found {actual}"),
                    status: FindingStatus::Error,
                    code: "stale_hash".into(),
                    remediation: reinstall(&name),
                },
            ),
            DoctorStatus::PeerMismatch {
                name,
                peer,
                required,
                installed,
            } => {
                let range_ok = semver::VersionReq::parse(&required).is_ok();
                let version_ok = installed
                    .as_deref()
                    .is_none_or(|v| semver::Version::parse(v).is_ok());
                let (check, remediation) = match &installed {
                    // The requirement itself is broken: reinstall the requirer.
                    _ if !range_ok => (
                        format!(
                            "extension {name}: its peer requirement '{required}' on {peer} \
                             is not a semver range"
                        ),
                        reinstall(&name),
                    ),
                    // The peer's recorded version can't be compared:
                    // reinstalling it records its declared version.
                    Some(version) if !version_ok => (
                        format!(
                            "extension {name}: requires peer {peer} at {required}, but the \
                             lock records {peer} at '{version}', which is not semver"
                        ),
                        reinstall(&peer),
                    ),
                    Some(version) => (
                        format!(
                            "extension {name}: requires peer {peer} at {required}, \
                             installed {version}"
                        ),
                        format!("run `specforge add {peer}@{required}`"),
                    ),
                    None => (
                        format!(
                            "extension {name}: requires peer {peer} at {required}, \
                             not installed"
                        ),
                        format!("run `specforge add {peer}@{required}`"),
                    ),
                };
                (
                    BinaryIssue::PeerMismatch {
                        name: name.clone(),
                        peer: peer.clone(),
                        required: required.clone(),
                        installed: installed.clone(),
                    },
                    Finding {
                        check,
                        status: FindingStatus::Error,
                        code: "peer_mismatch".into(),
                        remediation,
                    },
                )
            }
        };
        issues.push(issue);
        findings.push(finding);
    }
    let cache_status = if issues.iter().any(|i| {
        matches!(
            i,
            BinaryIssue::MissingBinary { .. } | BinaryIssue::StaleHash { .. }
        )
    }) {
        CacheStatus::Stale
    } else {
        CacheStatus::Ok
    };

    // Extensions the compile could not load: `check` fails on them, so
    // doctor does too.
    let mut load_failures = Vec::new();
    for diag in diagnostics {
        if !LOAD_FAILURE_CODES.contains(&diag.code.as_str()) {
            continue;
        }
        let suggestion = remediation(diag, || format!("run `specforge explain {}`", diag.code));
        findings.push(Finding {
            check: diag.message.clone(),
            status: match diag.severity {
                Severity::Error => FindingStatus::Error,
                _ => FindingStatus::Warn,
            },
            code: diag.code.clone(),
            remediation: suggestion.clone(),
        });
        load_failures.push(LoadFailure {
            code: diag.code.clone(),
            message: diag.message.clone(),
            suggestion,
        });
    }

    // Conflicts and shadowed keywords the compile reported.
    let mut conflicts = Vec::new();
    let mut shadowed = Vec::new();
    for diag in diagnostics {
        let conflict = CONFLICT_CODES.contains(&diag.code.as_str());
        let shadowing = SHADOWING_CODES.contains(&diag.code.as_str());
        if !conflict && !shadowing {
            continue;
        }
        let severity = match diag.severity {
            Severity::Error => FindingStatus::Error,
            _ => FindingStatus::Warn,
        };
        let suggestion = remediation(diag, || {
            format!(
                "uninstall or reconfigure one of the conflicting extensions \
                 (`specforge explain {}`)",
                diag.code
            )
        });
        findings.push(Finding {
            check: diag.message.clone(),
            status: severity,
            code: diag.code.clone(),
            remediation: suggestion.clone(),
        });
        if shadowing && let Some(keyword) = shadowed_keyword(&diag.message) {
            shadowed.push(ShadowedConstruct {
                keyword: keyword.to_string(),
                code: diag.code.clone(),
                message: diag.message.clone(),
                suggestion: suggestion.clone(),
            });
        }
        if conflict {
            conflicts.push(Conflict {
                code: diag.code.clone(),
                severity,
                message: diag.message.clone(),
                suggestion,
            });
        }
    }

    if !z3_available {
        findings.push(Finding {
            check: "z3 on PATH".into(),
            status: FindingStatus::Warn,
            code: "z3_missing".into(),
            remediation: "install z3; without it `specforge analyze --prove` skips SMT checks \
                          (W098)"
                .into(),
        });
    }

    DoctorReport {
        extensions,
        enhancements,
        conflicts,
        shadowed,
        load_failures,
        issues,
        extensions_checked: lock_entries.len(),
        cache_status,
        z3_available,
        findings,
    }
}

/// The keyword a shadowing diagnostic names: the `'quoted'` word right after
/// "kind" or "keyword" (E023 messages lead with the extension's name), else
/// the first quoted word.
fn shadowed_keyword(message: &str) -> Option<&str> {
    let parts: Vec<&str> = message.split('\'').collect();
    let quoted = (1..parts.len().saturating_sub(1)).step_by(2);
    quoted
        .clone()
        .find(|&i| {
            let before = parts[i - 1].trim_end();
            before.ends_with("kind") || before.ends_with("keyword")
        })
        .or_else(|| quoted.clone().next())
        .map(|i| parts[i])
}

fn z3_on_path() -> bool {
    std::process::Command::new("z3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// How to fix what `diag` reports: its own suggestion, else the
/// catalogue's explanation of its code, else `fallback` (a code the
/// catalogue doesn't have).
fn remediation(diag: &Diagnostic, fallback: impl FnOnce() -> String) -> String {
    diag.suggestion
        .clone()
        .or_else(|| specforge_diagnostics::lookup(&diag.code).map(|e| e.explanation.to_string()))
        .unwrap_or_else(fallback)
}

// ── registry credentials ────────────────────────────────────────────────────

/// How serious a credential line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialLevel {
    Ok,
    Warning,
    Error,
}

/// One line about one registry's stored credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CredentialLine {
    pub level: CredentialLevel,
    pub text: String,
}

/// The health of the user's registry credentials: expired tokens and
/// unreadable keyring entries break add and publish, so they are worth
/// surfacing before they bite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialHealth {
    /// One line per stored credential worth mentioning, by registry alias.
    pub lines: Vec<CredentialLine>,
    /// How many credentials are unusable (doctor then exits 1).
    pub failures: usize,
    /// Whether a publish signing key exists.
    pub signing_key_present: bool,
}

/// The user's credential health: the credential store and signing key
/// under `~/.specforge`.
pub fn user_credential_health() -> CredentialHealth {
    use specforge_registry_client::credentials::credentials_path;
    credential_health(
        &credentials_path(),
        &specforge_registry_client::signing::signing_key_path(),
        chrono::Utc::now(),
    )
}

/// The credential health of the store at `credentials` and the signing
/// key at `signing_key`, as of `now`.
pub fn credential_health(
    credentials: &Path,
    signing_key: &Path,
    now: chrono::DateTime<chrono::Utc>,
) -> CredentialHealth {
    use specforge_registry_client::credentials::{CredentialEntry, read_credentials};
    let mut lines = Vec::new();
    let mut failures = 0usize;
    let store = read_credentials(credentials).unwrap_or_default();
    let mut aliases: Vec<String> = store.registries.keys().cloned().collect();
    aliases.sort();
    for alias in aliases {
        match store.get_credential_detail(&alias) {
            Err(diag) => {
                failures += 1;
                lines.push(CredentialLine {
                    level: CredentialLevel::Error,
                    text: format!("{} — {}", diag.message, diag.suggestion.unwrap_or_default()),
                });
            }
            Ok(Some(_)) => {
                // Healthy; note an approaching expiry so re-login is not a surprise.
                let Some(CredentialEntry::Token {
                    expires_at: Some(expires_at),
                    ..
                }) = store.registries.get(&alias)
                else {
                    continue;
                };
                let line = match assess_expiry(Some(expires_at), now) {
                    Some(TokenExpiry::ExpiringSoon(days)) => CredentialLine {
                        level: CredentialLevel::Warning,
                        text: format!(
                            "registry '{alias}': token expires in {days} day(s) — re-login soon"
                        ),
                    },
                    // An expired token already failed get_credential_detail.
                    Some(TokenExpiry::Valid) | Some(TokenExpiry::Expired) => CredentialLine {
                        level: CredentialLevel::Ok,
                        text: format!("registry '{alias}': token ok"),
                    },
                    None => CredentialLine {
                        level: CredentialLevel::Ok,
                        text: format!("registry '{alias}': token ok (expires {expires_at})"),
                    },
                };
                lines.push(line);
            }
            Ok(None) => {}
        }
    }
    CredentialHealth {
        lines,
        failures,
        signing_key_present: signing_key.exists(),
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
    expires_at: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<TokenExpiry> {
    let deadline = chrono::DateTime::parse_from_rfc3339(expires_at?)
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
    use specforge_test_macros::test as specforge_test;

    fn diag(code: &str, message: &str, suggestion: Option<&str>) -> Diagnostic {
        Diagnostic {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            span: None,
            suggestion: suggestion.map(String::from),
        }
    }

    #[test]
    fn a_structural_keyword_collision_is_a_shadowed_construct() {
        let dir = tempfile::TempDir::new().unwrap();
        // What manifest_bridge reports for an extension kind named `spec`.
        let diagnostics = [diag(
            "E023",
            "extension 'acme': entity kind 'spec' conflicts with structural keyword",
            Some("choose a different keyword for this entity kind"),
        )];

        let report = diagnose_with(dir.path(), &[], &diagnostics, true);

        assert_eq!(report.shadowed.len(), 1);
        assert_eq!(report.shadowed[0].keyword, "spec");
        assert_eq!(report.shadowed[0].code, "E023");
        assert_eq!(
            report.conflicts[0].suggestion,
            "choose a different keyword for this entity kind"
        );
        assert!(report.has_errors());
    }

    #[test]
    fn a_non_shadowing_conflict_is_listed_but_not_shadowed() {
        let dir = tempfile::TempDir::new().unwrap();
        let diagnostics = [
            diag("W018", "edge type 'uses' declared twice", None),
            diag("W001", "unrelated", None),
        ];

        let report = diagnose_with(dir.path(), &[], &diagnostics, true);

        assert_eq!(report.conflicts.len(), 1);
        assert!(report.shadowed.is_empty());
        // W018 offers no suggestion: the catalogue's explanation stands in.
        assert!(
            report.conflicts[0]
                .suggestion
                .starts_with("Two extensions register an edge type with the same label"),
            "{}",
            report.conflicts[0].suggestion
        );
    }

    #[specforge_test(
        behavior = "run_doctor_check",
        verify = "a finding without its own suggestion quotes the catalogued explanation"
    )]
    fn a_finding_without_a_suggestion_quotes_the_catalogue() {
        let dir = tempfile::TempDir::new().unwrap();
        let diagnostics = [
            // No suggestion: the explanation of E028 is quoted.
            diag("E028", "extension '@acme/x' is not installed", None),
            // Its own suggestion wins.
            diag(
                "E033",
                "binary hash mismatch",
                Some("run `specforge add @acme/y`"),
            ),
        ];

        let report = diagnose_with(dir.path(), &[], &diagnostics, true);

        let remedies: Vec<&str> = report
            .load_failures
            .iter()
            .map(|f| f.suggestion.as_str())
            .collect();
        assert_eq!(remedies.len(), 2, "{remedies:?}");
        assert!(
            remedies[0].contains("confirm the extension is installed and up to date"),
            "E028's catalogued explanation: {}",
            remedies[0]
        );
        assert!(
            !remedies[0].contains("specforge explain"),
            "{}",
            remedies[0]
        );
        assert_eq!(remedies[1], "run `specforge add @acme/y`");
        let finding = report.findings.iter().find(|f| f.code == "E028").unwrap();
        assert_eq!(finding.remediation, remedies[0]);
    }

    #[test]
    fn an_entity_id_that_is_a_kind_keyword_is_shadowed_but_not_a_conflict() {
        let dir = tempfile::TempDir::new().unwrap();
        let diagnostics = [diag(
            "E013",
            "entity ID 'behavior' collides with a reserved keyword at project.spec",
            Some("rename the entity"),
        )];

        let report = diagnose_with(dir.path(), &[], &diagnostics, true);

        assert!(report.conflicts.is_empty());
        assert_eq!(report.shadowed[0].keyword, "behavior");
        assert_eq!(report.shadowed[0].suggestion, "rename the entity");
        assert_eq!(report.findings[0].code, "E013");
    }

    #[test]
    fn a_missing_z3_is_a_warning_not_an_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let report = diagnose_with(dir.path(), &[], &[], false);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code, "z3_missing");
        assert!(!report.has_errors());
        assert_eq!(report.cache_status, CacheStatus::Ok);
    }

    fn at(days_from_now: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now() + chrono::Duration::days(days_from_now)
    }

    #[test]
    fn expiry_buckets() {
        let now = chrono::Utc::now();
        let stamp = |days| at(days).to_rfc3339();
        assert_eq!(
            assess_expiry(Some(&stamp(-2)), now),
            Some(TokenExpiry::Expired)
        );
        // num_days truncates elapsed sub-second precision, so allow ±1 day.
        assert!(matches!(
            assess_expiry(Some(&stamp(3)), now),
            Some(TokenExpiry::ExpiringSoon(2)) | Some(TokenExpiry::ExpiringSoon(3))
        ));
        assert_eq!(
            assess_expiry(Some(&stamp(30)), now),
            Some(TokenExpiry::Valid)
        );
        assert_eq!(assess_expiry(None, now), None);
        assert_eq!(assess_expiry(Some("not-a-date"), now), None);
    }

    #[test]
    fn credential_health_reads_the_store_it_is_given() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = dir.path().join("credentials.json");
        let token =
            |days: i64| serde_json::json!({"token": "t", "expires_at": at(days).to_rfc3339()});
        std::fs::write(
            &store,
            serde_json::json!({"registries": {
                "soon": token(3), "gone": token(-2), "fine": token(30),
            }})
            .to_string(),
        )
        .unwrap();

        let health = credential_health(&store, &dir.path().join("no-key"), chrono::Utc::now());

        assert_eq!(health.failures, 1, "{health:?}");
        assert!(!health.signing_key_present);
        let levels: Vec<CredentialLevel> = health.lines.iter().map(|l| l.level).collect();
        // By alias: fine, gone, soon.
        assert_eq!(
            levels,
            [
                CredentialLevel::Ok,
                CredentialLevel::Error,
                CredentialLevel::Warning
            ]
        );
        assert_eq!(health.lines[0].text, "registry 'fine': token ok");
        assert!(health.lines[2].text.contains("re-login soon"), "{health:?}");

        let empty = credential_health(&dir.path().join("none.json"), &store, chrono::Utc::now());
        assert!(empty.lines.is_empty() && empty.failures == 0);
        assert!(empty.signing_key_present);
    }
}
