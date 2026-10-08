//! `specforge doctor`: one project health report, shared by the CLI command
//! and the MCP `specforge.doctor` tool so both surfaces say the same thing.
//!
//! The report is built from what a compile already produced (the loaded
//! declarations and the diagnostics) plus the project's lock file on disk:
//! enabled extensions with their enhancement counts, enhancements grouped
//! by the entity kind they target, extension conflicts with a resolution
//! suggestion, keywords that shadow an entity kind, extensions that failed
//! to load, installed-binary integrity, and whether the z3 solver is on PATH.
//!
//! Registry credential health is the user's, not the project's, and lives
//! with the credential store (`specforge_registry_client::credential_health`):
//! `specforge doctor` reports it; the MCP tool, whose spec'd report
//! (`McpDoctorReport`) is about the project, does not.

use serde::Serialize;
use specforge_common::{Code, Diagnostic, DiagnosticData, Severity, codes};
use specforge_installed::Health;
use std::collections::BTreeMap;

use crate::extension::Origin;
use crate::view::ProjectView;

/// Diagnostic codes that mean two contributions collide.
pub const CONFLICT_CODES: [Code; 3] = [codes::E026, codes::E057, codes::W018];

/// Codes that mean a name shadows a grammar-level construct: E013 (a project
/// entity ID is a structural keyword or an extension's kind keyword) and E026
/// (a kind keyword is registered twice, which is also a conflict).
pub const SHADOWING_CODES: [Code; 2] = [codes::E013, codes::E026];

/// Codes that mean `specforge.json` is not used as written: E069 (it can't
/// be read, isn't a JSON object, or has a mistyped key or item).
pub const CONFIG_CODES: [Code; 1] = [codes::E069];

/// Codes the compile reports a peer requirement by (ADR 0041): E027 (unsatisfied, or a cycle among
/// required peers) and E073 (a range that is not SemVer).
pub const PEER_CODES: [Code; 2] = [codes::E027, codes::E073];

/// The finding code of a project root without `specforge.json`.
pub const CONFIG_MISSING: &str = "config_missing";

/// The finding code of a `specforge.lock` that exists but cannot be read
/// (its check names the diagnostic, E033).
pub const LOCK_UNREADABLE: &str = "lock_unreadable";

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
    /// The peer requirements the compile reports unsatisfied (see [`PEER_CODES`]), in diagnostic
    /// order (ADR 0041).
    pub peers: Vec<PeerProblem>,
    /// Enabled extensions the compile could not load, in entry order: E028
    /// (not installed, not loadable, or its lock unreadable) and E070 (its
    /// installed binary is not the one the lock pins).
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
    /// Where it comes from, as the extensions listing names it: `builtin`,
    /// the lock entry's source, `file:<path>` for a `.wasm` file entry, or
    /// `unknown`.
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

/// A peer requirement the compile reports unsatisfied.
#[derive(Debug, Clone, Serialize)]
pub struct PeerProblem {
    pub code: String,
    pub message: String,
    /// The diagnostic's own suggestion.
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadFailure {
    pub code: String,
    pub message: String,
    /// The diagnostic's own suggestion, else the catalogue's explanation of
    /// its code.
    pub suggestion: String,
    /// The failure is a missing or changed installed binary, which
    /// [`DoctorReport::issues`] lists too: the findings list it once.
    pub binary_issue: bool,
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
        self.issues.is_empty() && self.load_failures.is_empty() && self.peers.is_empty()
    }

    /// Each conflict's message, in report order.
    pub fn conflict_messages(&self) -> Vec<&str> {
        self.conflicts.iter().map(|c| c.message.as_str()).collect()
    }
}

/// The health report of the project the view was compiled from: its
/// loaded declarations, the diagnostics its surface reports, and, with a
/// root, its lock and installed binaries. Without a root the installation
/// checks are skipped (`extensions_checked: 0`).
pub fn diagnose(view: &ProjectView) -> DoctorReport {
    diagnose_with(view, z3_on_path())
}

