//! `specforge collect` — run the project's test runners and record which
//! entities their tests prove in `specforge-report.json` (ADR 0002).
//!
//! Runners are extensions (`@specforge/cargo-test`, …). Each declares a
//! collector: the files that select it, the command that runs it, where
//! the report lands, and a pure export that maps the report to entities.
//! This command runs the declared command after the user approves it for
//! the project, then hands the report to the extension. `--no-run` and
//! `--report` skip running and parse an existing report instead. The flow
//! itself lives in [`specforge_ops::analysis::collect`], shared with MCP; this
//! adapter only hands the operation who can be asked (`--yes`, the
//! terminal, or nobody), the machine it runs on (`ProcessMachine`, ADR 0125) and this machine's approvals
//! (`UserConsent`, ADR 0335 D2.5), and presents the
//! outcome.

use crate::notes::Notes;
use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal};
use crate::terminal::{Terminal, outln};
use specforge_ops::analysis::collect::{
    self, Collector, Mode, ProcessMachine, Request, RunnerOutput,
};
use specforge_ops::view::ProjectView;
use std::path::PathBuf;

// `specforge collect`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    project: crate::project::ProjectArg,

    /// Collector to use (e.g. cargo-test). Detected from project files if omitted.
    #[arg(long, alias = "collector", value_name = "NAME")]
    runner: Option<String>,

    /// Parse the runner's existing report instead of running it
    #[arg(long)]
    no_run: bool,

    /// Report files to parse instead of running the runner (implies --no-run)
    #[arg(long = "report", value_name = "FILE")]
    reports: Vec<PathBuf>,

    /// Plan a writing collector's edits and write none of them
    #[arg(long)]
    dry_run: bool,

    /// Run the declared test command without asking (for CI)
    #[arg(long)]
    yes: bool,

    #[command(flatten)]
    effects: crate::effects::EffectsOption,

    #[command(flatten)]
    output: OutputArg,
}

impl Args {
    pub(crate) fn run(self, terminal: &Terminal, ports: &specforge_effects::Ports) -> Exit {
        collect_at(&self, terminal, ports)
    }
}

fn collect_at(options: &Args, terminal: &Terminal, ports: &specforge_effects::Ports) -> Exit {
    let format = options.output.format;
    // The project the path is in (ADR 0029 D6); collect refuses one that is none.
    let root = match options.project.root() {
        Ok(root) => root,
        Err(error) => return Refusal::of(format).report(terminal, &error),
    };

    let project = crate::project::compile_at(&root, ports);

    let mode = if !options.reports.is_empty() {
        Mode::Reports(&options.reports)
    } else if options.no_run {
        Mode::NoRun
    } else if format == OutputFormat::Json {
        // Keep stdout for the JSON result.
        Mode::Run(RunnerOutput::Stderr)
    } else {
        Mode::Run(RunnerOutput::Inherit)
    };
    let notes = Notes::new(terminal, format);
    let mut announce = |collector: &Collector, argv: &[String]| {
        notes.person(format_args!(
            "running {} ({}): {}",
            collector.name,
            collector.extension,
            argv.join(" ")
        ));
    };
    let consent = specforge_ops::consent::UserConsent::user(root.dir());
    let request = Request {
        runner: options.runner.as_deref(),
        mode,
        announce: &mut announce,
        caller: crate::caller::of(options.yes, format, terminal).lending(
            ports,
            &consent,
            crate::effects::lent(options.effects.effects),
        ),
    };
    // The view is rooted where the project compiled: the report lands where
    // every view over the project reads it.
    let view = ProjectView::of(&project);
    let mut outcome = match collect::collect(
        &view,
        request,
        &ProcessMachine::new(std::sync::Arc::clone(&ports.process)),
        &consent,
    ) {
        Ok(outcome) => outcome,
        Err(e) => {
            return Refusal::of(format).report(terminal, &e);
        }
    };
    // A writing collector's plans are applied after the report was written (ADR 0289).
    let plans = std::mem::take(&mut outcome.plans);
    let facts = std::mem::take(&mut outcome.facts);
    let caller = crate::caller::of(options.yes, format, terminal);
    let outcome = match specforge_ops::authoring::apply::apply_collected(
        &view,
        plans,
        facts,
        options.dry_run,
        caller,
    ) {
        Ok(applied) => outcome.with_applied(applied),
        Err(e) => return Refusal::of(format).report(terminal, &e),
    };
    for why in &outcome.unsaved_approvals {
        notes.warning(format_args!(
            "warning: could not remember the approval: {why}"
        ));
    }

    if format == OutputFormat::Json {
        terminal.json(&outcome.document());
    } else {
        for r in &outcome.runners {
            if let Some(why) = &r.skip_reason {
                outln!(terminal, "{}: skipped: {why}", r.name);
                continue;
            }
            let s = &r.stats;
            let by_convention = match r.by_convention {
                0 => String::new(),
                n => format!(", {n} linked by naming convention"),
            };
            outln!(
                terminal,
                "{}: {} entities, {} passed, {} failed, {} skipped ({} report file(s){by_convention})",
                r.name,
                s.entities,
                s.passed,
                s.failed,
                s.skipped,
                r.files
            );
            if r.facts > 0 {
                outln!(terminal, "  facts: {} stored", r.facts);
            }
            if let Some(plan) = &r.plan {
                let n = plan.report.affected_files.len();
                if plan.dry_run {
                    outln!(
                        terminal,
                        "  plan: {n} file(s) would change; nothing was written"
                    );
                    if !plan.report.diff.is_empty() {
                        outln!(terminal, "{}", plan.report.diff.trim_end());
                    }
                } else {
                    outln!(terminal, "  plan: wrote {n} file(s)");
                }
            }
            if let Some(code) = r.exit_code.filter(|c| *c != 0) {
                outln!(
                    terminal,
                    "  the runner exited with status {code}; results of the tests that ran are recorded"
                );
            }
        }
        for d in &outcome.diagnostics {
            outln!(terminal, "{}: {}", d.code, d.message);
            if let Some(hint) = &d.suggestion {
                outln!(terminal, "  hint: {hint}");
            }
        }
        match &outcome.report {
            Some(report) => outln!(terminal, "report written: {}", report.display()),
            None => outln!(terminal, "report unchanged"),
        }
    }
    Exit::Passed
}
