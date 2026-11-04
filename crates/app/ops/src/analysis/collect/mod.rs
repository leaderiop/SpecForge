//! Test-result collection (ADR 0002).
//!
//! A runner extension declares a *collector*: the files that select it, the
//! command that runs its test runner, where the runner leaves its report,
//! and a pure `collect__<name>` export that maps the report to entity
//! results. The host does everything that touches the machine: it asks for
//! consent, runs the declared command, reads the report and merges the
//! extension's answer into `specforge-report.json`. The extension never
//! runs anything itself. A collector's `facts` (ADR 0327) are admitted and
//! keyed here ([`facts`]); the caller stores them with the plans.
//!
//! The CLI and MCP are adapters: they hand the operation who can be asked
//! (`crate::asking::Asking`), the machine it runs on ([`CollectorMachine`], ADR 0125) and this
//! machine's approvals (the `Consent` port, ADR 0335 D2), and present the [`Outcome`]. A collector's
//! command is its extension's process capability, run only through that extension's grant.

mod approval;
mod collector;
mod convention;
mod dispatch;
mod evidence;
mod facts;
mod merge;
mod process;

pub use crate::machine::{
    CollectorCommand, CollectorMachine, REPORT_ENV, Ran, RunFailure, RunnerOutput,
};
use collector::detect;
pub use collector::{Collector, collectors, uses_here};
pub use dispatch::dispatch;
use dispatch::dispatch_over;
use evidence::{Evidence, Source};
pub use merge::MergeStats;
use merge::{KnownEntities, merge};
pub use process::ProcessMachine;

use crate::view::ProjectView;
use crate::{Caller, Held, OpError, OpErrorKind, Refusal};
use crate::{CollectedFacts, CollectedPlan, EditDocument};
use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_common::{Code, Diagnostic, DiagnosticList, ProjectPath, codes};
use specforge_project::coverage::TestReport;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use specforge_project::coverage::REPORT_FILE;

// ── the whole flow ──────────────────────────────────────────────────────────

/// How to get each collector's report.
#[derive(Debug, Clone, Copy)]
pub enum Mode<'a> {
    /// Run the declared command (after approval), then read its report.
    Run(RunnerOutput),
    /// Read the report already at the declared location.
    NoRun,
    /// Read only these files (one collector only): nothing the collector declares about its report,
    /// capture or command is resolved or checked.
    Reports(&'a [PathBuf]),
}

/// A `collect` request: what to collect, and who decides and hears about
/// a command that runs.
pub struct Request<'a> {
    /// Collector name or extension; detected from project files when absent.
    pub runner: Option<&'a str>,
    pub mode: Mode<'a>,
    /// Told just before a collector's command runs.
    pub announce: &'a mut dyn FnMut(&Collector, &[String]),
    /// What the collect may reach and who can be asked. Every file it reads or writes must land
    /// inside `caller.containment()` (ADR 0104 D3, ADR 0112 D2): the report, a collector's
    /// report files and its captured output; a name that lands outside is refused before any
    /// command runs, for every collector whose mode reads it. `caller.asking()` decides whether a collector's command may run; the consent
    /// store is read first (ADR 0110, ADR 0335 D2.4).
    pub caller: Caller<'a>,
}

/// A failure reported as diagnostic `code` (E058, E045).
fn fail(kind: OpErrorKind, code: Code, message: impl Into<String>) -> OpError {
    OpError::coded(kind, code, message)
}

