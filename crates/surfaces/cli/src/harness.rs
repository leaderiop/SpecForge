//! `specforge harness gc [--dry-run]`: write the harness ledger canonically (ADR 0287). The ledger is kept true by
//! every host write; this drops the lines of what the project no longer holds and puts the rest in order.

use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal, Written};
use crate::project::ProjectArg;
use crate::terminal::{Terminal, outln};
use specforge_ops::authoring::apply::Origin;
use specforge_ops::authoring::harness;
use specforge_ops::view::ProjectView;

#[derive(clap::Subcommand)]
pub(crate) enum Action {
    /// Write specforge-harness canonically: drop the lines of what the project no longer holds, sort the rest
    Gc(GcArgs),
}

#[derive(clap::Args)]
pub(crate) struct GcArgs {
    /// Print the diff and write nothing
    #[arg(long)]
    dry_run: bool,

    #[command(flatten)]
    project: ProjectArg,

    #[command(flatten)]
    output: OutputArg,
}

impl Action {
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let Action::Gc(args) = self;
        let format = args.output.format;
        let compiled = match args.project.compile() {
            Ok(compiled) => compiled,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let view = ProjectView::of(&compiled);
        let root = match view.project_root() {
            Ok(root) => root,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let collected = match harness::run(
            &view,
            args.dry_run,
            &Origin::cli(terminal.facts().interactive),
            crate::caller::of(false, format, terminal),
        ) {
            Ok(collected) => collected,
            Err(error) => return Refusal::of(format).at(root).report(terminal, &error),
        };
        match format {
            OutputFormat::Json => {
                terminal.json(&Written::of(collected.document(), &collected.wrote, root));
            }
            OutputFormat::Human => {
                let diff = collected.change.diff();
                if !diff.is_empty() {
                    outln!(terminal, "{}", diff.trim_end());
                }
                match (collected.wrote.previewed(), diff.is_empty()) {
                    (_, true) => outln!(terminal, "nothing to write"),
                    (true, false) => outln!(terminal, "dry run: nothing was written"),
                    (false, false) => outln!(terminal, "wrote specforge-harness"),
                }
            }
        }
        Exit::Passed
    }
}