/// [`diagnose`] with the z3 probe supplied, so tests do not depend on PATH.
pub fn diagnose_with(view: &ProjectView, z3_available: bool) -> DoctorReport {
    let declarations = view.registries().declarations();
    let diagnostics = view.reported();
    let lock = view.lock().file();
    let lock_entries = view.lock().entries();

    // Extensions: loaded declarations first (load order), then lock entries
    // that did not load. Each is named by the listing's source rule: the
    // .wasm file an entry names, the lock entry's source, a builtin, else
    // unknown.
    let mut extensions: Vec<ExtensionHealth> = declarations
        .iter()
        .map(|d| ExtensionHealth {
            name: d.name().to_string(),
            version: d.version().to_string(),
            source: Origin::of(d.name(), &view.env().enabled, lock).source(),
            enhancement_count: d.enhancements.len(),
        })
        .collect();
    for entry in lock_entries {
        if !extensions.iter().any(|e| e.name == entry.name.as_str()) {
            extensions.push(ExtensionHealth {
                name: entry.name.to_string(),
                version: entry.version.clone(),
                source: entry.source.to_string(),
                enhancement_count: 0,
            });
        }
    }

    let mut enhancements: BTreeMap<String, Vec<EnhancementEntry>> = BTreeMap::new();
    for declaration in declarations {
        for enhancement in &declaration.enhancements {
            enhancements
                .entry(enhancement.target_kind.clone())
                .or_default()
                .push(EnhancementEntry {
                    extension: declaration.name().to_string(),
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

    // The config itself: a `specforge.json` the compile could not use as
    // written (E069, an error: `check` fails on it, so doctor does too),
    // or none at the project root (a warning: the default config is a
    // valid project, but rarely the one meant).
    for diag in diagnostics
        .iter()
        .filter(|d| CONFIG_CODES.iter().any(|code| d.is(*code)))
    {
        findings.push(Finding {
            check: diag.message.clone(),
            status: match diag.severity {
                Severity::Error => FindingStatus::Error,
                _ => FindingStatus::Warn,
            },
            code: diag.code.clone(),
            remediation: remediation(diag, || format!("run `specforge explain {}`", diag.code)),
        });
    }
    if let Some(root) = view.root()
        && !view.env().config_found
    {
        findings.push(Finding {
            check: format!("specforge.json at {}", root.display()),
            status: FindingStatus::Warn,
            code: CONFIG_MISSING.into(),
            remediation: "run `specforge init` here, or pass --path to the project root; without \
                          specforge.json the project has the default config and no extension"
                .into(),
        });
    }

    // A lock file that cannot be used: nothing is known to be installed.
    if let Some(problem) = view.lock().problem() {
        findings.push(Finding {
            check: format!("{} [{}]", problem.message, problem.code),
            status: FindingStatus::Error,
            code: LOCK_UNREADABLE.into(),
            remediation: remediation(problem, || {
                format!("run `specforge explain {}`", problem.code)
            }),
        });
    }

    // Installed binaries against the lock file.
    let installed = view.installed();
    let reinstall = |name: &str| format!("run `{}` to reinstall it", installed.reinstall(name));
    let mut issues = Vec::new();
    for status in installed.health() {
        let (issue, finding) = match status {
            Health::MissingModule { name } => (
                BinaryIssue::MissingBinary { name: name.clone() },
                Finding {
                    check: format!("extension {name}"),
                    status: FindingStatus::Error,
                    code: "missing_binary".into(),
                    remediation: reinstall(&name),
                },
            ),
            Health::Changed {
                name,
                locked: expected,
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
        };
        issues.push(issue);
        findings.push(finding);
    }
    let cache_status = if issues.is_empty() {
        CacheStatus::Ok
    } else {
        CacheStatus::Stale
    };

    // Extensions the compile could not load: `check` fails on them, so
    // doctor does too. A missing or changed binary is a finding of its own
    // above, with the command that reinstalls it: not listed twice.
    let mut load_failures = Vec::new();
    for enabled in &view.env().enabled {
        let Some(failure) = &enabled.failure else {
            continue;
        };
        let diag = &failure.diagnostic;
        let binary_issue = failure.problem.is_module_health();
        let suggestion = remediation(diag, || format!("run `specforge explain {}`", diag.code));
        if !binary_issue {
            findings.push(Finding {
                check: diag.message.clone(),
                status: match diag.severity {
                    Severity::Error => FindingStatus::Error,
                    _ => FindingStatus::Warn,
                },
                code: diag.code.clone(),
                remediation: suggestion.clone(),
            });
        }
        load_failures.push(LoadFailure {
            code: diag.code.clone(),
            message: diag.message.clone(),
            suggestion,
            binary_issue,
        });
    }

    // Conflicts and shadowed keywords the compile reported.
    let mut conflicts = Vec::new();
    let mut shadowed = Vec::new();
    let mut peers = Vec::new();
    for diag in &diagnostics {
        let conflict = CONFLICT_CODES.iter().any(|code| diag.is(*code));
        let shadowing = SHADOWING_CODES.iter().any(|code| diag.is(*code));
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
        // The keyword is the diagnostic's data, not a quoted word of its
        // message.
        if shadowing && let Some(DiagnosticData::ShadowedKeyword { keyword }) = diag.data.as_deref()
        {
            shadowed.push(ShadowedConstruct {
                keyword: keyword.clone(),
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

    // The peer requirements the compile reports unsatisfied: one rule, so doctor and
    // check cannot disagree (ADR 0041).
    for diag in diagnostics
        .iter()
        .filter(|d| PEER_CODES.iter().any(|code| d.is(*code)))
    {
        let suggestion = remediation(diag, || format!("run `specforge explain {}`", diag.code));
        findings.push(Finding {
            check: diag.message.clone(),
            status: FindingStatus::Error,
            code: diag.code.clone(),
            remediation: suggestion.clone(),
        });
        peers.push(PeerProblem {
            code: diag.code.clone(),
            message: diag.message.clone(),
            suggestion,
        });
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
        peers,
        load_failures,
        issues,
        extensions_checked: lock_entries.len(),
        cache_status,
        z3_available,
        findings,
    }
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
        .or_else(|| {
            specforge_diagnostics::describes(&diag.code, diag.origin())
                .map(|e| e.explanation.to_string())
        })
        .unwrap_or_else(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::testing::Fixture;
    use specforge_test_macros::test as specforge_test;

    fn diag(code: &str, message: &str, suggestion: Option<&str>) -> Diagnostic {
        let mut diagnostic = Diagnostic::untyped(code, Severity::Error, message);
        diagnostic.suggestion = suggestion.map(String::from);
        diagnostic
    }

    #[test]
    fn a_kind_registered_twice_is_a_shadowed_construct() {
        // What the registry build reports for a kind two extensions declare,
        // worded so that no quoted word of it is the keyword: only the data
        // names it.
        let mut e026 = diag(
            "E026",
            "extension 'acme': its entity kind is registered by another extension",
            Some("choose a different keyword for this entity kind"),
        );
        e026.data = Some(Box::new(DiagnosticData::ShadowedKeyword {
            keyword: "memo".into(),
        }));
        let diagnostics = [e026];

        let fixture = Fixture::new().reporting(diagnostics.to_vec());
        let report = diagnose_with(&fixture.view(), true);

        assert_eq!(report.shadowed.len(), 1);
        assert_eq!(report.shadowed[0].keyword, "memo");
        assert_eq!(report.shadowed[0].code, "E026");
        assert_eq!(
            report.conflicts[0].suggestion,
            "choose a different keyword for this entity kind"
        );
        assert!(report.has_errors());
    }

    #[test]
    fn a_non_shadowing_conflict_is_listed_but_not_shadowed() {
        let diagnostics = [
            diag("W018", "edge type 'uses' declared twice", None),
            diag("W001", "unrelated", None),
        ];

        let fixture = Fixture::new().reporting(diagnostics.to_vec());
        let report = diagnose_with(&fixture.view(), true);

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

    /// An entry the load could not load, as the environment records it.
    fn failed(
        entry: &str,
        problem: specforge_installed::LoadProblem,
        diagnostic: Diagnostic,
    ) -> specforge_project::EnabledExtension {
        specforge_project::EnabledExtension {
            failure: Some(specforge_installed::LoadFailure {
                problem,
                diagnostic,
            }),
            ..specforge_project::EnabledExtension::unloaded(entry)
        }
    }

    #[specforge_test(
        behavior = "run_doctor_check",
        verify = "a finding without its own suggestion quotes the catalogued explanation"
    )]
    fn a_finding_without_a_suggestion_quotes_the_catalogue() {
        use specforge_installed::LoadProblem;
        let fixture = Fixture::new().enabled(vec![
            // No suggestion: the explanation of E028 is quoted.
            failed(
                "@acme/x",
                LoadProblem::NotInstalled,
                diag("E028", "extension '@acme/x' is not installed", None),
            ),
            // Its own suggestion wins.
            failed(
                "@acme/y",
                LoadProblem::NoDeclaration {
                    reason: "no handshake".into(),
                },
                diag(
                    "E028",
                    "protocol loading failed",
                    Some("run `specforge add @acme/y`"),
                ),
            ),
        ]);

        let report = diagnose_with(&fixture.view(), true);

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
        let mut e013 = diag(
            "E013",
            "entity ID 'behavior' collides with a reserved keyword at project.spec",
            Some("rename the entity"),
        );
        e013.data = Some(Box::new(DiagnosticData::ShadowedKeyword {
            keyword: "behavior".into(),
        }));
        let diagnostics = [e013];

        let fixture = Fixture::new().reporting(diagnostics.to_vec());
        let report = diagnose_with(&fixture.view(), true);

        assert!(report.conflicts.is_empty());
        assert_eq!(report.shadowed[0].keyword, "behavior");
        assert_eq!(report.shadowed[0].suggestion, "rename the entity");
        assert_eq!(report.findings[0].code, "E013");
    }

    #[test]
    fn a_missing_z3_is_a_warning_not_an_error() {
        let fixture = Fixture::new();
        let report = diagnose_with(&fixture.view(), false);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code, "z3_missing");
        assert!(!report.has_errors());
        assert_eq!(report.cache_status, CacheStatus::Ok);
    }

    #[specforge_test(
        behavior = "management_operations_over_the_project_view",
        verify = "doctor reads the diagnostics its view reports"
    )]
    fn doctor_reads_the_diagnostics_its_view_reports() {
        let e028 = diag("E028", "extension '@acme/x' is not installed", None);
        let failing = Fixture::new().enabled(vec![failed(
            "@acme/x",
            specforge_installed::LoadProblem::NotInstalled,
            e028,
        )]);
        let report = diagnose_with(&failing.view(), true);
        let failures: Vec<&str> = report
            .load_failures
            .iter()
            .map(|f| f.code.as_str())
            .collect();
        assert_eq!(failures, ["E028"]);
        assert!(report.has_errors());

        let clean = Fixture::new();
        let report = diagnose_with(&clean.view(), true);
        assert!(report.load_failures.is_empty());
        assert!(!report.has_errors());
    }

    #[test]
    fn a_rootless_view_skips_the_installation_checks() {
        let fixture = Fixture::new()
            .declarations(vec![Fixture::declaration("@acme/loaded", "2.0.0")])
            .lock(&[("@acme/locked", "1.0.0", "registry")]);

        let report = diagnose_with(&fixture.rootless_view(), true);

        assert_eq!(report.extensions_checked, 0);
        assert_eq!(report.cache_status, CacheStatus::Ok);
        let names: Vec<&str> = report.extensions.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["@acme/loaded"], "no lock is read without a root");

        // Rooted, the lock is read and its binary checked (missing here).
        let report = diagnose_with(&fixture.view(), true);
        assert_eq!(report.extensions_checked, 1);
        assert_eq!(report.cache_status, CacheStatus::Stale);
    }

    #[specforge_test(
        behavior = "run_doctor_check",
        verify = "doctor gives each extension the source the extensions listing gives it"
    )]
    fn doctor_names_each_extension_by_the_listings_source() {
        let fixture = Fixture::new()
            .config(&["@specforge/product", "greet.wasm", "@acme/stray"])
            .enabled(vec![
                specforge_project::EnabledExtension::unloaded("@specforge/product"),
                specforge_project::EnabledExtension {
                    entry: "greet.wasm".into(),
                    name: "@sdk/greet".into(),
                    file: Some("greet.wasm".into()),
                    failure: None,
                },
            ])
            .declarations(vec![
                Fixture::declaration("@specforge/product", "1.0.0"),
                Fixture::declaration("@sdk/greet", "0.1.0"),
                Fixture::declaration("@acme/stray", "3.0.0"),
            ]);
        let view = fixture.view();

        let report = diagnose_with(&view, true);

        let sources: Vec<(&str, &str)> = report
            .extensions
            .iter()
            .map(|e| (e.name.as_str(), e.source.as_str()))
            .collect();
        assert_eq!(
            sources,
            [
                ("@specforge/product", "builtin"),
                ("@sdk/greet", "file:greet.wasm"),
                ("@acme/stray", "unknown"),
            ]
        );
        for extension in &report.extensions {
            let listed = crate::extension::list(&view)
                .extensions
                .into_iter()
                .find(|e| e.name == extension.name)
                .unwrap();
            assert_eq!(
                listed.origin.source(),
                extension.source,
                "{}",
                extension.name
            );
        }
    }

    fn finding_codes(report: &DoctorReport) -> Vec<(&str, FindingStatus)> {
        report
            .findings
            .iter()
            .map(|f| (f.code.as_str(), f.status))
            .collect()
    }

    #[specforge_test(
        behavior = "run_doctor_check",
        verify = "doctor in a directory without specforge.json reports config_missing as a warning"
    )]
    fn doctor_without_specforge_json_says_so() {
        let fixture = Fixture::new().without_config_file();

        let report = diagnose_with(&fixture.view(), true);

        assert_eq!(
            finding_codes(&report),
            [(CONFIG_MISSING, FindingStatus::Warn)]
        );
        assert!(!report.has_errors(), "a warning: doctor stays healthy");
        assert!(
            report.findings[0]
                .check
                .contains(&fixture.dir.path().display().to_string()),
            "{:?}",
            report.findings[0]
        );
        assert!(report.findings[0].remediation.contains("specforge init"));

        // A project with specforge.json gets no such finding.
        let found = Fixture::new();
        assert!(finding_codes(&diagnose_with(&found.view(), true)).is_empty());
    }

    #[specforge_test(
        behavior = "run_doctor_check",
        verify = "doctor in a directory without specforge.json reports config_missing as a warning"
    )]
    fn a_rootless_view_has_no_config_missing_finding() {
        let fixture = Fixture::new().without_config_file();

        let report = diagnose_with(&fixture.rootless_view(), true);

        assert!(finding_codes(&report).is_empty(), "{:?}", report.findings);
    }

    /// A project with `@sdk/greet` installed from a file in a second
    /// directory (returned with it: the lock names its path), whose module
    /// was then replaced by other bytes.
    fn project_with_a_changed_greet() -> (tempfile::TempDir, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("specforge.json"),
            r#"{"name": "p", "version": "0.1.0", "extensions": []}"#,
        )
        .unwrap();
        let files = tempfile::tempdir().unwrap();
        let blob = files.path().join("greet.wasm");
        std::fs::write(&blob, crate::testing::GREET).unwrap();
        crate::extension::add(
            &crate::extension::AddRequest {
                root: dir.path(),
                source: crate::extension::Source::Local(blob),
                allow_unsigned: false,
                trust: crate::extension::Trust::Refuse,
                dry_run: false,
            },
            &crate::registry::Unconfigured("add"),
            &crate::testing::candidates(),
        )
        .unwrap();
        let module = dir
            .path()
            .join(".specforge/extensions/@sdk/greet/extension.wasm");
        let mut bytes = std::fs::read(&module).unwrap();
        bytes.extend_from_slice(b"changed after install");
        std::fs::write(module, bytes).unwrap();
        (dir, files)
    }

    #[specforge_test(
        behavior = "run_doctor_check",
        verify = "doctor reports a missing or changed installed binary once, with the remedy its load gives"
    )]
    fn doctor_reports_a_changed_binary_once() {
        let (dir, _files) = project_with_a_changed_greet();
        let runtime = std::sync::Arc::new(crate::testing::candidates());
        let compiled =
            specforge_project::CompiledProject::compile(dir.path(), Some(runtime.clone()));

        let report = diagnose_with(&ProjectView::of(&compiled), true);

        let about_greet: Vec<(&str, &str)> = report
            .findings
            .iter()
            .filter(|f| f.check.contains("@sdk/greet"))
            .map(|f| (f.code.as_str(), f.remediation.as_str()))
            .collect();
        assert_eq!(about_greet.len(), 1, "{:?}", report.findings);
        let (code, remedy) = about_greet[0];
        assert_eq!(code, "stale_hash");
        assert!(
            remedy.starts_with("run `specforge add ") && remedy.ends_with(".wasm` to reinstall it"),
            "{remedy}"
        );
        // Both facets stay in the report's data: the issue and the load
        // failure that duplicates it.
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.load_failures.len(), 1);
        assert_eq!(report.load_failures[0].code, "E070");
        assert!(report.load_failures[0].binary_issue);

        // Running the remedy reinstalls the pinned binary: doctor is clean.
        let command = remedy
            .strip_prefix("run `specforge add ")
            .and_then(|r| r.strip_suffix("` to reinstall it"))
            .unwrap();
        crate::extension::add(
            &crate::extension::AddRequest {
                root: dir.path(),
                source: crate::extension::Source::Local(command.into()),
                allow_unsigned: false,
                trust: crate::extension::Trust::Refuse,
                dry_run: false,
            },
            &crate::registry::Unconfigured("add"),
            runtime.as_ref(),
        )
        .unwrap();
        let compiled = specforge_project::CompiledProject::compile(dir.path(), Some(runtime));
        let report = diagnose_with(&ProjectView::of(&compiled), true);
        assert!(report.issues.is_empty(), "{:?}", report.issues);
        assert!(
            report.load_failures.is_empty(),
            "{:?}",
            report.load_failures
        );
    }

    #[specforge_test(
        behavior = "provide_mcp_doctor_tool",
        verify = "specforge.doctor reports an unusable specforge.json (E069) as a finding"
    )]
    fn an_unusable_config_is_an_error_finding() {
        let e069 = diag(
            "E069",
            "specforge.json can't be used: ./specforge.json is not valid JSON: expected value at line 1 column 41; no extension is loaded",
            Some("fix specforge.json; `specforge explain E069` says what it must be"),
        );
        let fixture = Fixture::new().reporting(vec![e069]);

        let report = diagnose_with(&fixture.view(), true);

        assert_eq!(finding_codes(&report), [("E069", FindingStatus::Error)]);
        assert!(report.has_errors());
        assert_eq!(
            report.findings[0].remediation,
            "fix specforge.json; `specforge explain E069` says what it must be"
        );
        assert!(
            report.load_failures.is_empty(),
            "E069 is about the config, not an extension"
        );
    }
}