/// What happened for one collector.
#[derive(Debug, Clone, Serialize, Shape)]
pub struct RunnerResult {
    pub name: String,
    pub extension: String,
    pub ran: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub files: usize,
    #[serde(flatten)]
    pub stats: MergeStats,
    /// Tests linked by naming convention rather than by the report.
    pub by_convention: usize,
    /// The facts stored for it (ADR 0327): none when it answered none, or they were refused (E116).
    pub facts: usize,
    /// What a writing collector's plan did to the project (ADR 0289), as `specforge.edit` reports an edit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<EditDocument>,
    /// Why a collector chosen by detection gave no evidence (its E045 message: no report, no captured output, a
    /// command that cannot start), so it was skipped and the others kept (ADR 0331, amending ADR 0125 D5); nothing
    /// of it is recorded, so it reports no run and no file. A collector `--runner` names, or the one the project
    /// enabled, fails with E045 instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

impl RunnerResult {
    /// `collector`, skipped for `why`: nothing ran, read or recorded.
    fn skipped(collector: &Collector, why: String) -> Self {
        Self {
            name: collector.name.clone(),
            extension: collector.extension.clone(),
            ran: false,
            exit_code: None,
            files: 0,
            stats: MergeStats::default(),
            by_convention: 0,
            facts: 0,
            plan: None,
            skip_reason: Some(why),
        }
    }
}

/// The outcome of a successful collect.
#[derive(Debug)]
pub struct Outcome {
    /// The project the collect ran in: the document names the report from it.
    pub root: PathBuf,
    pub runners: Vec<RunnerResult>,
    pub diagnostics: Vec<Diagnostic>,
    /// The test report the collect wrote; `None` when no collector's answer changed it (a facts-only collect).
    pub report: Option<PathBuf>,
    /// Approvals given at a prompt that could not be remembered (the
    /// consent store could not be written), each with why.
    pub unsaved_approvals: Vec<String>,
    /// The plans writing collectors answered, for the caller to apply after the report is written (ADR 0289): the
    /// report is per machine and the plan is the project's, so they are separate writes. `collect` applies none
    /// (a plan is an `authoring` operation); see [`Outcome::with_applied`].
    pub plans: Vec<CollectedPlan>,
    /// The facts collectors answered and the host admitted, for the caller to store with the plans (ADR 0327).
    pub facts: CollectedFacts,
}

impl Outcome {
    /// The plans the caller applied, each under its runner's name, recorded on the runner's result.
    pub fn with_applied(mut self, applied: Vec<(String, EditDocument)>) -> Self {
        for (runner, document) in applied {
            if let Some(result) = self.runners.iter_mut().find(|r| r.name == runner) {
                result.plan = Some(document);
            }
        }
        self.plans.clear();
        self.facts = CollectedFacts::default();
        self
    }

