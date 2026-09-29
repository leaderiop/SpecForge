//! Test-result collection (ADR 0002).
//!
//! A runner extension declares a *collector*: the files that select it, the
//! command that runs its test runner, where the runner leaves its report,
//! and a pure `collect__<name>` export that maps the report to entity
//! results. The host does everything that touches the machine: it asks for
//! consent, runs the declared command, reads the report and merges the
//! extension's answer into `specforge-report.json`. The extension never
//! runs anything itself.

use crate::analyze::{ReportedEntity, ReportedTest, TestReport};
use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, Severity};
use specforge_registry::ManifestV2;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

/// Where `collect` writes the merged results `analyze` reads.
pub const REPORT_FILE: &str = "specforge-report.json";

/// Environment variable the host sets to the absolute report path when it
/// runs a collector, so a runner integration can write there directly.
pub const REPORT_ENV: &str = "SPECFORGE_REPORT";

/// Overrides the user-level consent store (tests, custom homes).
pub const CONSENT_FILE_ENV: &str = "SPECFORGE_CONSENT_FILE";

/// A collector contributed by an enabled extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collector {
    pub extension: String,
    pub name: String,
    pub export: String,
    pub detect: Vec<String>,
    pub run: Vec<String>,
    /// Report file or directory, relative to the project root.
    pub report: String,
}

/// Every collector the enabled extensions declare, in manifest order.
pub fn collectors(manifests: &[ManifestV2]) -> Vec<Collector> {
    manifests
        .iter()
        .flat_map(|m| {
            m.collector_contributions.iter().map(|c| Collector {
                extension: m.name.clone(),
                name: c.name.clone(),
                export: c.export.clone(),
                detect: c
                    .auto_detect
                    .as_ref()
                    .map(|d| d.file_patterns.clone())
                    .unwrap_or_default(),
                run: c.run.clone(),
                report: c
                    .report
                    .clone()
                    .unwrap_or_else(|| format!(".specforge/reports/{}.json", c.name)),
            })
        })
        .collect()
}

/// Collectors whose detection files are present at the project root.
pub fn detect<'a>(collectors: &'a [Collector], root: &Path) -> Vec<&'a Collector> {
    collectors
        .iter()
        .filter(|c| c.detect.iter().any(|p| pattern_matches(root, p)))
        .collect()
}

/// A detection pattern is a path relative to the project root whose last
/// segment may use `*` wildcards (`vitest.config.*`).
fn pattern_matches(root: &Path, pattern: &str) -> bool {
    let path = Path::new(pattern);
    let Some(file) = path.file_name().and_then(|f| f.to_str()) else {
        return false;
    };
    if !file.contains('*') {
        return root.join(path).exists();
    }
    let dir = root.join(path.parent().unwrap_or(Path::new("")));
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.file_name().to_str().is_some_and(|n| wildcard(file, n)))
}

fn wildcard(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !name.starts_with(first) || !name.ends_with(last) || name.len() < first.len() + last.len() {
        return false;
    }
    let mut rest = &name[first.len()..name.len() - last.len()];
    for part in &parts[1..parts.len() - 1] {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    true
}

/// The absolute report location. Extensions may only name paths inside the
/// project: absolute paths and `..` are refused.
pub fn report_path(collector: &Collector, root: &Path) -> Result<PathBuf, String> {
    let relative = Path::new(&collector.report);
    let inside = relative
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    if !inside {
        return Err(format!(
            "collector '{}' of {} declares report path '{}' outside the project",
            collector.name, collector.extension, collector.report
        ));
    }
    Ok(root.join(relative))
}

/// The declared command with `{report}` expanded.
pub fn command_line(collector: &Collector, report: &Path) -> Vec<String> {
    let report = report.display().to_string();
    collector
        .run
        .iter()
        .map(|arg| arg.replace("{report}", &report))
        .collect()
}

// ── consent ─────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConsentStore {
    #[serde(default)]
    approved: Vec<Consent>,
}

