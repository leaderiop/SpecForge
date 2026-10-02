//! `specforge analyze`: run the analysis passes over a [`ProjectView`].
//!
//! One operation serves the CLI (and, later, MCP): it validates the pass
//! selection, reads the test report, runs the built-in passes, the
//! extension passes (through the [`WasmRuntime`] port) and, when asked,
//! `prove`, then applies strictness once and computes `ok` once. The shape of
//! the JSON document lives in [`AnalyzeOutcome::to_json`] and nowhere else.
//!
//! Surfaces keep rendering, exit-code or error-channel mapping, and the
//! choice of which project to analyse.

use std::path::{Path, PathBuf};

use specforge_common::{Diagnostic, Severity};
use specforge_emitter::analyze::{self as passes, AnalysisContext, COVERAGE_PASS, PASS_NAMES};
use specforge_emitter::compile::CompilationContext;
use specforge_emitter::coverage::{self, ReportError};
use specforge_graph::Graph;
use specforge_registry::validation_engine::ValidationRulePattern;
use specforge_registry::{FieldRegistry, KindRegistry, ManifestV2};
use specforge_wasm::runtime::WasmRuntime;

use crate::OpError;

/// The read-only slice of a compiled project an analysis reads, borrowed.
/// `root` is the project path as the caller gave it; without one, extension
/// passes are skipped and the recorded report is not looked for.
#[derive(Clone, Copy)]
pub struct ProjectView<'a> {
    pub graph: &'a Graph,
    pub kind_registry: &'a KindRegistry,
    pub field_registry: &'a FieldRegistry,
    pub rules: &'a [(ValidationRulePattern, String)],
    pub manifests: &'a [ManifestV2],
    pub root: Option<&'a Path>,
}

impl<'a> ProjectView<'a> {
    /// The view of a compiled project rooted at `root`.
    pub fn of(ctx: &'a CompilationContext, root: &'a Path) -> Self {
        Self {
            graph: &ctx.graph,
            kind_registry: &ctx.kind_registry,
            field_registry: &ctx.field_registry,
            rules: &ctx.extension_rules,
            manifests: &ctx.manifests,
            root: Some(root),
        }
    }
}

/// Where the test report comes from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ReportSource {
    /// What `specforge collect` last recorded in the project, if anything.
    #[default]
    Recorded,
    /// A named report file, which must exist.
    File(PathBuf),
    /// No report.
    None,
}

pub use crate::prove::ProveOptions;

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
            pass: "all".to_string(),
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

/// What an analysis found. Strictness is already applied.
#[derive(Debug, Clone)]
pub struct AnalyzeOutcome {
    /// No finding is an error, after strict promotion.
    pub ok: bool,
    /// Built-in passes, extension passes in their declared order, `prove` last.
    pub passes: Vec<PassOutcome>,
}

impl AnalyzeOutcome {
    /// The JSON document `{ok, passes: [{pass, findings, summary}]}`.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
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
        })
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
            AnalyzeError::MinNeedsTestResults => {
                f.write_str("a minimum coverage needs test results")
            }
        }
    }
}

impl std::error::Error for AnalyzeError {}

/// Run the selected passes over `view`.
pub fn analyze(
    view: &ProjectView,
    runtime: &dyn WasmRuntime,
    options: &AnalyzeOptions,
) -> Result<AnalyzeOutcome, AnalyzeError> {
    analyze_via(view, runtime, options, &crate::prove::run_prove_with)
}

/// The prove step as the operation sees it; tests swap in a scripted solver.
type ProveFn<'a> = &'a dyn Fn(&AnalysisContext, &ProveOptions) -> crate::prove::ProveReport;