    /// The document both surfaces answer with:
    /// `{status, runners, diagnostics, report}`.
    pub fn document(&self) -> CollectDocument {
        CollectDocument {
            status: CollectStatus::Collected,
            runners: self.runners.clone(),
            diagnostics: DiagnosticList::from(self.diagnostics.clone()),
            report: self
                .report
                .as_ref()
                .map(|report| ProjectPath::of(&self.root, report)),
        }
    }
}

/// What a collect answers with, on both surfaces.
#[derive(Debug, Clone, Serialize, Shape)]
pub struct CollectDocument {
    pub status: CollectStatus,
    pub runners: Vec<RunnerResult>,
    pub diagnostics: DiagnosticList,
    /// The merged test report the collect wrote, named from the project root; absent when it wrote none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ProjectPath>,
}

/// A collect that finished says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Shape)]
#[serde(rename_all = "lowercase")]
pub enum CollectStatus {
    Collected,
}

/// Collect test results for the project the view was compiled from:
/// select collectors among its declarations', run (in `runtime`) or read
/// each one's report, map it through the extension to the view's entities
/// and merge the answer into `<root>/specforge-report.json`: a collector's answer is merged when it carries test
/// results or the report holds that runner's tests, and the report is written only when a merge changed it. The
/// facts collectors answer are admitted (E116) and keyed (I209) here, and stored by the caller with the plans. Every refusal that needs no run (a
/// report location or capture the mode reads, a command, an approval) is decided for every selected
/// collector before the first command runs (ADR 0125 D5, D7): each command runs through its extension's grant
/// (ADR 0335 D2), the approvals it needs settled with the request's `caller.asking()` and kept in `consent`; a
/// command the grant refuses is E090. Its `announce` is told just before a command runs. The machine runs the
/// commands. Without a root, or at a root that holds no project: `no_project`.
pub fn collect(
    view: &ProjectView,
    request: Request,
    machine: &dyn CollectorMachine,
    consent: &dyn specforge_effects::Consent,
) -> Result<Outcome, Refusal<CollectField>> {
    let root = view.project_root()?;
    if !view.env().config_found() {
        return Err(OpError::no_project(format!(
            "no specforge project at {} (no specforge.json at or above it)",
            root.display()
        ))
        .into());
    }
    let Request {
        runner,
        mode,
        announce,
        caller,
    } = request;
    let (asking, containment) = (caller.asking(), caller.containment());
    let known = &KnownEntities::of(view.entities());
    let available = collectors(view.registries().declarations());
    let (selected, chosen) = select(&available, runner, root)?;
    if let Mode::Reports(_) = mode
        && selected.len() > 1
    {
        let names: Vec<&str> = selected.iter().map(|c| c.name.as_str()).collect();
        return Err(fail(
            OpErrorKind::PreconditionFailed,
            codes::E058,
            format!(
                "report files need one collector, but {} apply here; name the runner that wrote them",
                names.join(", ")
            ),
        )
        .into());
    }

    // Held before anything runs: what is read and what is written is the file checked here.
    let report_file = containment
        .resolve(&root.join(REPORT_FILE))
        .map_err(OpError::from)?;
    let mut report = load_report(&report_file);
    let loaded = serde_json::to_string(&report).expect("report serialization cannot fail");
    let mut merged = false;
    let mut stores = Vec::new();
    let mut runners = Vec::new();
    let mut plans: Vec<CollectedPlan> = Vec::new();
    // Reported whether or not this run needs an approval (ADR 0335 D2.5): `--yes` and a report-only run say it too.
    let mut diagnostics: Vec<Diagnostic> = consent
        .list()
        .err()
        .map(|error| crate::consent::unusable_store(&error))
        .into_iter()
        .collect();
    let mut unsaved_approvals = Vec::new();
    // Decided for every collector before anything runs (ADR 0125 D5): a refusal that needs no run never
    // follows an earlier collector's command.
    let planned = selected
        .into_iter()
        .map(|collector| {
            Source::plan(collector, mode, root, containment).map(|source| (collector, source))
        })
        .collect::<Result<Vec<_>, OpError>>()?;
    approval::settle_runs(view, &planned, asking, consent, &mut unsaved_approvals)?;
    for (collector, source) in planned {
        let Evidence {
            files,
            stdout,
            exit_code,
            ran,
        } = match source.gather(collector, root, containment, machine, &mut *announce) {
            Ok(evidence) => evidence,
            // Detection chose it, so the project did not ask for it: no evidence skips it (ADR 0331).
            Err(error) if chosen == Chosen::Detected && error.code == codes::E045.id() => {
                runners.push(RunnerResult::skipped(collector, error.message));
                continue;
            }
            Err(error) => return Err(error.into()),
        };

        let lent = crate::extension::lend::port_call(
            view,
            caller,
            &collector.extension,
            specforge_protocol_types::Operation::Collect,
        );
        let mut collected = dispatch_over(
            view.runtime().map(|r| r.as_ref()),
            collector,
            &files,
            stdout.as_deref(),
            Some(view),
            lent.clone(),
        )
        .map_err(|error| {
            let mut failed = OpError::of(OpErrorKind::Internal, error.diagnostic());
            // A failure has no diagnostics of its own: what a guest was refused, this collector or an earlier one,
            // goes with its message, or failing would hide it.
            let earlier = diagnostics.iter().filter(|d| codes::E090.matches(&d.code));
            for denial in earlier.chain(&error.denials) {
                failed.message.push_str(&format!("\n{denial}"));
            }
            failed
        })?;
        if let Some(lent) = lent {
            diagnostics.extend(lent.take_denials());
        }
        let (by_convention, diags) = convention::resolve(&collected.unlinked, known);
        diagnostics.extend(diags);
        let by_convention_count = by_convention.iter().map(|e| e.test_results.len()).sum();
        collected.entity_results.extend(by_convention);
        // A facts-only or plan-only answer is not merged: it would create a report, or empty a runner's, for nothing.
        let merges = !collected.entity_results.is_empty()
            || !collected.unlinked.is_empty()
            || report.results.values().any(|entity| {
                entity
                    .tests
                    .iter()
                    .any(|t| t.runner.as_deref() == Some(collector.name.as_str()))
            });
        let stats = if merges {
            merged = true;
            let (stats, diags) = merge(&mut report, &collector.name, &collected, known);
            diagnostics.extend(diags);
            stats
        } else {
            MergeStats::default()
        };
        let facts = facts::admitted(
            view,
            collector,
            std::mem::take(&mut collected.facts),
            &mut diagnostics,
        );
        let stored = facts.len();
        if !facts.is_empty() {
            stores.push((collector.name.clone(), collector.extension.clone(), facts));
        }
        if let Some(plan) = collected.plan.take() {
            plans.push(CollectedPlan {
                runner: collector.name.clone(),
                extension: collector.extension.clone(),
                writes: collector.writes,
                plan,
            });
        }
        runners.push(RunnerResult {
            name: collector.name.clone(),
            extension: collector.extension.clone(),
            ran,
            exit_code,
            files: files.len(),
            stats,
            by_convention: by_convention_count,
            facts: stored,
            plan: None,
            skip_reason: None,
        });
    }

    let changed =
        serde_json::to_string(&report).expect("report serialization cannot fail") != loaded;
    let report = if merged && changed {
        Some(save_report(&report_file, &report)?)
    } else {
        None
    };
    let facts = CollectedFacts {
        key: if stores.is_empty() {
            String::new()
        } else {
            facts::key(view, &mut diagnostics)
        },
        stores,
    };
    Ok(Outcome {
        root: root.to_path_buf(),
        runners,
        diagnostics,
        report,
        unsaved_approvals,
        plans,
        facts,
    })
}

/// The field of a collect request a refusal can be about (ADR 0072).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectField {
    /// `Request::runner`: the collector asked for.
    Runner,
}