/// One approval: this project may run this extension's collector command.
/// A changed command is a different approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Consent {
    project: String,
    extension: String,
    collector: String,
    run: Vec<String>,
}

/// The user-level consent store. It lives outside the project so a cloned
/// repository can't pre-approve its own commands.
pub fn consent_path() -> PathBuf {
    if let Some(path) = std::env::var_os(CONSENT_FILE_ENV) {
        return PathBuf::from(path);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join(".specforge").join("collector-consent.json")
}

fn consent_for(collector: &Collector, root: &Path) -> Consent {
    let project = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    Consent {
        project: project.display().to_string(),
        extension: collector.extension.clone(),
        collector: collector.name.clone(),
        run: collector.run.clone(),
    }
}

fn load_consent(path: &Path) -> ConsentStore {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Whether the user already approved this collector's command here.
pub fn is_approved(store: &Path, collector: &Collector, root: &Path) -> bool {
    let wanted = consent_for(collector, root);
    load_consent(store).approved.contains(&wanted)
}

/// Record the user's approval of this collector's command here.
pub fn approve(store: &Path, collector: &Collector, root: &Path) -> Result<(), String> {
    let mut consents = load_consent(store);
    let wanted = consent_for(collector, root);
    consents.approved.retain(|c| {
        !(c.project == wanted.project
            && c.extension == wanted.extension
            && c.collector == wanted.collector)
    });
    consents.approved.push(wanted);
    if let Some(dir) = store.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(&consents).expect("consent serialization cannot fail");
    std::fs::write(store, json).map_err(|e| format!("{}: {e}", store.display()))
}

// ── running ─────────────────────────────────────────────────────────────────

/// Where the runner's own output goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerOutput {
    /// The user's terminal.
    Inherit,
    /// Stderr, keeping stdout clean for machine-readable output.
    Stderr,
    /// Discarded (the MCP server owns stdio).
    Discard,
}

/// What running a collector's command produced.
#[derive(Debug, Clone, Copy)]
pub struct Ran {
    /// The runner's exit code; non-zero is normal when tests fail.
    pub exit_code: Option<i32>,
    /// When the command started: a report directory is only read for
    /// files written since, so an old report can never pass for a new run.
    pub started: SystemTime,
}

/// Run the collector's declared command in the project root. A report
/// *file* left by an earlier run is removed first; a report *directory* is
/// left alone (other tools may keep files there) and filtered by
/// modification time when it's read.
pub fn run(
    collector: &Collector,
    root: &Path,
    report: &Path,
    output: RunnerOutput,
) -> Result<Ran, String> {
    let argv = command_line(collector, report);
    let Some((program, args)) = argv.split_first() else {
        return Err(format!(
            "collector '{}' of {} declares no command; use --no-run with its report",
            collector.name, collector.extension
        ));
    };
    if report.is_file() {
        std::fs::remove_file(report).map_err(|e| format!("{}: {e}", report.display()))?;
    }
    if let Some(parent) = report.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let mut cmd = std::process::Command::new(program);
    cmd.args(args)
        .current_dir(root)
        .env(REPORT_ENV, report)
        .stdin(std::process::Stdio::null());
    match output {
        RunnerOutput::Inherit => {}
        RunnerOutput::Stderr => {
            cmd.stdout(std::io::stderr());
        }
        RunnerOutput::Discard => {
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }
    }
    let started = SystemTime::now();
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run `{}`: {e}", argv.join(" ")))?;
    Ok(Ran {
        exit_code: status.code(),
        started,
    })
}

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
}

// ── reading and dispatch ────────────────────────────────────────────────────

/// One report file handed to the collector export.
#[derive(Debug, Clone, Serialize)]
pub struct ReportFile {
    pub path: String,
    pub content: String,
}

