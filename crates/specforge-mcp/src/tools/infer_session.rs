use serde_json::{Value, json};

use specforge_ops::OpError;
use specforge_ops::infer::{self, EndStatus, Recorded, SessionOutcome, SessionStep};

use crate::args::Arguments;
use crate::mutation::{Mutated, MutationHandled, Written};
use crate::target::Call;
use crate::tool::{McpError, ToolOutcome};

/// The actions a session takes, in the order the listing states them.
const ACTIONS: &[&str] = &["start", "mark_analyzed", "end"];

/// The states a session ends in.
const END_STATUSES: &[&str] = &["completed", "paused"];

/// `specforge.infer_session`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Session action to perform
    #[arg(names = ACTIONS)]
    action: String,
    /// Agent identifier (for start)
    agent: Option<String>,
    /// Source directories to scan (for start)
    source_roots: Option<Vec<String>>,
    /// Relative path to analyzed file (for mark_analyzed)
    source_file: Option<String>,
    /// Entity IDs produced from the file (for mark_analyzed)
    entities_produced: Vec<String>,
    /// Session ID to end (for end)
    session_id: Option<String>,
    /// Final status (for end, default: completed)
    #[arg(names = END_STATUSES)]
    status: Option<String>,
}

/// `specforge.infer_session`: one step of an inference session
/// (`specforge_ops::infer::session`), which writes `specforge-infer.json`.
pub fn call(call: &mut Call<'_>, args: Args) -> MutationHandled {
    let project = call.project()?;
    let step = match step(&args) {
        Ok(step) => step,
        Err(refused) => return Ok(Mutated::refused(refused)),
    };
    Ok(match infer::session(&project.view(), step) {
        Ok(SessionOutcome { recorded, writes }) => {
            let written = match &recorded {
                Recorded::Marked { entities, .. } => {
                    Written::files(writes).with_entities(entities.clone())
                }
                _ => Written::files(writes),
            };
            Mutated::wrote(ToolOutcome::ok(reply(&recorded)), written)
        }
        Err(error) => {
            let argument = argument_of(&error);
            let refused = McpError::from(error);
            Mutated::refused(match argument {
                Some(argument) => refused.with_argument(argument),
                None => refused,
            })
        }
    })
}

/// The step `args` asks for, or the refusal of the arguments.
fn step(args: &Args) -> Result<SessionStep<'_>, ToolOutcome> {
    Ok(match args.action.as_str() {
        "start" => SessionStep::Start {
            agent: args.agent.as_deref(),
            source_roots: args.source_roots.as_deref(),
        },
        "mark_analyzed" => SessionStep::MarkAnalyzed {
            source_file: required(&args.source_file, "source_file")?,
            entities: &args.entities_produced,
        },
        "end" => SessionStep::End {
            session_id: required(&args.session_id, "session_id")?,
            status: match args.status.as_deref().unwrap_or("completed") {
                "completed" => EndStatus::Completed,
                "paused" => EndStatus::Paused,
                other => {
                    let expected = END_STATUSES.join(", ");
                    return Err(ToolOutcome::invalid_input(
                        "status",
                        format!("Invalid status: '{other}'. Expected: {expected}"),
                    ));
                }
            },
        },
        other => {
            let expected = ACTIONS.join(", ");
            return Err(ToolOutcome::invalid_input(
                "action",
                format!("Unknown action: '{other}'. Expected: {expected}"),
            ));
        }
    })
}

/// The argument `value` holds, or the refusal that it is missing.
fn required<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str, ToolOutcome> {
    value.as_deref().ok_or_else(|| {
        ToolOutcome::invalid_input(name, format!("Missing required parameter: {name}"))
    })
}

/// The argument an operation refusal is about.
fn argument_of(error: &OpError) -> Option<&'static str> {
    match error.code.as_ref() {
        infer::UNKNOWN_SESSION => Some("session_id"),
        infer::SOURCE_UNREADABLE => Some("source_file"),
        _ => None,
    }
}

/// What a recorded step replies.
fn reply(recorded: &Recorded) -> Value {
    match recorded {
        Recorded::Started { session_id } => {
            json!({"session_id": session_id, "status": "active"})
        }
        Recorded::Marked {
            source_file,
            entities,
        } => json!({
            "source_file": source_file,
            "entities_produced": entities,
            "status": "recorded",
        }),
        Recorded::Ended { session_id, status } => {
            json!({"session_id": session_id, "status": status.name()})
        }
    }
}
