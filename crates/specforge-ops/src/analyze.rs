//! `specforge analyze`: run the analysis passes over a [`ProjectView`].
//!
//! One operation serves the CLI and the MCP tool: it validates the pass
//! selection, reads the test report, runs the built-in passes, the
//! extension passes (through the [`WasmRuntime`] port) and, when asked,
//! `prove`, then applies strictness once and computes `ok` once. The shape of
//! the JSON document lives in [`AnalyzeOutcome::to_json`] and nowhere else.
//!
//! Surfaces keep rendering, exit-code or error-channel mapping, and the
//! choice of which project to analyse.

use std::path::PathBuf;
use std::sync::Arc;

use crate::builtin_passes::{COVERAGE_PASS, PASS_NAMES};
use specforge_common::{Diagnostic, Severity};
use specforge_graph::Graph;
use specforge_project::coverage;
use specforge_project::coverage::TestReport;
use specforge_project::passes::{self, AnalysisContext};
use specforge_registry::DeclaredPass;
use specforge_wasm::runtime::WasmRuntime;

use crate::{OpError, OpErrorKind};

pub use crate::view::ProjectView;

/// Where the test report comes from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ReportSource {
    /// The view's recorded report: what `specforge collect` last wrote at
    /// the root the project was compiled from, if anything (never an
    /// ancestor's: a sub-path does not inherit its parent's report).
    #[default]
    Recorded,
    /// A named report file, which must exist.
    File(PathBuf),
    /// No report.
    None,
}

pub use crate::prove::ProveOptions;

/// The pass name that runs every pass: what an absent pass means on every
/// surface (`specforge analyze`, `specforge.analyze`).
pub const EVERY_PASS: &str = "all";

/// What to run. `Default` is a plain `analyze all`.
#[derive(Debug, Clone)]
pub struct AnalyzeOptions {
    /// `all`, `coverage`, a built-in pass, or a declared `<extension>:<pass>`.
    pub pass: String,
    pub strict: bool,
    pub report: ReportSource,
    /// The proof-coverage threshold the caller will gate on; it needs a report.
    pub min: Option<f64>,
    pub prove: Option<ProveOptions>,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            pass: EVERY_PASS.to_string(),
            strict: false,
            report: ReportSource::default(),
            min: None,
            prove: None,
        }
    }
}

/// The result of one pass (built-in, extension-owned or `prove`).
#[derive(Debug, Clone)]
pub struct PassOutcome {
    pub name: String,
    pub description: String,
    pub findings: Vec<Diagnostic>,
    pub summary: serde_json::Value,
}

/// W097: a test record naming an entity the graph does not know, with the
/// closest known id when one is near. Not a finding of any pass, so strict
/// never promotes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    pub entity_id: String,
    pub near: Option<String>,
}

/// What an analysis found. Strictness is already applied.
#[derive(Debug, Clone)]
pub struct AnalyzeOutcome {
    /// No finding is an error, after strict promotion.
    pub ok: bool,
    /// Built-in passes, extension passes in their declared order, `prove` last.
    pub passes: Vec<PassOutcome>,
    /// The `min` proof-coverage gate. It never changes `ok` or `passes`; the
    /// caller decides what a failed gate costs (CLI: exit code).
    pub gate: Gate,
    /// Test records for unknown entities, outside the pass reports.
    pub orphans: Vec<Orphan>,
}

/// Where the proof-coverage gate landed.
#[derive(Debug, Clone, PartialEq)]
pub enum Gate {
    /// `min` was not set.
    NotRequested,
    /// Proof coverage is at or above `min` (nothing testable always is).
    Met,
    /// Proof coverage is under `min`.
    Below {
        pct: f64,
        min: f64,
        proven: usize,
        total: usize,
    },
    /// `min` was set but the coverage pass did not run.
    NoCoveragePass,
    /// The coverage pass ran but its summary is not the shape read here;
    /// carries the parse error.
    UnreadableSummary(String),
}

