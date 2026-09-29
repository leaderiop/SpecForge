//! `specforge collect` — run the project's test runners and record which
//! entities their tests prove in `specforge-report.json` (ADR 0002).
//!
//! Runners are extensions (`@specforge/cargo-test`, …). Each declares a
//! collector: the files that select it, the command that runs it, where
//! the report lands, and a pure export that maps the report to entities.
//! This command runs the declared command after the user approves it for
//! the project, then hands the report to the extension. `--no-run` and
//! `--report` skip running and parse an existing report instead. The flow
//! itself lives in [`specforge_emitter::collect`], shared with MCP.

use crate::OutputFormat;
use specforge_common::find_project_root;
use specforge_emitter::collect::{self, Collector, Mode, Request, RunnerOutput};
use std::collections::HashSet;
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
        return report_error(msg, "E045", format);
    };

    let (ctx, runtime) = crate::pipeline::compile_with_runtime(&root);
    let known_ids: HashSet<String> = ctx
        .graph
        .nodes()
        .iter()
        .map(|n| n.id.raw.to_string())
        .collect();

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
    let request = Request {
        root: &root,
        runner: options.runner,
        mode,
    };
    let store = collect::consent_path();
    let mut approve = |collector: &Collector, argv: &[String]| {
        approved(collector, argv, &root, options.yes, &store, format)
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
    let outcome = match collect::collect(
        &request,
        &ctx.manifests,
        &runtime,
        &known_ids,
        &mut approve,
        &mut announce,
    ) {
        Ok(outcome) => outcome,
        Err(e) => return report_error(&e.message, e.code, format),
    };

    if format == OutputFormat::Json {
        let output = serde_json::json!({
            "status": "collected",
            "runners": outcome.runners,
            "diagnostics": outcome.diagnostics,
            "report": outcome.report.display().to_string(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("serialize JSON output")
        );
    } else {
        for r in &outcome.runners {
            let s = &r.stats;
            println!(
                "{}: {} entities, {} passed, {} failed, {} skipped ({} report file(s))",
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

/// Whether the collector's command may run: approved before, approved with
/// `--yes`, or approved now at an interactive prompt (and remembered).
fn approved(
    collector: &Collector,
    argv: &[String],
    root: &Path,
    yes: bool,
    store: &Path,
    format: OutputFormat,
) -> bool {
    if yes || collect::is_approved(store, collector, root) {
        return true;
    }
    if format == OutputFormat::Json || !std::io::stdin().is_terminal() {
        return false;
    }
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
    if !matches!(answer.trim(), "y" | "Y" | "yes") {
        return false;
    }
    if let Err(e) = collect::approve(store, collector, root) {
        eprintln!("warning: could not remember the approval: {e}");
    }
    true
}

fn report_error(msg: &str, code: &str, format: OutputFormat) -> i32 {
    if format == OutputFormat::Json {
        let output = serde_json::json!({
            "error": msg,
            "code": code,
            "exit_code": 1,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("serialize JSON output")
        );
    } else {
        eprintln!("error[{code}]: {msg}");
    }
    1
}
