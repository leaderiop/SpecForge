//! `specforge collect` — run the project's test runners and record which
//! entities their tests prove in `specforge-report.json` (ADR 0002).
//!
//! Runners are extensions (`@specforge/cargo-test`, …). Each declares a
//! collector: the files that select it, the command that runs it, where
//! the report lands, and a pure export that maps the report to entities.
//! This command runs the declared command after the user approves it for
//! the project, then hands the report to the extension. `--no-run` and
//! `--report` skip running and parse an existing report instead. The flow
//! itself lives in [`specforge_ops::collect`], shared with MCP; this
//! adapter only chooses the consent (a terminal prompt, `--yes`, or what
//! was approved before) and presents the outcome.

use crate::OutputFormat;
use specforge_common::find_project_root;
use specforge_ops::OpError;
use specforge_ops::collect::{self, Collector, Consent, Mode, Request, RunnerOutput};
use specforge_ops::view::ProjectView;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

pub struct Options<'a> {
    pub runner: Option<&'a str>,
    pub no_run: bool,
    pub reports: &'a [PathBuf],
    pub yes: bool,
}

pub fn run(path: &Path, options: &Options, format: OutputFormat) -> i32 {
    let Some(root) = find_project_root(path) else {
        let msg = "no specforge project found (missing specforge.json or specforge.spec)";
        format.print_op_error(&OpError::diagnostic("E045", msg));
        return 1;
    };

    let (project, runtime) = crate::pipeline::compile_project(&root);

    let mode = if !options.reports.is_empty() {
        Mode::Reports(options.reports)
    } else if options.no_run {
        Mode::NoRun
    } else if format == OutputFormat::Json {
        // Keep stdout for the JSON result.
        Mode::Run(RunnerOutput::Stderr)
    } else {
        Mode::Run(RunnerOutput::Inherit)
    };
    let mut prompt = |collector: &Collector, argv: &[String]| ask(collector, argv, &root);
    let consent = if options.yes {
        Consent::Yes
    } else if format == OutputFormat::Json || !std::io::stdin().is_terminal() {
        // Nobody to ask: only what was approved before runs.
        Consent::Approved
    } else {
        Consent::Prompt(&mut prompt)
    };
    let mut announce = |collector: &Collector, argv: &[String]| {
        if format != OutputFormat::Json {
            eprintln!(
                "running {} ({}): {}",
                collector.name,
                collector.extension,
                argv.join(" ")
            );
        }
    };
    let request = Request {
        runner: options.runner,
        mode,
        consent,
        announce: &mut announce,
    };
    // The view is rooted where the project compiled: the report lands where
    // every view over the project reads it.
    let outcome = match collect::collect(&ProjectView::of(&project), &runtime, request) {
        Ok(outcome) => outcome,
        Err(e) => {
            format.print_op_error(&e);
            return 1;
        }
    };
    for why in &outcome.unsaved_approvals {
        eprintln!("warning: could not remember the approval: {why}");
    }

    if format == OutputFormat::Json {
        println!(
            "{}",
            serde_json::to_string_pretty(&outcome.to_json()).expect("serialize JSON output")
        );
    } else {
        for r in &outcome.runners {
            let s = &r.stats;
            let by_convention = match r.by_convention {
                0 => String::new(),
                n => format!(", {n} linked by naming convention"),
            };
            println!(
                "{}: {} entities, {} passed, {} failed, {} skipped ({} report file(s){by_convention})",
                r.name, s.entities, s.passed, s.failed, s.skipped, r.files
            );
            if let Some(code) = r.exit_code.filter(|c| *c != 0) {
                println!(
                    "  the runner exited with status {code}; results of the tests that ran are recorded"
                );
            }
        }
        for d in &outcome.diagnostics {
            println!("{}: {}", d.code, d.message);
            if let Some(hint) = &d.suggestion {
                println!("  hint: {hint}");
            }
        }
        println!("report written: {}", outcome.report.display());
    }
    0
}

/// Ask on the terminal whether the collector's command may run here.
fn ask(collector: &Collector, argv: &[String], root: &Path) -> bool {
    let mut err = std::io::stderr();
    let _ = writeln!(
        err,
        "{} wants to run this command in {}:\n  {}",
        collector.extension,
        root.display(),
        argv.join(" ")
    );
    let _ = write!(err, "Allow it for this project? [y/N] ");
    let _ = err.flush();
    let mut answer = String::new();
    if std::io::stdin().lock().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim(), "y" | "Y" | "yes")
}