impl Gate {
    fn of(min: Option<f64>, passes: &[PassOutcome]) -> Gate {
        let Some(min) = min else {
            return Gate::NotRequested;
        };
        let Some(pass) = passes.iter().find(|r| r.name == COVERAGE_PASS) else {
            return Gate::NoCoveragePass;
        };
        // A missing or renamed key is an error, never a silent 0.
        let summary: coverage::Summary = match serde_json::from_value(pass.summary.clone()) {
            Ok(summary) => summary,
            Err(e) => return Gate::UnreadableSummary(e.to_string()),
        };
        let pct = summary.proof_pct();
        if pct + f64::EPSILON < min {
            Gate::Below {
                pct,
                min,
                proven: summary.testable_proven,
                total: summary.testable_total,
            }
        } else {
            Gate::Met
        }
    }
}

impl AnalyzeOutcome {
    /// The JSON document `{ok, passes: [{pass, findings, summary}]}`, plus
    /// `orphans` when there are any.
    pub fn to_json(&self) -> serde_json::Value {
        let mut doc = serde_json::json!({
            "ok": self.ok,
            "passes": self
                .passes
                .iter()
                .map(|r| serde_json::json!({
                    "pass": r.name,
                    "findings": r.findings,
                    "summary": r.summary,
                }))
                .collect::<Vec<_>>(),
        });
        if !self.orphans.is_empty() {
            doc["orphans"] = self
                .orphans
                .iter()
                .map(|o| serde_json::json!({"entity_id": o.entity_id, "near": o.near}))
                .collect();
        }
        doc
    }
}

/// Why no analysis ran. Raised before any pass runs, in this order.
#[derive(Debug, Clone, PartialEq)]
pub enum AnalyzeError {
    UnknownPass {
        requested: String,
        available: Vec<String>,
    },
    /// The test report is there (or was named) but cannot be used.
    UnusableReport(OpError),
    /// `min` was set and there is no report to score.
    MinNeedsTestResults,
}

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyzeError::UnknownPass {
                requested,
                available,
            } => write!(
                f,
                "Unknown analysis pass '{requested}' (available: {})",
                available.join(", ")
            ),
            AnalyzeError::UnusableReport(e) => e.fmt(f),
            AnalyzeError::MinNeedsTestResults => f.write_str(
                "--min needs test results: run `specforge collect` or pass --test-results",
            ),
        }
    }
}

impl std::error::Error for AnalyzeError {}

/// What each way of not running is, for every surface: an unknown pass is
/// `invalid_input` (`unknown_pass`, with a did-you-mean when one is close),
/// an unusable report is the classified E045, `--min` without a report is
/// `precondition_failed` (`no_test_results`).
impl From<AnalyzeError> for OpError {
    fn from(error: AnalyzeError) -> Self {
        let message = error.to_string();
        match error {
            AnalyzeError::UnknownPass {
                requested,
                available,
            } => {
                let close = specforge_common::suggest::find_close_match(
                    &requested,
                    available.iter().map(String::as_str),
                );
                let error = OpError::new(OpErrorKind::InvalidInput, "unknown_pass", message);
                match close {
                    Some(close) => error.with_suggestion(format!("did you mean '{close}'?")),
                    None => error,
                }
            }
            AnalyzeError::UnusableReport(error) => error,
            AnalyzeError::MinNeedsTestResults => {
                OpError::new(OpErrorKind::PreconditionFailed, "no_test_results", message)
            }
        }
    }
}

/// Run the selected passes over `view`; the extension passes in `runtime`,
/// when there is one (a rootless analysis has none and runs none).
pub fn analyze(
    view: &ProjectView,
    runtime: Option<&dyn WasmRuntime>,
    options: &AnalyzeOptions,
) -> Result<AnalyzeOutcome, AnalyzeError> {
    analyze_via(view, runtime, options, &crate::prove::run_prove_with)
}

/// The prove step as the operation sees it; tests swap in a scripted solver.
type ProveFn<'a> = &'a dyn Fn(&AnalysisContext, &ProveOptions) -> crate::prove::ProveReport;