/// Read the report at `report`: the file itself, or every `*.json` file
/// directly inside a directory, only those modified at or after `since`
/// when given. Paths are reported relative to `root`.
pub fn read_report(
    report: &Path,
    root: &Path,
    since: Option<SystemTime>,
) -> Result<Vec<ReportFile>, String> {
    let fresh = |path: &PathBuf| {
        since.is_none_or(|since| {
            std::fs::metadata(path)
                .and_then(|m| m.modified())
                .is_ok_and(|modified| modified >= since)
        })
    };
    let paths = if report.is_dir() {
        json_files(report).into_iter().filter(fresh).collect()
    } else if report.is_file() {
        vec![report.to_path_buf()]
    } else {
        Vec::new()
    };
    paths
        .iter()
        .map(|path| {
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("failed to read report {}: {e}", path.display()))?;
            let shown = path.strip_prefix(root).unwrap_or(path);
            Ok(ReportFile {
                path: shown.display().to_string(),
                content,
            })
        })
        .collect()
}

/// A collector export's answer.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CollectedResults {
    #[serde(default)]
    pub entity_results: Vec<EntityResults>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntityResults {
    pub entity_id: String,
    #[serde(default)]
    pub test_results: Vec<CollectedTest>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CollectedTest {
    #[serde(default)]
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub verify: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<f64>,
}

/// Hand the report files to the collector's pure export.
pub fn dispatch(
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    collector: &Collector,
    reports: &[ReportFile],
) -> Result<CollectedResults, String> {
    use specforge_wasm::runtime::WasmCallResult;
    let input = serde_json::to_vec(&serde_json::json!({ "reports": reports }))
        .map_err(|e| format!("cannot serialize reports: {e}"))?;
    match runtime.call_export(&collector.extension, &collector.export, &input) {
        WasmCallResult::Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
            format!(
                "{}: {}() returned malformed results: {e}",
                collector.extension, collector.export
            )
        }),
        WasmCallResult::Trap(trap) => Err(format!(
            "{}: {}() trapped: {}: {}",
            collector.extension, collector.export, trap.kind, trap.message
        )),
    }
}

// ── merging ─────────────────────────────────────────────────────────────────

/// Counts from one merge.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MergeStats {
    pub entities: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// Replace `runner`'s results in `report` with `collected`. Other runners'
/// results are kept, so several runners can report on one project. Tests
/// that name an unknown entity become W115 warnings, and skipped tests are
/// counted but not recorded: a skipped test proves nothing.
pub fn merge(
    report: &mut TestReport,
    runner: &str,
    collected: &CollectedResults,
    known_ids: &HashSet<String>,
) -> (MergeStats, Vec<Diagnostic>) {
    for entity in report.results.values_mut() {
        entity
            .tests
            .retain(|t| t.runner.as_deref().is_some_and(|r| r != runner));
    }
    report.results.retain(|_, e| !e.tests.is_empty());

    let mut stats = MergeStats::default();
    let mut diagnostics = Vec::new();
    let mut unknown = BTreeSet::new();
    for entity in &collected.entity_results {
        if !known_ids.contains(&entity.entity_id) {
            unknown.insert(entity.entity_id.as_str());
            continue;
        }
        let mut recorded = Vec::new();
        for test in &entity.test_results {
            let status = match test.status.as_str() {
                "passed" | "pass" => "pass",
                "failed" | "fail" => "fail",
                _ => {
                    stats.skipped += 1;
                    continue;
                }
            };
            if status == "pass" {
                stats.passed += 1;
            } else {
                stats.failed += 1;
            }
            recorded.push(ReportedTest {
                name: Some(test.name.clone()),
                status: status.to_string(),
                duration_ms: test.duration_ms,
                verify: test.verify.clone(),
                runner: Some(runner.to_string()),
            });
        }
        if recorded.is_empty() {
            continue;
        }
        stats.entities += 1;
        report
            .results
            .entry(entity.entity_id.clone())
            .or_insert_with(|| ReportedEntity {
                file: None,
                tests: Vec::new(),
            })
            .tests
            .extend(recorded);
    }
    for id in unknown {
        diagnostics.push(Diagnostic {
            code: "W115".to_string(),
            severity: Severity::Warning,
            message: format!("{runner} reported tests for unknown entity '{id}'"),
            span: None,
            suggestion: Some("check the test's entity annotation for a rename or typo".to_string()),
        });
    }

    let runners: BTreeSet<&str> = report
        .results
        .values()
        .flat_map(|e| e.tests.iter().filter_map(|t| t.runner.as_deref()))
        .collect();
    report.runner =
        (!runners.is_empty()).then(|| runners.into_iter().collect::<Vec<_>>().join(", "));
    (stats, diagnostics)
}

