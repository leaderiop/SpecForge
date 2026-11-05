//! `specforge.collect`: record which entities the tests prove (`specforge_ops::analysis::collect`).

use crate::target::{AnyProject, Project};
use crate::tool::{CoreTool, Effect, ReplySpec, ToolGroup, WriteHints};

use crate::args::Arguments;
use crate::refusal::ArgumentOf;
use crate::reply::Answered;
use crate::target::ProjectRef;
use crate::tool::McpError;
use specforge_ops::analysis::check::{CheckField, parse_effects};
use specforge_ops::analysis::collect::CollectField;
use specforge_ops::authoring::apply::apply_collected;

/// `specforge.collect`'s reply: the document `specforge collect --format
/// json` prints (`McpCollectResult`).
type Reply = specforge_ops::analysis::collect::CollectDocument;

/// `specforge.collect`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Collector name (e.g. cargo-test); detected from project files if omitted
    runner: Option<String>,
    /// Run the test command first; it must have been approved with `specforge collect` in a terminal (otherwise the existing report is parsed)
    run: bool,
    /// Plan a writing collector's edits and write none of them (the report is still recorded)
    dry_run: bool,
    /// Whether an http or process effect the collector asks for is performed, reused or replayed (ADR 0337): cached (the default) performs none and answers from the effects journal; replay answers only from it; online and auto perform what the collector's approved capabilities allow
    #[arg(names = &specforge_effects::EffectsMode::NAMES)]
    effects: Option<String>,
}

pub(crate) struct Tool;

impl ArgumentOf<CollectField> for Tool {
    fn argument(field: CollectField) -> Option<&'static str> {
        match field {
            CollectField::Runner => Some("runner"),
        }
    }
}

impl ArgumentOf<CheckField> for Tool {
    fn argument(field: CheckField) -> Option<&'static str> {
        matches!(field, CheckField::Effects).then_some("effects")
    }
}

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.collect";
    const DESCRIPTION: &'static str = "Record which entities the project's tests prove, from the test runner's report (runs the runner only with run: true and prior approval)";
    const EFFECT: Effect = Effect::WritesOutput {
        group: ToolGroup::Management,
        hints: WriteHints {
            destructive: true,
            idempotent: false,
            open_world: true,
        },
    };
    const SPEC: ReplySpec = ReplySpec::Type("McpCollectResult");
    type Args = Args;
    type Input = Project<AnyProject>;
    type Returns = Answered<Reply>;

    fn call(project: ProjectRef<'_>, args: Args) -> Answered<Reply> {
        use specforge_ops::analysis::collect::{self, Mode, Request, RunnerOutput};

        let runner = args.runner.as_deref();
        let run = args.run;
        let mode = args
            .effects
            .as_deref()
            .map(parse_effects)
            .transpose()
            .map_err(Self::refused)?
            .unwrap_or(specforge_effects::EffectsMode::Cached);

        // Tests map to the entities on disk now: the target brought the served
        // project up to date, or compiled the project `path` names for this
        // call, in the runtime its environment was loaded in.
        let mut user = None;
        let consent = project.consent_or_users(&mut user);
        let request = Request {
            runner,
            mode: if run {
                // The server owns stdio: the runner's output is discarded.
                Mode::Run(RunnerOutput::Discard)
            } else {
                Mode::NoRun
            },
            announce: &mut |_, _| {},
            // Every file the collect reads or writes lands inside the call's project, and the
            // server never prompts: a command runs only if the user already approved it for this
            // project with `specforge collect` in a terminal.
            caller: project
                .caller()
                .lending(project.ports, consent, mode)
                .previewing(args.dry_run),
        };
        let view = project.view();
        let mut outcome = collect::collect(&view, request, project.machine, consent)
            .map_err(|refusal| Box::new(Self::refused(refusal)))?;
        // A writing collector's plans are applied after the report was written (ADR 0289).
        let applied = apply_collected(
            &view,
            std::mem::take(&mut outcome.plans),
            std::mem::take(&mut outcome.facts),
            args.dry_run,
            project.caller(),
        )
        .map_err(|error| Box::new(McpError::from(error)))?;
        Ok(outcome.with_applied(applied).document().into())
    }
}