fn analyze_via(
    view: &ProjectView,
    runtime: Option<&dyn WasmRuntime>,
    options: &AnalyzeOptions,
    prove: ProveFn,
) -> Result<AnalyzeOutcome, AnalyzeError> {
    let selection = select(view, &options.pass)?;
    let report = read_report(view, &options.report)?;
    if options.min.is_some() && report.is_none() {
        return Err(AnalyzeError::MinNeedsTestResults);
    }

    let orphans = find_orphans(view.graph(), report.as_deref());

    let registries = view.registries();
    let base = AnalysisContext {
        graph: view.graph(),
        kind_registry: &registries.kinds,
        field_registry: &registries.fields,
        entities: view.entities(),
        project_root: view.root(),
        test_results: report.as_deref(),
        proved_claims: None,
    };

    // The proof pass runs first: its entailment verdicts feed the coverage
    // discharge funnel (a proved claim discharges `verify property`
    // obligations without executable tests).
    let proved = options
        .prove
        .as_ref()
        .map(|prove_options| prove(&base, prove_options));
    let proved_claims: std::collections::HashSet<String> = proved
        .as_ref()
        .map(|r| r.proved_claim_ids.iter().cloned().collect())
        .unwrap_or_default();
    let input = AnalysisContext {
        proved_claims: proved.as_ref().map(|_| &proved_claims),
        ..base
    };

    let mut passes_run: Vec<PassOutcome> = Vec::new();
    for name in &selection.builtins {
        if let Some(r) = crate::builtin_passes::run_pass(&input, name) {
            passes_run.push(PassOutcome {
                name: r.name.to_string(),
                description: r.description.to_string(),
                findings: r.findings,
                summary: r.summary,
            });
        }
    }
    if view.root().is_some()
        && let Some(runtime) = runtime
    {
        // Declared `after` constraints order a single extension's passes;
        // across extensions they are advisory.
        passes_run.extend(
            passes::run_extension_passes(&registries.passes, &input, runtime, &selection.extension)
                .into_iter()
                .map(|r| PassOutcome {
                    name: r.name,
                    description: "extension compiler pass".to_string(),
                    findings: r.findings,
                    summary: r.summary,
                }),
        );
    }
    if let Some(r) = proved {
        passes_run.push(PassOutcome {
            name: "prove".to_string(),
            description: "declared bounds and claims verified with an SMT solver".to_string(),
            findings: r.findings,
            summary: r.summary,
        });
    }

    // Strictness and the error state, once, over every report.
    let policy = specforge_project::DiagnosticPolicy::strict(options.strict);
    let mut ok = true;
    for r in &mut passes_run {
        policy.promote(&mut r.findings);
        if r.findings.iter().any(|d| d.severity == Severity::Error) {
            ok = false;
        }
    }
    let gate = Gate::of(options.min, &passes_run);
    Ok(AnalyzeOutcome {
        ok,
        passes: passes_run,
        gate,
        orphans,
    })
}

/// Report entries for entities the graph does not know. Matching is exact;
/// a close match is only a hint.
fn find_orphans(graph: &Graph, report: Option<&TestReport>) -> Vec<Orphan> {
    let Some(report) = report else {
        return Vec::new();
    };
    report
        .results
        .keys()
        .filter(|id| graph.node(id).is_none())
        .map(|id| Orphan {
            entity_id: id.clone(),
            near: specforge_common::suggest::find_close_match(
                id,
                graph.nodes().iter().map(|n| n.id.raw.as_str()),
            )
            .map(str::to_string),
        })
        .collect()
}

struct Selection {
    builtins: Vec<&'static str>,
    /// What `run_extension_passes` is asked for: `all` or one full name.
    extension: String,
}

fn select(view: &ProjectView, requested: &str) -> Result<Selection, AnalyzeError> {
    let one = |builtins: Vec<&'static str>, extension: &str| Selection {
        builtins,
        extension: extension.to_string(),
    };
    if requested == EVERY_PASS {
        return Ok(one(PASS_NAMES.to_vec(), EVERY_PASS));
    }
    // Coverage is an extension pass (ADR 0002).
    if requested == "coverage" {
        return Ok(one(Vec::new(), COVERAGE_PASS));
    }
    if let Some(name) = PASS_NAMES.iter().find(|n| **n == requested) {
        return Ok(one(vec![name], requested));
    }
    let declared = declared_pass_names(view);
    if declared.iter().any(|n| n == requested) {
        return Ok(one(Vec::new(), requested));
    }
    let mut available: Vec<String> = [EVERY_PASS, "coverage"].map(String::from).to_vec();
    available.extend(PASS_NAMES.iter().map(|n| n.to_string()));
    available.extend(declared);
    Err(AnalyzeError::UnknownPass {
        requested: requested.to_string(),
        available,
    })
}