fn analyze_via(
    view: &ProjectView,
    runtime: &dyn WasmRuntime,
    options: &AnalyzeOptions,
    prove: ProveFn,
) -> Result<AnalyzeOutcome, AnalyzeError> {
    let selection = select(view, runtime, &options.pass)?;
    let report = read_report(view, &options.report)?;
    if options.min.is_some() && report.is_none() {
        return Err(AnalyzeError::MinNeedsTestResults);
    }

    let base = AnalysisContext {
        graph: view.graph,
        kind_registry: view.kind_registry,
        field_registry: view.field_registry,
        rules: view.rules,
        project_root: view.root,
        test_results: report.as_ref(),
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
        if let Some(r) = passes::run_pass(&input, name) {
            passes_run.push(PassOutcome {
                name: r.name.to_string(),
                description: r.description.to_string(),
                findings: r.findings,
                summary: r.summary,
            });
        }
    }
    if view.root.is_some() {
        // Declared `after` constraints order a single extension's passes;
        // across extensions they are advisory.
        passes_run.extend(
            passes::run_extension_passes(view.manifests, &input, runtime, &selection.extension)
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
            description: "numeric constraint bounds verified with an SMT solver".to_string(),
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
    Ok(AnalyzeOutcome {
        ok,
        passes: passes_run,
    })
}

struct Selection {
    builtins: Vec<&'static str>,
    /// What `run_extension_passes` is asked for: `all` or one full name.
    extension: String,
}

fn select(
    view: &ProjectView,
    runtime: &dyn WasmRuntime,
    requested: &str,
) -> Result<Selection, AnalyzeError> {
    let one = |builtins: Vec<&'static str>, extension: &str| Selection {
        builtins,
        extension: extension.to_string(),
    };
    if requested == "all" {
        return Ok(one(PASS_NAMES.to_vec(), "all"));
    }
    // Coverage is an extension pass (ADR 0002).
    if requested == "coverage" {
        return Ok(one(Vec::new(), COVERAGE_PASS));
    }
    if let Some(name) = PASS_NAMES.iter().find(|n| **n == requested) {
        return Ok(one(vec![name], requested));
    }
    let declared = declared_pass_names(view, runtime);
    if declared.iter().any(|n| n == requested) {
        return Ok(one(Vec::new(), requested));
    }
    let mut available: Vec<String> = ["all", "coverage"].map(String::from).to_vec();
    available.extend(PASS_NAMES.iter().map(|n| n.to_string()));
    available.extend(declared);
    Err(AnalyzeError::UnknownPass {
        requested: requested.to_string(),
        available,
    })
}

/// `<extension>:<pass>` of every analyze-phase pass the manifests declare.
fn declared_pass_names(view: &ProjectView, runtime: &dyn WasmRuntime) -> Vec<String> {
    if view.root.is_none() {
        return Vec::new();
    }
    view.manifests
        .iter()
        .flat_map(|m| {
            passes::declared_passes(runtime, &m.name)
                .into_iter()
                .filter(|p| !passes::is_check_phase(p))
                .map(|p| format!("{}:{}", m.name, p.name))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn read_report(
    view: &ProjectView,
    source: &ReportSource,
) -> Result<Option<passes::TestReport>, AnalyzeError> {
    let read = match source {
        ReportSource::None => Ok(None),
        ReportSource::File(path) => coverage::read_report_file(path).map(Some),
        ReportSource::Recorded => view
            .root
            .and_then(specforge_common::find_project_root)
            .map_or(Ok(None), |root| coverage::read_report(&root)),
    };
    read.map_err(|e: ReportError| AnalyzeError::UnusableReport(e.diagnostic().into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use specforge_wasm::runtime::{WasmCallResult, WasmTrapInfo};
    use std::sync::Mutex;

    const EXT: &str = "@t/x";

    /// An extension declaring `scan` (warns W900) and `hidden` (check
    /// phase), recording the `proved_claims` its passes receive.
    struct Fake {
        proved_seen: Mutex<Vec<Value>>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                proved_seen: Mutex::new(Vec::new()),
            }
        }
    }

    impl WasmRuntime for Fake {
        fn load_module(&self, _: &Path) -> Result<(), String> {
            Ok(())
        }

        fn call_export(&self, _ext: &str, export: &str, input: &[u8]) -> WasmCallResult {
            let ok = |v: Value| WasmCallResult::Ok(v.to_string().into_bytes());
            match export {
                "__describe" => ok(json!({"category": "passes", "items": [
                    {"name": "scan"}, {"name": "hidden", "phase": "check"}
                ]})),
                "__pass_scan" => {
                    let input: Value = serde_json::from_slice(input).unwrap();
                    self.proved_seen
                        .lock()
                        .unwrap()
                        .push(input["proved_claims"].clone());
                    ok(json!([{"code": "W900", "severity": "Warning", "message": "scanned"}]))
                }
                _ => WasmCallResult::Trap(WasmTrapInfo {
                    kind: "export_not_found".into(),
                    message: export.into(),
                    export_name: export.into(),
                }),
            }
        }
    }

    struct Project {
        graph: Graph,
        kinds: KindRegistry,
        fields: FieldRegistry,
        manifests: Vec<ManifestV2>,
        dir: tempfile::TempDir,
    }

    impl Project {
        fn new() -> Self {
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::write(dir.path().join("specforge.json"), "{}").unwrap();
            let manifest = serde_json::from_value(json!({
                "name": EXT, "version": "1.0.0", "manifestVersion": 2, "wasmPath": ""
            }))
            .unwrap();
            Self {
                graph: Graph::new(),
                kinds: KindRegistry::default(),
                fields: FieldRegistry::default(),
                manifests: vec![manifest],
                dir,
            }
        }

        fn view(&self) -> ProjectView<'_> {
            ProjectView {
                graph: &self.graph,
                kind_registry: &self.kinds,
                field_registry: &self.fields,
                rules: &[],
                manifests: &self.manifests,
                root: Some(self.dir.path()),
            }
        }

        fn run(&self, options: &AnalyzeOptions) -> Result<AnalyzeOutcome, AnalyzeError> {
            analyze(&self.view(), &Fake::new(), options)
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
        let view = ProjectView {
            root: None,
            ..project.view()
        };
        let outcome = analyze(&view, &Fake::new(), &AnalyzeOptions::default()).unwrap();
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
        let fake = Fake::new();
        analyze(&project.view(), &fake, &AnalyzeOptions::default()).unwrap();
        assert_eq!(*fake.proved_seen.lock().unwrap(), vec![Value::Null]);

        let fake = Fake::new();
        let options = AnalyzeOptions {
            prove: Some(ProveOptions::default()),
            ..Default::default()
        };
        let outcome = analyze(&project.view(), &fake, &options).unwrap();
        assert_eq!(names(&outcome), vec!["contracts", "@t/x:scan", "prove"]);
        assert_eq!(*fake.proved_seen.lock().unwrap(), vec![json!([])]);
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
        let fake = Fake::new();
        let seen = Mutex::new(None);
        let prove = |ctx: &AnalysisContext, o: &ProveOptions| {
            *seen.lock().unwrap() = Some(o.z3_timeout);
            let mut r = crate::prove::analyze_with(ctx, &NoZ3);
            r.proved_claim_ids = vec!["claim_a".to_string()];
            r
        };
        analyze_via(&project.view(), &fake, &prove_options(7), &prove).unwrap();
        assert_eq!(
            *seen.lock().unwrap(),
            Some(std::time::Duration::from_secs(7))
        );
        assert_eq!(*fake.proved_seen.lock().unwrap(), vec![json!(["claim_a"])]);
    }

    #[test]
    fn prove_with_z3_missing_is_a_last_w098_report_with_empty_proved_claims() {
        let project = Project::new();
        let fake = Fake::new();
        let prove =
            |ctx: &AnalysisContext, _: &ProveOptions| crate::prove::analyze_with(ctx, &NoZ3);
        let mut options = prove_options(1);
        let lenient = analyze_via(&project.view(), &fake, &options, &prove).unwrap();
        assert_eq!(names(&lenient).last().copied(), Some("prove"));
        let report = lenient.passes.last().unwrap();
        assert!(report.findings.iter().any(|f| f.code == "W098"));
        assert!(lenient.ok);
        assert_eq!(*fake.proved_seen.lock().unwrap(), vec![json!([])]);

        options.strict = true;
        let strict = analyze_via(&project.view(), &Fake::new(), &options, &prove).unwrap();
        assert!(!strict.ok, "strict promotes the prove report too");
    }
}