// ── the whole flow ──────────────────────────────────────────────────────────

/// How to get each collector's report.
#[derive(Debug, Clone, Copy)]
pub enum Mode<'a> {
    /// Run the declared command (after approval), then read its report.
    Run(RunnerOutput),
    /// Read the report already at the declared location.
    NoRun,
    /// Read these files instead (one collector only).
    Reports(&'a [PathBuf]),
}

/// A `collect` request.
pub struct Request<'a> {
    pub root: &'a Path,
    /// Collector name or extension; detected from project files when absent.
    pub runner: Option<&'a str>,
    pub mode: Mode<'a>,
}

/// A failed collect, with the diagnostic code that explains it.
#[derive(Debug)]
pub struct CollectError {
    pub code: &'static str,
    pub message: String,
}

fn fail(code: &'static str, message: impl Into<String>) -> CollectError {
    CollectError {
        code,
        message: message.into(),
    }
}

/// What happened for one collector.
#[derive(Debug, Clone, Serialize)]
pub struct RunnerResult {
    pub name: String,
    pub extension: String,
    pub ran: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub files: usize,
    #[serde(flatten)]
    pub stats: MergeStats,
}

/// The outcome of a successful collect.
#[derive(Debug, Serialize)]
pub struct Outcome {
    pub runners: Vec<RunnerResult>,
    pub diagnostics: Vec<Diagnostic>,
    pub report: PathBuf,
}