/// `<extension>:<pass>` of every analyze-phase pass the extensions declare.
fn declared_pass_names(view: &ProjectView) -> Vec<String> {
    if view.root().is_none() {
        return Vec::new();
    }
    view.registries()
        .passes
        .iter()
        .filter(|p| !p.is_check_phase())
        .map(DeclaredPass::full_name)
        .collect()
}

fn read_report(
    view: &ProjectView,
    source: &ReportSource,
) -> Result<Option<Arc<TestReport>>, AnalyzeError> {
    match source {
        ReportSource::None => Ok(None),
        ReportSource::File(path) => crate::report::named(path).map(Some),
        ReportSource::Recorded => view.test_report(),
    }
    .map_err(AnalyzeError::UnusableReport)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use specforge_extension_sdk::prelude::*;
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::runtime::{WasmCallResult, WasmTrapInfo};
    use specforge_wasm::testing::InProcessRuntime;
    use std::sync::Mutex;

    const EXT: &str = "@t/x";

    /// `@t/x`, declaring `scan` (warns W900) and `hidden` (check phase);
    /// the runtime records the input every pass receives.
    fn scanning_extension() -> InProcessRuntime {
        InProcessRuntime::new().with(|| {
            let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
            c.pass("scan", |p| {
                p.run(|_: &PassInput| vec![PassDiagnostic::warning("W900", "scanned")]);
            });
            c.pass("hidden", |p| {
                p.phase("check")
                    .run(|_: &PassInput| Vec::<PassDiagnostic>::new());
            });
            c
        })
    }

    /// The `proved_claims` each `scan` call received (`null` when absent).
    fn proved_seen(runtime: &InProcessRuntime) -> Vec<Value> {
        runtime
            .calls()
            .into_iter()
            .filter(|c| c.export == "__pass_scan")
            .map(|c| c.input.get("proved_claims").cloned().unwrap_or(Value::Null))
            .collect()
    }

    struct Project {
        graph: Graph,
        env: specforge_project::Environment,
        recorded: std::sync::OnceLock<coverage::RecordedCoverage>,
        dir: tempfile::TempDir,
    }

    /// `name`, a pass of `extension` in `phase`.
    fn declared(extension: &str, name: &str, phase: Option<&str>) -> DeclaredPass {
        DeclaredPass {
            extension: extension.to_string(),
            pass: specforge_protocol_types::CompilerPassDescriptor {
                name: name.to_string(),
                phase: phase.map(str::to_string),
                ..Default::default()
            },
        }
    }

    impl Project {
        fn new() -> Self {
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::write(dir.path().join("specforge.json"), "{}").unwrap();
            let mut registries = specforge_registry::RegistryBuild::default();
            registries.passes = vec![
                declared(EXT, "scan", None),
                declared(EXT, "hidden", Some("check")),
            ];
            Self {
                graph: Graph::new(),
                env: specforge_project::Environment::with_registries(registries),
                recorded: std::sync::OnceLock::new(),
                dir,
            }
        }

        fn view(&self) -> ProjectView<'_> {
            self.view_at(Some(self.dir.path()))
        }

        /// The project's view rooted at `root`: what a view reads at its
        /// root, never in an ancestor.
        fn view_at<'a>(&'a self, root: Option<&'a std::path::Path>) -> ProjectView<'a> {
            ProjectView::new(
                &self.graph,
                &self.env,
                root,
                self.recorded
                    .get_or_init(|| coverage::RecordedCoverage::over(&self.graph, &self.env)),
            )
        }

        fn run(&self, options: &AnalyzeOptions) -> Result<AnalyzeOutcome, AnalyzeError> {
            analyze(&self.view(), Some(&scanning_extension()), options)
        }
    }

    fn names(outcome: &AnalyzeOutcome) -> Vec<&str> {
        outcome.passes.iter().map(|p| p.name.as_str()).collect()
    }

    fn pass(name: &str) -> AnalyzeOptions {
        AnalyzeOptions {
            pass: name.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn default_options_run_every_pass_in_order() {
        let outcome = Project::new().run(&AnalyzeOptions::default()).unwrap();
        assert_eq!(names(&outcome), vec!["contracts", "@t/x:scan"]);
        assert!(outcome.ok, "a warning is not an error");
    }

    #[test]
    fn strict_promotes_warnings_and_ok_follows() {
        let outcome = Project::new()
            .run(&AnalyzeOptions {
                strict: true,
                ..Default::default()
            })
            .unwrap();
        assert!(!outcome.ok);
        assert_eq!(outcome.passes[1].findings[0].severity, Severity::Error);
    }

    #[test]
    fn json_shape_is_ok_and_passes() {
        let outcome = Project::new().run(&pass("contracts")).unwrap();
        assert_eq!(
            outcome.to_json(),
            json!({
                "ok": true,
                "passes": [{
                    "pass": "contracts",
                    "findings": [],
                    "summary": {
                        "contract_entities": 0,
                        "unconstrained_entities": 0,
                        "obligation_references": 0
                    }
                }]
            })
        );
    }

    #[specforge_test(
        behavior = "call_extension_exports",
        verify = "an analyze pass that traps is reported as an E028 finding of that pass"
    )]
    fn an_analyze_pass_that_traps_is_an_e028_finding_and_the_analysis_fails() {
        let project = Project::new();
        for answer in [
            WasmCallResult::Trap(WasmTrapInfo {
                kind: "call_failed".into(),
                message: "unreachable: the pass panicked".into(),
                export_name: "__pass_scan".into(),
            }),
            WasmCallResult::Ok(b"not diagnostics".to_vec()),
        ] {
            let runtime = InProcessRuntime::new().answer_raw(EXT, "__pass_scan", answer);
            let outcome =
                analyze(&project.view(), Some(&runtime), &AnalyzeOptions::default()).unwrap();
            assert!(!outcome.ok, "a failed pass fails the analysis");
            let scan = outcome
                .passes
                .iter()
                .find(|p| p.name == "@t/x:scan")
                .expect("the failed pass has its report");
            assert_eq!(scan.findings.len(), 1, "{:?}", scan.findings);
            assert_eq!(scan.findings[0].code, "E028");
            assert_eq!(scan.findings[0].severity, Severity::Error);
            assert!(
                scan.findings[0]
                    .message
                    .starts_with("compiler pass __pass_scan() of '@t/x' "),
                "{}",
                scan.findings[0].message
            );
            assert_eq!(scan.summary["failed"], true);
        }
    }

    #[test]
    fn a_declared_extension_pass_runs_alone_by_its_full_name() {
        let outcome = Project::new().run(&pass("@t/x:scan")).unwrap();
        assert_eq!(names(&outcome), vec!["@t/x:scan"]);
    }

    #[test]
    fn coverage_is_an_alias_for_the_testing_pass() {
        let outcome = Project::new().run(&pass("coverage")).unwrap();
        assert!(outcome.passes.is_empty(), "no testing extension declared");
    }

    #[test]
    fn unknown_and_check_phase_passes_are_refused_with_the_available_ones() {
        for requested in ["nope", "@t/x:hidden", "@t/x:typo"] {
            let err = Project::new().run(&pass(requested)).unwrap_err();
            assert_eq!(
                err,
                AnalyzeError::UnknownPass {
                    requested: requested.to_string(),
                    available: ["all", "coverage", "contracts", "@t/x:scan"]
                        .map(String::from)
                        .to_vec(),
                }
            );
        }
    }

    #[test]
    fn extension_passes_are_skipped_without_a_root() {
        let project = Project::new();
        let view = project.view_at(None);
        let outcome = analyze(
            &view,
            Some(&scanning_extension()),
            &AnalyzeOptions::default(),
        )
        .unwrap();
        assert_eq!(names(&outcome), vec!["contracts"]);
    }

    #[test]
    fn an_unusable_report_is_refused_as_e045() {
        let project = Project::new();
        std::fs::write(
            project.dir.path().join("specforge-report.json"),
            "{not json",
        )
        .unwrap();
        let AnalyzeError::UnusableReport(e) = project.run(&AnalyzeOptions::default()).unwrap_err()
        else {
            panic!("expected UnusableReport");
        };
        assert_eq!(e.code, "E045");

        let missing = AnalyzeOptions {
            report: ReportSource::File(project.dir.path().join("absent.json")),
            ..Default::default()
        };
        assert!(matches!(
            project.run(&missing).unwrap_err(),
            AnalyzeError::UnusableReport(_)
        ));
    }

    #[test]
    fn a_report_source_of_none_ignores_a_broken_recorded_report() {
        let project = Project::new();
        std::fs::write(
            project.dir.path().join("specforge-report.json"),
            "{not json",
        )
        .unwrap();
        let options = AnalyzeOptions {
            report: ReportSource::None,
            ..Default::default()
        };
        assert!(project.run(&options).is_ok());
    }

    #[specforge_test(
        behavior = "read_views_over_the_project_view",
        verify = "the recorded test report is read at the view's root, never an ancestor's"
    )]
    fn a_sub_path_does_not_read_the_ancestors_report() {
        let project = Project::new();
        std::fs::write(project.dir.path().join("specforge-report.json"), "{}").unwrap();
        let sub = project.dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let view = project.view_at(Some(&sub));
        let min = AnalyzeOptions {
            min: Some(50.0),
            ..Default::default()
        };
        // The view rooted at the sub-path reads its root and nothing above.
        assert_eq!(
            analyze(&view, Some(&scanning_extension()), &min).unwrap_err(),
            AnalyzeError::MinNeedsTestResults
        );
        assert!(analyze(&project.view(), Some(&scanning_extension()), &min).is_ok());
    }

    #[test]
    fn min_needs_a_report_and_comes_after_the_other_checks() {
        let project = Project::new();
        let min = AnalyzeOptions {
            min: Some(50.0),
            ..Default::default()
        };
        assert_eq!(
            project.run(&min).unwrap_err(),
            AnalyzeError::MinNeedsTestResults
        );

        let both = AnalyzeOptions {
            pass: "nope".to_string(),
            ..min.clone()
        };
        assert!(matches!(
            project.run(&both).unwrap_err(),
            AnalyzeError::UnknownPass { .. }
        ));

        std::fs::write(project.dir.path().join("specforge-report.json"), "{}").unwrap();
        assert!(project.run(&min).is_ok());
    }

    #[test]
    fn prove_runs_last_and_tells_the_passes_it_ran() {
        let project = Project::new();
        let fake = scanning_extension();
        analyze(&project.view(), Some(&fake), &AnalyzeOptions::default()).unwrap();
        assert_eq!(proved_seen(&fake), vec![Value::Null]);

        let fake = scanning_extension();
        let options = AnalyzeOptions {
            prove: Some(ProveOptions::default()),
            ..Default::default()
        };
        let outcome = analyze(&project.view(), Some(&fake), &options).unwrap();
        assert_eq!(names(&outcome), vec!["contracts", "@t/x:scan", "prove"]);
        assert_eq!(proved_seen(&fake), vec![json!([])]);
    }

    /// A `@specforge/testing` whose `coverage` pass answers `summary`.
    fn coverage_answering(summary: Value) -> InProcessRuntime {
        InProcessRuntime::new().with(move || {
            let summary = summary.clone();
            let mut c =
                ContributionsBuilder::new(ExtensionMeta::new("@specforge/testing", "1.0.0"));
            c.pass("coverage", |p| {
                p.run(move |_: &PassInput| PassOutput {
                    diagnostics: Vec::new(),
                    summary: summary.as_object().cloned().unwrap_or_default(),
                });
            });
            c
        })
    }

    /// A project with a (blank) recorded report, gated at `min`, whose
    /// testing extension reports `proven` of `total`.
    fn gate_of(pass_name: &str, min: Option<f64>, summary: Value) -> Gate {
        let mut project = Project::new();
        project.env.registries.passes = vec![declared("@specforge/testing", "coverage", None)];
        std::fs::write(project.dir.path().join("specforge-report.json"), "{}").unwrap();
        let options = AnalyzeOptions {
            pass: pass_name.to_string(),
            min,
            ..Default::default()
        };
        analyze(
            &project.view(),
            Some(&coverage_answering(summary)),
            &options,
        )
        .unwrap()
        .gate
    }

    fn tally(proven: usize, total: usize) -> Value {
        serde_json::to_value(coverage::Summary {
            testable_proven: proven,
            testable_total: total,
            ..Default::default()
        })
        .unwrap()
    }

    #[specforge_test(
        behavior = "te_coverage_gate",
        verify = "the analysis reports where the gate landed and leaves the analysis result alone"
    )]
    fn the_gate_is_not_requested_without_min() {
        assert_eq!(gate_of("all", None, tally(0, 4)), Gate::NotRequested);
    }

    #[specforge_test(
        behavior = "te_coverage_gate",
        verify = "the analysis reports where the gate landed and leaves the analysis result alone"
    )]
    fn the_gate_is_met_at_or_above_the_minimum() {
        assert_eq!(gate_of("all", Some(50.0), tally(2, 4)), Gate::Met);
        assert_eq!(gate_of("coverage", Some(50.0), tally(3, 4)), Gate::Met);
    }

    #[specforge_test(
        behavior = "te_coverage_gate",
        verify = "the analysis reports where the gate landed and leaves the analysis result alone"
    )]
    fn nothing_testable_satisfies_any_minimum() {
        assert_eq!(gate_of("all", Some(100.0), tally(0, 0)), Gate::Met);
    }

    #[specforge_test(
        behavior = "te_coverage_gate",
        verify = "the analysis reports where the gate landed and leaves the analysis result alone"
    )]
    fn the_gate_is_below_with_the_numbers_to_print() {
        assert_eq!(
            gate_of("all", Some(75.0), tally(1, 4)),
            Gate::Below {
                pct: 25.0,
                min: 75.0,
                proven: 1,
                total: 4
            }
        );
    }

    #[specforge_test(
        behavior = "te_coverage_gate",
        verify = "the analysis reports where the gate landed and leaves the analysis result alone"
    )]
    fn a_failed_gate_leaves_ok_and_the_reports_alone() {
        let mut project = Project::new();
        project.env.registries.passes.clear();
        std::fs::write(project.dir.path().join("specforge-report.json"), "{}").unwrap();
        let outcome = project
            .run(&AnalyzeOptions {
                min: Some(10.0),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(outcome.gate, Gate::NoCoveragePass);
        assert!(outcome.ok);
        assert_eq!(names(&outcome), vec!["contracts"]);
    }

    #[specforge_test(
        behavior = "te_coverage_gate",
        verify = "a gate without a readable coverage pass is not met"
    )]
    fn selecting_a_pass_other_than_coverage_has_no_coverage_pass() {
        assert_eq!(
            gate_of("contracts", Some(10.0), tally(4, 4)),
            Gate::NoCoveragePass
        );
    }

    #[specforge_test(
        behavior = "te_coverage_gate",
        verify = "a gate without a readable coverage pass is not met"
    )]
    fn a_summary_of_another_shape_is_unreadable_not_zero() {
        assert_eq!(
            gate_of("all", Some(10.0), json!({"testable_total": "many"})),
            Gate::UnreadableSummary("invalid type: string \"many\", expected usize".into())
        );
    }

    struct NoZ3;
    impl crate::prove::Solver for NoZ3 {
        fn version(&self) -> Option<String> {
            None
        }
        fn solve(&self, _: &str) -> Result<String, crate::prove::SolveFailure> {
            unreachable!("no solver, nothing to solve")
        }
    }

    fn prove_options(secs: u64) -> AnalyzeOptions {
        AnalyzeOptions {
            prove: Some(ProveOptions {
                z3_timeout: std::time::Duration::from_secs(secs),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn the_z3_timeout_reaches_the_prove_step_and_its_claims_reach_coverage() {
        let project = Project::new();
        let fake = scanning_extension();
        let seen = Mutex::new(None);
        let prove = |ctx: &AnalysisContext, o: &ProveOptions| {
            *seen.lock().unwrap() = Some(o.z3_timeout);
            let mut r = crate::prove::analyze_with(ctx, &NoZ3);
            r.proved_claim_ids = vec!["claim_a".to_string()];
            r
        };
        analyze_via(&project.view(), Some(&fake), &prove_options(7), &prove).unwrap();
        assert_eq!(
            *seen.lock().unwrap(),
            Some(std::time::Duration::from_secs(7))
        );
        assert_eq!(proved_seen(&fake), vec![json!(["claim_a"])]);
    }

    #[test]
    fn prove_with_z3_missing_is_a_last_w098_report_with_empty_proved_claims() {
        let project = Project::new();
        let fake = scanning_extension();
        let prove =
            |ctx: &AnalysisContext, _: &ProveOptions| crate::prove::analyze_with(ctx, &NoZ3);
        let mut options = prove_options(1);
        let lenient = analyze_via(&project.view(), Some(&fake), &options, &prove).unwrap();
        assert_eq!(names(&lenient).last().copied(), Some("prove"));
        let report = lenient.passes.last().unwrap();
        assert!(report.findings.iter().any(|f| f.code == "W098"));
        assert!(lenient.ok);
        assert_eq!(proved_seen(&fake), vec![json!([])]);

        options.strict = true;
        let strict = analyze_via(
            &project.view(),
            Some(&scanning_extension()),
            &options,
            &prove,
        )
        .unwrap();
        assert!(!strict.ok, "strict promotes the prove report too");
    }

    fn write_report(project: &Project, ids: &[&str]) {
        let results: serde_json::Map<String, Value> = ids
            .iter()
            .map(|id| {
                (
                    id.to_string(),
                    json!({"tests": [{"name": "t", "status": "pass"}]}),
                )
            })
            .collect();
        std::fs::write(
            project.dir.path().join("specforge-report.json"),
            json!({"runner": "r", "results": results}).to_string(),
        )
        .unwrap();
    }

    fn add_entity(project: &mut Project, id: &str) {
        use specforge_common::{SourceSpan, Sym};
        use specforge_parser::{EntityId, EntityKind, FieldMap};
        project.graph.add_node(specforge_graph::Node {
            id: EntityId { raw: Sym::new(id) },
            kind: EntityKind {
                raw: Sym::new("behavior"),
            },
            title: None,
            fields: FieldMap::new(),
            source_span: SourceSpan {
                file: Sym::new("t.spec"),
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            },
            methods: Vec::new(),
        });
    }

    #[test]
    fn a_record_for_an_unknown_entity_is_an_orphan_with_a_close_match() {
        let mut project = Project::new();
        add_entity(&mut project, "widget");
        write_report(&project, &["widget", "wodget", "zzzzzzzz"]);
        let outcome = project.run(&pass("contracts")).unwrap();
        assert_eq!(
            outcome.orphans,
            vec![
                Orphan {
                    entity_id: "wodget".to_string(),
                    near: Some("widget".to_string()),
                },
                Orphan {
                    entity_id: "zzzzzzzz".to_string(),
                    near: None,
                },
            ]
        );
        assert_eq!(
            outcome.to_json()["orphans"],
            json!([
                {"entity_id": "wodget", "near": "widget"},
                {"entity_id": "zzzzzzzz", "near": null}
            ])
        );
    }

    #[test]
    fn orphans_are_never_promoted_by_strict_and_change_neither_ok_nor_the_list() {
        let mut project = Project::new();
        add_entity(&mut project, "widget");
        write_report(&project, &["wodget"]);
        let lax = project.run(&pass("contracts")).unwrap();
        let strict = project
            .run(&AnalyzeOptions {
                strict: true,
                ..pass("contracts")
            })
            .unwrap();
        assert!(lax.ok && strict.ok);
        assert_eq!(lax.orphans, strict.orphans);
        assert_eq!(strict.orphans.len(), 1);
        assert!(strict.passes.iter().all(|p| p.findings.is_empty()));
    }

    #[test]
    fn the_orphans_key_is_absent_when_there_are_none() {
        let mut project = Project::new();
        add_entity(&mut project, "widget");
        write_report(&project, &["widget"]);
        let outcome = project.run(&pass("contracts")).unwrap();
        assert!(outcome.orphans.is_empty());
        assert!(outcome.to_json().get("orphans").is_none());

        let none = Project::new().run(&pass("contracts")).unwrap();
        assert!(none.to_json().get("orphans").is_none());
    }
}
