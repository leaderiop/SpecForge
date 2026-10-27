//! Running a writing command (ADR 0289): the command answers, and the plan it answered is applied for it.

use super::{
    Clock, JudgedRules, NoJudgedRules, Origin, SystemClock, UNDECLARED_WRITE, apply_plan_with,
};
use crate::authoring::edit::Edited;
use crate::extension::command::{CommandFormat, ExtensionCommand, RunError, run};
use crate::view::ProjectView;
use crate::{Caller, OpError, OpErrorKind};
use serde_json::{Map, Value};
use specforge_protocol_types::CommandOutput;
use std::sync::Arc;

/// What became of the plan a command answered.
#[derive(Debug, Clone)]
pub enum PlanOutcome {
    /// Written: the report, and the files it wrote.
    Applied(Edited),
    /// Planned and judged, and not written (`--dry-run`).
    Previewed(Edited),
    /// Refused, with nothing written: the error names why (`invalid_plan`, `stale_revision`, `E080`, ...).
    Refused(OpError),
    /// The command failed (a nonzero exit code): its plan is not applied.
    Failed,
}

/// What a command did: its output, and what became of its plan when it answered one.
#[derive(Debug, Clone)]
pub struct CommandRun {
    pub output: CommandOutput,
    pub plan: Option<PlanOutcome>,
}

/// Run `command` over the project `view` describes ([`run`]) and apply the plan it answered, unless `dry_run`. A plan
/// from a command that does not declare `writes` is refused (`undeclared_write`), and one from a call that failed
/// is not applied.
pub fn execute(
    view: &ProjectView<'_>,
    command: &ExtensionCommand,
    given: &Map<String, Value>,
    format: CommandFormat,
    dry_run: bool,
    caller: Caller<'_>,
) -> Result<CommandRun, RunError> {
    let rules: Arc<dyn JudgedRules> = Arc::new(NoJudgedRules);
    execute_with(
        view,
        command,
        given,
        format,
        dry_run,
        caller,
        &rules,
        &SystemClock,
    )
}

/// [`execute`] for a host that judges rules: a `set_verdict` in the plan is checked through `rules` and dated by `clock`.
#[allow(
    clippy::too_many_arguments,
    reason = "the command, its inputs, how it is applied and the judged-rules port are each one thing"
)]
pub fn execute_with(
    view: &ProjectView<'_>,
    command: &ExtensionCommand,
    given: &Map<String, Value>,
    format: CommandFormat,
    dry_run: bool,
    caller: Caller<'_>,
    rules: &Arc<dyn JudgedRules>,
    clock: &dyn Clock,
) -> Result<CommandRun, RunError> {
    let output = run(view, command, given, format)?;
    let Some(plan) = &output.plan else {
        return Ok(CommandRun { output, plan: None });
    };
    let outcome = if !command.writes() {
        PlanOutcome::Refused(OpError::new(
            OpErrorKind::PermissionDenied,
            UNDECLARED_WRITE,
            format!(
                "{} {} answered a plan but does not declare `writes`; nothing was written",
                command.short(),
                command.id()
            ),
        ))
    } else if output.exit_code != 0 {
        PlanOutcome::Failed
    } else {
        let origin = Origin::Command {
            extension: command.extension().to_string(),
        };
        match apply_plan_with(view, plan, &origin, dry_run, caller, rules, clock) {
            Ok(edited) if edited.wrote.previewed() => PlanOutcome::Previewed(edited),
            Ok(edited) => PlanOutcome::Applied(edited),
            Err(refusal) => PlanOutcome::Refused(refusal.into_error()),
        }
    };
    Ok(CommandRun {
        output,
        plan: Some(outcome),
    })
}
