//! Running a writing command (ADR 0289): the command answers, and the plan it answered is applied for it.

use super::flags::agent_facing;
use super::{
    Clock, HUMAN_ONLY, HostJudgedRules, JudgedRules, Origin, Surface, SystemClock,
    UNDECLARED_WRITE, apply_plan_with,
};
use crate::authoring::edit::Edited;
use crate::extension::command::{CommandFormat, ExtensionCommand, RunError, called, settle};
use crate::host::Host;
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
    /// The refusal of the state patch the command answered (over the cap, not an object): the run, and the files its
    /// plan wrote, stand; only the state is unchanged (ADR 0022 D4: an error never drops the writes).
    pub state_refused: Option<OpError>,
}

/// Run `command` over the project `view` describes ([`run`]) and apply the plan it answered, unless `dry_run`. A plan
/// from a command that does not declare `writes` is refused (`undeclared_write`), and one from a call that failed
/// is not applied.
#[allow(
    clippy::too_many_arguments,
    reason = "the command, its inputs, the calling host, how it is applied and who applies it are each one thing"
)]
pub fn execute(
    view: &ProjectView<'_>,
    command: &ExtensionCommand,
    given: &Map<String, Value>,
    format: CommandFormat,
    host: Option<Host>,
    dry_run: bool,
    caller: Caller<'_>,
    surface: Surface,
) -> Result<CommandRun, RunError> {
    let rules: Arc<dyn JudgedRules> = Arc::new(HostJudgedRules::of(view));
    execute_with(
        view,
        command,
        given,
        format,
        host,
        dry_run,
        caller,
        surface,
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
    host: Option<Host>,
    dry_run: bool,
    caller: Caller<'_>,
    surface: Surface,
    rules: &Arc<dyn JudgedRules>,
    clock: &dyn Clock,
) -> Result<CommandRun, RunError> {
    // A command that records what a person says is not run for an agent, `--dry-run` included (ADR 0301).
    if command.human() && agent_facing(&Origin::Core { surface }) {
        let refused = OpError::new(
            OpErrorKind::PermissionDenied,
            HUMAN_ONLY,
            format!(
                "`{}` records what a person says: a person runs it at a terminal, not an agent's call",
                command.tool_name()
            ),
        )
        .with_suggestion(format!(
            "ask the person to run `specforge {} {}` in their terminal",
            command.short(),
            command.cli_name()
        ));
        return Err(RunError::Gate(Box::new(refused)));
    }
    let mut output = called(view, command, given, format, host)?;
    let Some(plan) = &output.plan else {
        let state_refused = if dry_run {
            None
        } else {
            keep_state(view, command, &mut output)
        };
        output.state = None;
        return Ok(CommandRun {
            output,
            plan: None,
            state_refused,
        });
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
    // The state is kept after the plan, and only when the plan did not fail or get refused.
    let state_refused =
        if dry_run || matches!(outcome, PlanOutcome::Refused(_) | PlanOutcome::Failed) {
            None
        } else {
            keep_state(view, command, &mut output)
        };
    output.state = None;
    Ok(CommandRun {
        output,
        plan: Some(outcome),
        state_refused,
    })
}

/// [`settle`], its refusal returned rather than raised.
fn keep_state(
    view: &ProjectView<'_>,
    command: &ExtensionCommand,
    output: &mut CommandOutput,
) -> Option<OpError> {
    match settle(view, command, output) {
        Err(RunError::State(refusal)) => Some(*refusal),
        _ => None,
    }
}