/// Collect test results for the project: select collectors, run or read
/// each one's report, map it through the extension and merge the answer
/// into `specforge-report.json`. `approve` decides whether a collector's
/// command may run; `announce` is told just before it runs.
pub fn collect(
    request: &Request,
    manifests: &[ManifestV2],
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    known_ids: &HashSet<String>,
    approve: &mut dyn FnMut(&Collector, &[String]) -> bool,
    announce: &mut dyn FnMut(&Collector, &[String]),
) -> Result<Outcome, CollectError> {
    let root = request.root;
    let available = collectors(manifests);
    let parse_only = !matches!(request.mode, Mode::Run(_));
    let selected = select(&available, request.runner, root)?;
    if let Mode::Reports(_) = request.mode
        && selected.len() > 1
    {
        let names: Vec<&str> = selected.iter().map(|c| c.name.as_str()).collect();
        return Err(fail(
            "E058",
            format!(
                "--report needs one collector, but {} apply here; pick one with --runner",
                names.join(", ")
            ),
        ));
    }

    let mut report = load_report(root);
    let mut runners = Vec::new();
    let mut diagnostics = Vec::new();
    for collector in selected {
        let report_at = report_path(collector, root).map_err(|m| fail("E058", m))?;
        let argv = command_line(collector, &report_at);
        let mut exit_code = None;
        let mut since = None;
        if let Mode::Run(output) = request.mode {
            if !approve(collector, &argv) {
                return Err(fail(
                    "E059",
                    format!(
                        "running `{}` needs your approval: run `specforge collect` in a \
                         terminal, pass --yes, or parse an existing report with --no-run",
                        argv.join(" ")
                    ),
                ));
            }
            announce(collector, &argv);
            let ran = run(collector, root, &report_at, output).map_err(|m| fail("E045", m))?;
            exit_code = ran.exit_code;
            since = Some(ran.started);
        }

        let files = match request.mode {
            Mode::Reports(paths) => paths
                .iter()
                .map(|p| read_report(p, root, None))
                .collect::<Result<Vec<_>, _>>()
                .map(|files| files.into_iter().flatten().collect()),
            _ => read_report(&report_at, root, since),
        }
        .map_err(|m| fail("E045", m))?;
        if files.is_empty() {
            let message = match request.mode {
                Mode::Run(_) => format!(
                    "{} produced no report at {} (did the tests build?)",
                    collector.name,
                    report_at.display()
                ),
                Mode::NoRun => format!(
                    "no {} report at {}; run without --no-run to produce one",
                    collector.name,
                    report_at.display()
                ),
                Mode::Reports(_) => "the --report files don't exist".to_string(),
            };
            return Err(fail("E045", message));
        }

        let collected = dispatch(runtime, collector, &files).map_err(|m| fail("E028", m))?;
        let (stats, diags) = merge(&mut report, &collector.name, &collected, known_ids);
        diagnostics.extend(diags);
        runners.push(RunnerResult {
            name: collector.name.clone(),
            extension: collector.extension.clone(),
            ran: !parse_only,
            exit_code,
            files: files.len(),
            stats,
        });
    }

    let report = save_report(root, &report).map_err(|m| fail("E056", m))?;
    Ok(Outcome {
        runners,
        diagnostics,
        report,
    })
}

/// The collectors to use: the one `runner` names, else every collector
/// whose detection files are present.
fn select<'a>(
    available: &'a [Collector],
    runner: Option<&str>,
    root: &Path,
) -> Result<Vec<&'a Collector>, CollectError> {
    if available.is_empty() {
        return Err(fail(
            "E058",
            "no enabled extension provides a test collector; enable a runner extension \
             (e.g. `specforge add @specforge/cargo-test`)",
        ));
    }
    let names = || {
        available
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if let Some(runner) = runner {
        return available
            .iter()
            .find(|c| c.name == runner || c.extension == runner)
            .map(|c| vec![c])
            .ok_or_else(|| {
                fail(
                    "E058",
                    format!("unknown runner '{runner}' (available: {})", names()),
                )
            });
    }
    let detected = detect(available, root);
    // A project that enabled a single runner chose it: detection only
    // decides between several (vitest configured inside vite.config.ts,
    // for one, has no file of its own to detect).
    if detected.is_empty() && available.len() == 1 {
        return Ok(vec![&available[0]]);
    }
    if detected.is_empty() {
        return Err(fail(
            "E058",
            format!(
                "no test runner detected in {}; pick one with --runner (available: {})",
                root.display(),
                names()
            ),
        ));
    }
    Ok(detected)
}

/// Load `specforge-report.json`, or an empty report when it's missing or
/// unreadable.
pub fn load_report(root: &Path) -> TestReport {
    std::fs::read_to_string(root.join(REPORT_FILE))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(TestReport {
            runner: None,
            results: BTreeMap::new(),
        })
}

