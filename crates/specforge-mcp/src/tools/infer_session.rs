use serde_json::{Value, json};

use specforge_ops::OpError;
use specforge_ops::infer::{
    self, EndStatus, Recorded, SessionAction, SessionOutcome, SessionStatus, SessionStep,
};

use crate::args::Arguments;
use crate::mutation::{Mutated, MutationHandled, Written};
use crate::target::Call;
use crate::tool::{ErrorCode, McpError, ToolOutcome};

/// `specforge.infer_session`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Session action to perform
    #[arg(choice = infer::SESSION_ACTION)]
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
    /// Final status (for end)
    #[arg(choice = infer::END_STATUS)]
    status: EndStatus,
}

/// `specforge.infer_session`: one step of an inference session
/// (`specforge_ops::infer::session`), which writes `specforge-infer.json`.
pub fn call(call: &mut Call<'_>, args: Args) -> MutationHandled {
    let project = call.project()?;
    let step = step(&args)?;
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
fn step(args: &Args) -> Result<SessionStep<'_>, Box<McpError>> {
    let action = infer::SESSION_ACTION
        .parse(&args.action)
        .map_err(|error| Box::new(McpError::from(error).with_argument("action")))?;
    Ok(match action {
        SessionAction::Start => SessionStep::Start {
            agent: args.agent.as_deref(),
            source_roots: args.source_roots.as_deref(),
        },
        SessionAction::MarkAnalyzed => SessionStep::MarkAnalyzed {
            source_file: required(&args.source_file, "source_file")?,
            entities: &args.entities_produced,
        },
        SessionAction::End => SessionStep::End {
            session_id: required(&args.session_id, "session_id")?,
            status: args.status,
        },
    })
}

/// The argument `value` holds, or the refusal that it is missing.
fn required<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str, Box<McpError>> {
    value.as_deref().ok_or_else(|| {
        let message = format!("Missing required parameter: {name}");
        Box::new(McpError::new(ErrorCode::InvalidInput, message).with_argument(name))
    })
}

/// The argument an operation refusal is about.
fn argument_of(error: &OpError) -> Option<&'static str> {
    match error.code.as_ref() {
        infer::UNKNOWN_SESSION => Some("session_id"),
        infer::SOURCE_UNREADABLE => Some("source_file"),
        infer::SOURCE_OUTSIDE_ROOT => {
            match error.data.as_ref().and_then(|d| d["argument"].as_str()) {
                Some("source_roots") => Some("source_roots"),
                _ => Some("source_file"),
            }
        }
        _ => None,
    }
}

/// What a recorded step replies.
fn reply(recorded: &Recorded) -> Value {
    match recorded {
        Recorded::Started { session_id } => {
            json!({"session_id": session_id, "status": SessionStatus::Active.name()})
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
