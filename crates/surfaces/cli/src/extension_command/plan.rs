//! What a writing command's plan became, as the command line prints it (ADR 0289): the command's own output first,
//! then the plan's diff (`--dry-run`) or the files written; a refused plan fails the command with the refusal.

use crate::terminal::{Terminal, err, out, outln};
use serde_json::{Value, json};
use specforge_ops::authoring::apply::{CommandRun, PlanOutcome};
use specforge_ops::extension::command::CommandFormat;
use specforge_ops::{OpError, Wrote};
use specforge_protocol_types::CommandError;
use std::path::Path;

/// Print `run` and return the exit code: the command's own when its plan was applied or previewed, 1 for a plan
/// the host refused. `root` names the files written.
pub(super) fn ended(
    run: CommandRun,
    format: CommandFormat,
    root: &Path,
    terminal: &Terminal,
) -> i32 {
    // A state patch the host refused is reported beside what the run did, and the exit code is not 0.
    let state = run.state_refused.clone();
    let code = printed(run, format, root, terminal);
    match state {
        Some(refusal) => {
            err!(terminal, "{}", refused(&refusal, format));
            code.max(1)
        }
        None => code,
    }
}

fn printed(run: CommandRun, format: CommandFormat, root: &Path, terminal: &Terminal) -> i32 {
    let CommandRun { output, plan, .. } = run;
    let (edited, refusal) = match plan {
        Some(PlanOutcome::Applied(edited) | PlanOutcome::Previewed(edited)) => (Some(edited), None),
        Some(PlanOutcome::Refused(refusal)) => (None, Some(refusal)),
        Some(PlanOutcome::Failed) | None => (None, None),
    };
    if let Some(refusal) = refusal {
        if format == CommandFormat::Human {
            out!(terminal, "{}", output.stdout);
        }
        err!(terminal, "{}", refused(&refusal, format));
        return 1;
    }
    let Some(edited) = edited else {
        out!(terminal, "{}", output.stdout);
        err!(terminal, "{}", output.stderr);
        return output.exit_code;
    };
    match format {
        CommandFormat::Json => {
            let printed = serde_json::from_str(&output.stdout)
                .unwrap_or_else(|_| Value::String(output.stdout.clone()));
            let mut document = json!({"output": printed, "plan": edited.document()});
            if let Some(files) = edited.wrote.named(Some(root)) {
                document["files_written"] = json!(files);
            }
            terminal.json(&document);
        }
        CommandFormat::Human => {
            out!(terminal, "{}", output.stdout);
            let report = &edited.report;
            if !report.diff.is_empty() {
                outln!(terminal, "{}", report.diff.trim_end());
            }
            match &edited.wrote {
                Wrote::Previewed => outln!(
                    terminal,
                    "dry run: {} file(s) would change; nothing was written",
                    report.affected_files.len()
                ),
                wrote => {
                    let files = wrote.named(Some(root)).unwrap_or_default();
                    if files.is_empty() {
                        outln!(terminal, "nothing to write");
                    } else {
                        let files: Vec<&str> = files.iter().map(|f| f.as_str()).collect();
                        outln!(terminal, "wrote: {}", files.join(", "));
                    }
                }
            }
        }
    }
    err!(terminal, "{}", output.stderr);
    output.exit_code
}

/// A refused plan as stderr shows it: `error[code]: message` under `human`, the command's error object on one line
/// under `json`.
fn refused(refusal: &OpError, format: CommandFormat) -> String {
    match format {
        CommandFormat::Human => format!("error[{}]: {}\n", refusal.code, refusal.message),
        CommandFormat::Json => {
            let error = CommandError {
                suggestion: refusal.suggestion.clone(),
                ..CommandError::new(refusal.code.to_string(), refusal.message.clone())
            };
            format!(
                "{}\n",
                serde_json::to_string(&error).expect("a command error serializes")
            )
        }
    }
}
