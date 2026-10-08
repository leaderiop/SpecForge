//! `specforge.collect`: record which entities the tests prove (`specforge_ops::collect`).

use specforge_common::codes;

use crate::args::Arguments;
use crate::target::ProjectRef;
use crate::tool::{McpError, ToolOutcome};

/// `specforge.collect`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Collector name (e.g. cargo-test); detected from project files if omitted
    runner: Option<String>,
    /// Run the test command first; it must have been approved with `specforge collect` in a terminal (otherwise the existing report is parsed)
    run: bool,
}

pub(crate) fn call(project: &ProjectRef<'_>, args: Args) -> ToolOutcome {
    use specforge_ops::collect::{self, Consent, Mode, Request, RunnerOutput};

    let runner = args.runner.as_deref().filter(|r| *r != "auto");
    let run = args.run;

    // Tests map to the entities on disk now: the target brought the served
    // project up to date, or compiled the project `path` names for this
    // call, in the runtime its environment was loaded in.
    let request = Request {
        runner,
        mode: if run {
            // The server owns stdio: the runner's output is discarded.
            Mode::Run(RunnerOutput::Discard)
        } else {
            Mode::NoRun
        },
        // The server never prompts: a command runs only if the user already
        // approved it for this project with `specforge collect` in a terminal.
        consent: Consent::Approved,
        announce: &mut |_, _| {},
    };
    match collect::collect(&project.view(), request) {
        Ok(outcome) => ToolOutcome::ok(outcome.to_json()),
        Err(mut e) => {
            if e.is(codes::E059) {
                e.message = format!(
                    "the test command isn't approved for this project; run `specforge collect` \
                     in a terminal once to approve it ({})",
                    e.message
                );
            }
            McpError::from(e).into()
        }
    }
}
