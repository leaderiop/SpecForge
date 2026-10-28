//! A writing command's tool is a mutation (ADR 0289): the command runs, the host applies the plan it answered, and the
//! reply is `{output, plan, files_written}` (`dry_run` plans without writing). It goes through the mutation pipeline,
//! so the served project is brought up to date and `mcp_mutation_completed` is recorded.

use serde::Serialize;
use serde_json::Value;
use specforge_common::shape::Shape;
use specforge_ops::authoring::apply::{CommandRun, PlanOutcome, execute};
use specforge_ops::authoring::edit::EditDocument;
use specforge_ops::extension::command::{CommandFormat, ExtensionCommand, RunError};
use specforge_ops::{Writes, Wrote};

use super::{elapsed_ms, failed_call, failed_run, refused_args};
use crate::events::McpEvent;
use crate::mutation::{self, Failed, Mutated, Mutation, MutationReply, NoDiagnostics, Written};
use crate::surface_table::ToolEntry;
use crate::target::{Call, ProjectRef};
use crate::tool::{ErrorCode, McpError, ToolOutcome};

/// The `dry_run` argument a writing command's tool takes beside the command's own.
pub(crate) const DRY_RUN: &str = "dry_run";

/// What a writing command's tool answers beside `files_written`.
#[derive(Debug, Serialize, Shape)]
pub(crate) struct CommandReply {
    /// The command's stdout: JSON when it printed JSON, else its text.
    output: Value,
    /// The plan the command answered, as `specforge.edit` reports an edit; absent when it answered none.
    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<EditDocument>,
}

impl MutationReply for CommandReply {
    type After = NoDiagnostics;
}

/// The tool's output schema.
pub(crate) fn output_schema() -> &'static Value {
    mutation::output_schema::<CommandReply>()
}

/// Run the writing command `command` of `entry` over the call's project and complete it as a mutation.
pub(super) fn run(
    call: &mut Call<'_>,
    entry: &ToolEntry,
    command: &ExtensionCommand,
    arguments: &Value,
) -> ToolOutcome {
    let mut given = arguments.as_object().cloned().unwrap_or_default();
    let dry_run = match given.remove(DRY_RUN) {
        None => false,
        Some(Value::Bool(dry_run)) => dry_run,
        Some(_) => {
            let error = McpError::new(ErrorCode::InvalidInput, "`dry_run` must be a boolean")
                .with_argument(DRY_RUN);
            return mutation::refused(&entry.name, false, error, call.events());
        }
    };
    let (mutation, dispatched) = match call.project() {
        Ok(project) => attempt(&project, command, &given, dry_run),
        Err(refused) => return mutation::refused(&entry.name, dry_run, refused, call.events()),
    };
    if let Some(event) = dispatched {
        call.events().record(event);
    }
    mutation::complete(call, &entry.name, output_schema(), dry_run, mutation)
}

/// The command run and its plan applied (or only planned), as a mutation.
fn attempt(
    project: &ProjectRef<'_>,
    command: &ExtensionCommand,
    given: &serde_json::Map<String, Value>,
    dry_run: bool,
) -> (Mutation<CommandReply>, Option<McpEvent>) {
    let started = std::time::Instant::now();
    let run = match execute(
        &project.view(),
        command,
        given,
        CommandFormat::Json,
        dry_run,
        project.caller(),
    ) {
        Ok(run) => run,
        Err(RunError::Args(refused)) => return (Err(refused_args(&refused).into()), None),
        Err(RunError::Call(error)) => return (Err(failed_call(&error.diagnostic()).into()), None),
        Err(RunError::NoProject(error)) => return (Err(McpError::from(error).into()), None),
    };
    let event = McpEvent::SurfaceCommandDispatched {
        extension_name: command.extension().to_string(),
        command_id: command.id().to_string(),
        exit_code: run.output.exit_code,
        duration_ms: elapsed_ms(started),
    };
    (replied(command, run, dry_run), Some(event))
}

fn replied(command: &ExtensionCommand, run: CommandRun, dry_run: bool) -> Mutation<CommandReply> {
    let CommandRun { output, plan } = run;
    if output.exit_code != 0 {
        return Err(Failed::from(failed_run(command, output)));
    }
    let printed = serde_json::from_str(&output.stdout).unwrap_or(Value::String(output.stdout));
    match plan {
        None => Ok(Mutated::new(
            CommandReply {
                output: printed,
                plan: None,
            },
            Written::of(Wrote::of(dry_run, Writes::none())),
        )),
        Some(PlanOutcome::Applied(edited) | PlanOutcome::Previewed(edited)) => Ok(Mutated::new(
            CommandReply {
                output: printed,
                plan: Some(edited.document()),
            },
            Written::of(edited.wrote.clone()).with_entities(edited.report.entities.iter().cloned()),
        )),
        Some(PlanOutcome::Refused(refusal)) => Err(Failed::from(refusal)),
        Some(PlanOutcome::Failed) => Err(Failed::from(McpError::new(
            ErrorCode::InternalError,
            "the command failed and its plan was not applied",
        ))),
    }
}