/// Write `specforge-report.json`.
pub fn save_report(root: &Path, report: &TestReport) -> Result<PathBuf, String> {
    let path = root.join(REPORT_FILE);
    let json = serde_json::to_string_pretty(report).expect("report serialization cannot fail");
    std::fs::write(&path, json).map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    fn collector(report: &str) -> Collector {
        Collector {
            extension: "@acme/runner".into(),
            name: "acme".into(),
            export: "collect__acme".into(),
            detect: vec!["acme.config.*".into()],
            run: vec!["acme".into(), "--out={report}".into()],
            report: report.into(),
        }
    }

    #[specforge_test(
        behavior = "auto_detect_collector",
        verify = "wildcards match within the last path segment"
    )]
    fn wildcard_matches_segments() {
        assert!(wildcard("vitest.config.*", "vitest.config.ts"));
        assert!(wildcard("*.json", "a.json"));
        assert!(wildcard("a*b*c", "a-b-c"));
        assert!(!wildcard("vitest.config.*", "vite.config.ts"));
        assert!(!wildcard("a*a", "a"));
    }

    #[specforge_test(
        behavior = "auto_detect_collector",
        verify = "file pattern match selects collector"
    )]
    fn detects_by_root_files() {
        let dir = tempfile::tempdir().unwrap();
        let all = vec![collector("r.json")];
        assert!(detect(&all, dir.path()).is_empty());
        std::fs::write(dir.path().join("acme.config.js"), "").unwrap();
        assert_eq!(detect(&all, dir.path()).len(), 1);
    }

    #[specforge_test(
        behavior = "run_collector_command",
        verify = "report path must stay inside the project"
    )]
    fn report_path_must_stay_inside_the_project() {
        let root = Path::new("/p");
        assert_eq!(
            report_path(&collector("out/r.json"), root).unwrap(),
            Path::new("/p/out/r.json")
        );
        assert!(report_path(&collector("../r.json"), root).is_err());
        assert!(report_path(&collector("/etc/r.json"), root).is_err());
    }

    #[specforge_test(
        behavior = "run_collector_command",
        verify = "command line expands the report placeholder"
    )]
    fn command_line_expands_the_report_placeholder() {
        let argv = command_line(&collector("r.json"), Path::new("/p/r.json"));
        assert_eq!(argv, vec!["acme", "--out=/p/r.json"]);
    }

    #[specforge_test(
        behavior = "approve_collector_command",
        verify = "consent is keyed by project, extension and command"
    )]
    fn consent_is_keyed_by_project_extension_and_command() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("consent.json");
        let root = dir.path();
        let mut c = collector("r.json");
        assert!(!is_approved(&store, &c, root));
        approve(&store, &c, root).unwrap();
        assert!(is_approved(&store, &c, root));
        assert!(!is_approved(&store, &c, &root.join("other")));
        c.run.push("--extra".into());
        assert!(
            !is_approved(&store, &c, root),
            "a changed command re-prompts"
        );
        approve(&store, &c, root).unwrap();
        assert_eq!(load_consent(&store).approved.len(), 1, "approval replaced");
    }

    #[specforge_test(
        behavior = "dispatch_collector",
        verify = "reads a file or every json file in a directory"
    )]
    fn reads_a_file_or_every_json_file_in_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("out");
        std::fs::create_dir(&reports).unwrap();
        std::fs::write(reports.join("b.json"), "{}").unwrap();
        std::fs::write(reports.join("a.json"), "[]").unwrap();
        std::fs::write(reports.join("notes.txt"), "x").unwrap();
        let files = read_report(&reports, dir.path(), None).unwrap();
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["out/a.json", "out/b.json"]);
        assert!(
            read_report(&dir.path().join("none.json"), dir.path(), None)
                .unwrap()
                .is_empty()
        );
    }

    #[specforge_test(
        behavior = "run_collector_command",
        verify = "run ignores stale reports and sets the report env"
    )]
    fn run_ignores_stale_reports_and_sets_the_report_env() {
        let dir = tempfile::tempdir().unwrap();
        let report = dir.path().join("r.json");
        std::fs::write(&report, "stale").unwrap();
        let mut c = collector("r.json");
        c.run = vec![
            "sh".into(),
            "-c".into(),
            format!("test ! -e {{report}} && printf fresh > \"${REPORT_ENV}\""),
        ];
        let ran = run(&c, dir.path(), &report, RunnerOutput::Discard).unwrap();
        assert_eq!(ran.exit_code, Some(0));
        assert_eq!(std::fs::read_to_string(&report).unwrap(), "fresh");

        // A report directory keeps its other files; only files the run
        // wrote are read.
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        std::fs::write(out.join("old.json"), "{}").unwrap();
        std::fs::write(out.join("graph.json"), "{}").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        c.run = vec![
            "sh".into(),
            "-c".into(),
            format!("printf '{{}}' > \"${REPORT_ENV}/new.json\""),
        ];
        let ran = run(&c, dir.path(), &out, RunnerOutput::Discard).unwrap();
        assert!(out.join("old.json").exists() && out.join("graph.json").exists());
        let read = read_report(&out, dir.path(), Some(ran.started)).unwrap();
        let paths: Vec<&str> = read.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["out/new.json"]);
    }

    fn collected(entries: &[(&str, &str, &str)]) -> CollectedResults {
        let mut by_entity: BTreeMap<&str, Vec<CollectedTest>> = BTreeMap::new();
        for (id, name, status) in entries {
            by_entity.entry(id).or_default().push(CollectedTest {
                name: name.to_string(),
                status: status.to_string(),
                verify: None,
                duration_ms: None,
            });
        }
        CollectedResults {
            entity_results: by_entity
                .into_iter()
                .map(|(id, tests)| EntityResults {
                    entity_id: id.to_string(),
                    test_results: tests,
                })
                .collect(),
        }
    }

    #[specforge_test(
        behavior = "ingest_collector_report",
        verify = "merge replaces only the same runner"
    )]
    fn merge_replaces_only_the_same_runner() {
        let known: HashSet<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
        let mut report = TestReport {
            runner: None,
            results: BTreeMap::new(),
        };
        merge(
            &mut report,
            "vitest",
            &collected(&[("a", "t1", "passed")]),
            &known,
        );
        let (stats, diags) = merge(
            &mut report,
            "cargo-test",
            &collected(&[
                ("a", "t2", "failed"),
                ("b", "t3", "skipped"),
                ("zz", "t4", "passed"),
            ]),
            &known,
        );
        assert_eq!(
            stats,
            MergeStats {
                entities: 1,
                passed: 0,
                failed: 1,
                skipped: 1
            }
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W115");
        assert_eq!(report.results["a"].tests.len(), 2);
        assert!(!report.results.contains_key("b"));
        assert_eq!(report.runner.as_deref(), Some("cargo-test, vitest"));

        // Re-collecting cargo-test drops its old results, keeps vitest's.
        merge(&mut report, "cargo-test", &collected(&[]), &known);
        assert_eq!(report.results["a"].tests.len(), 1);
        assert_eq!(report.results["a"].tests[0].status, "pass");
        assert_eq!(report.runner.as_deref(), Some("vitest"));
    }

    fn empty_report() -> TestReport {
        TestReport {
            runner: None,
            results: BTreeMap::new(),
        }
    }

    #[specforge_test(
        invariant = "collector_output_conformance",
        verify = "unknown entity ID in collector entry produces W115"
    )]
    fn unknown_entities_are_dropped_with_w115() {
        let known: HashSet<String> = HashSet::from(["a".to_string()]);
        let mut report = empty_report();
        let (_, diags) = merge(
            &mut report,
            "r",
            &collected(&[("zz", "t", "passed")]),
            &known,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W115");
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(report.results.is_empty());
    }

    #[specforge_test(
        invariant = "collector_output_conformance",
        verify = "skipped tests are not recorded as proof"
    )]
    fn skipped_tests_are_not_recorded() {
        let known: HashSet<String> = HashSet::from(["a".to_string()]);
        let mut report = empty_report();
        let (stats, _) = merge(
            &mut report,
            "r",
            &collected(&[("a", "t", "skipped")]),
            &known,
        );
        assert_eq!(stats.skipped, 1);
        assert!(!report.results.contains_key("a"));
    }
}