/// How [`select`] chose the collectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chosen {
    /// `--runner` named it, or it is the one the project enabled: no evidence is E045.
    Named,
    /// Their detection files are present: one with no evidence is skipped (ADR 0331).
    Detected,
}

/// The collectors to use: the one `runner` names, else every collector
/// whose detection files are present. A runner no collector answers to is
/// invalid input about the runner (E058).
fn select<'a>(
    available: &'a [Collector],
    runner: Option<&str>,
    root: &Path,
) -> Result<(Vec<&'a Collector>, Chosen), Refusal<CollectField>> {
    if available.is_empty() {
        return Err(fail(
            OpErrorKind::PreconditionFailed,
            codes::E058,
            "no enabled extension provides a test collector; enable a runner extension \
             (e.g. `specforge add @specforge/cargo-test`)",
        )
        .into());
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
            .map(|c| (vec![c], Chosen::Named))
            .ok_or_else(|| {
                fail(
                    OpErrorKind::InvalidInput,
                    codes::E058,
                    format!("unknown runner '{runner}' (available: {})", names()),
                )
                .about(CollectField::Runner)
            });
    }
    let detected = detect(available, root);
    // A project that enabled a single runner chose it: detection only
    // decides between several (vitest configured inside vite.config.ts,
    // for one, has no file of its own to detect).
    if detected.is_empty() && available.len() == 1 {
        return Ok((vec![&available[0]], Chosen::Named));
    }
    if detected.is_empty() {
        return Err(fail(
            OpErrorKind::PreconditionFailed,
            codes::E058,
            format!(
                "no test runner detected in {}; name the runner to use (available: {})",
                root.display(),
                names()
            ),
        )
        .into());
    }
    Ok((detected, Chosen::Detected))
}

/// The report `collect` merges new results into: `specforge-report.json`,
/// or an empty report when it's missing or unreadable. Unlike the views that
/// score coverage (`CompiledProject::recorded`), collect is the writer: it
/// replaces a report it cannot read.
fn load_report(file: &Held) -> TestReport {
    file.read()
        .ok()
        .and_then(|bytes| TestReport::parse(&file.at, &bytes).ok())
        .unwrap_or(TestReport {
            runner: None,
            results: BTreeMap::new(),
        })
}

/// Write `specforge-report.json`, replacing it atomically where it is held.
fn save_report(file: &Held, report: &TestReport) -> Result<PathBuf, OpError> {
    let json = serde_json::to_string_pretty(report).expect("report serialization cannot fail");
    file.replace(json).map_err(|e| {
        OpError::coded(
            OpErrorKind::of_io(&e),
            codes::E056,
            format!("failed to write {}: {e}", file.named.display()),
        )
    })?;
    Ok(file.at.clone())
}

#[cfg(test)]
mod tests;
