//! `specforge doctor`: one project health report, shared by the CLI command
//! and the MCP `specforge.doctor` tool so both surfaces say the same thing.
//!
//! The report is built from what a compile already produced (the loaded
//! manifests and the diagnostics) plus the project's lock file on disk:
//! enabled extensions with their enhancement counts, enhancements grouped
//! by the entity kind they target, extension conflicts with a resolution
//! suggestion, keywords that shadow an entity kind, installed-binary
//! integrity, and whether the z3 solver is on PATH.

use serde::Serialize;
use specforge_common::{Diagnostic, Severity};
use specforge_registry::ManifestV2;
use specforge_wasm::{DoctorStatus, read_lock_file, run_doctor_check};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Diagnostic codes that mean two contributions collide.
pub const CONFLICT_CODES: [&str; 7] = ["E017", "E018", "E023", "E026", "E029", "E057", "W018"];

/// Codes that mean a name shadows a grammar-level construct: E013 (a project
/// entity ID is a structural keyword or an extension's kind keyword), E023
/// (an extension's entity kind collides with a structural keyword) and E026
/// (a kind keyword is registered twice). E023 and E026 are also conflicts.
pub const SHADOWING_CODES: [&str; 3] = ["E013", "E023", "E026"];

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
    /// The diagnostic's own suggestion, else a pointer to `specforge explain`.
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShadowedConstruct {
    pub keyword: String,
    pub code: String,
    pub message: String,
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
        required: String,
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
    let reinstall = |name: &str| match installed_versions.get(name) {
        Some(version) => format!("run `specforge add {name}@{version}` to reinstall it"),
        None => format!("run `specforge add {name}` to reinstall it"),
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
            } => (
                BinaryIssue::PeerMismatch {
                    name: name.clone(),
                    peer: peer.clone(),
                    required: required.clone(),
                },
                Finding {
                    check: format!("extension {name}: requires peer {peer} at {required}"),
                    status: FindingStatus::Error,
                    code: "peer_mismatch".into(),
                    remediation: format!("run `specforge add {peer}@{required}`"),
                },
            ),
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
        let suggestion = diag.suggestion.clone().unwrap_or_else(|| {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            report.conflicts[0]
                .suggestion
                .contains("specforge explain W018")
        );
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
}
