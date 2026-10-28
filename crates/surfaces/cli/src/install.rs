//! `specforge install <host>…` and `specforge uninstall <host>…` (ADR 0292): the project's contributions rendered
//! into each host's files, idempotently, all or nothing; uninstall removes exactly what install marked. The adapters
//! (`specforge-hosts`) know each host's formats; the operations (`specforge_ops::host::install`) plan and write.

use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal};
use crate::project::ProjectArg;
use crate::terminal::{Terminal, errln, outln};
use specforge_ops::host::install::{self, Options, Outcome};
use specforge_ops::host::{Host, HostRenderer};
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;

/// `specforge install`'s command line.
#[derive(clap::Args)]
pub(crate) struct InstallArgs {
    /// The hosts to install into: claude-code, codex or cursor
    #[arg(required = true, num_args = 1..)]
    hosts: Vec<String>,

    /// Print the plan and write nothing
    #[arg(long)]
    dry_run: bool,

    /// Print the plan, write nothing, and exit 1 when anything would change (for CI)
    #[arg(long)]
    check: bool,

    /// Rewrite a file install wrote that was edited since (never a file it did not write)
    #[arg(long)]
    force: bool,

    #[command(flatten)]
    project: ProjectArg,

    #[command(flatten)]
    output: OutputArg,
}

/// `specforge uninstall`'s command line.
#[derive(clap::Args)]
pub(crate) struct UninstallArgs {
    /// The hosts to remove install's files from: claude-code, codex or cursor
    #[arg(required = true, num_args = 1..)]
    hosts: Vec<String>,

    /// Print the plan and write nothing
    #[arg(long)]
    dry_run: bool,

    /// Remove a file install wrote that was edited since (never a file it did not write)
    #[arg(long)]
    force: bool,

    #[command(flatten)]
    project: ProjectArg,

    #[command(flatten)]
    output: OutputArg,
}

impl InstallArgs {
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let project = match self.project.compile() {
            Ok(project) => project,
            Err(error) => return Refusal::fixed().report(terminal, &error),
        };
        self.run_in(&project, terminal)
    }

    /// Over `project`, already compiled.
    pub(crate) fn run_in(self, project: &CompiledProject, terminal: &Terminal) -> Exit {
        go(
            project,
            &self.hosts,
            false,
            Options {
                force: self.force,
                dry_run: self.dry_run || self.check,
            },
            self.check,
            self.output.format,
            terminal,
        )
    }
}

impl UninstallArgs {
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let project = match self.project.compile() {
            Ok(project) => project,
            Err(error) => return Refusal::fixed().report(terminal, &error),
        };
        self.run_in(&project, terminal)
    }

    pub(crate) fn run_in(self, project: &CompiledProject, terminal: &Terminal) -> Exit {
        go(
            project,
            &self.hosts,
            true,
            Options {
                force: self.force,
                dry_run: self.dry_run,
            },
            false,
            self.output.format,
            terminal,
        )
    }
}

/// The adapters of the hosts the words name, each once.
fn renderers(words: &[String]) -> Result<Vec<Box<dyn HostRenderer>>, String> {
    let mut out: Vec<Box<dyn HostRenderer>> = Vec::new();
    for word in words {
        let host = Host::from_flag(word)
            .ok_or_else(|| format!("unknown host '{word}': claude-code, codex or cursor"))?;
        if !out.iter().any(|r| r.host() == host) {
            out.extend(specforge_hosts::renderer(host));
        }
    }
    Ok(out)
}

fn go(
    project: &CompiledProject,
    hosts: &[String],
    remove: bool,
    options: Options,
    check: bool,
    format: OutputFormat,
    terminal: &Terminal,
) -> Exit {
    let adapters = match renderers(hosts) {
        Ok(adapters) => adapters,
        Err(message) => {
            errln!(terminal, "error: {message}");
            return Exit::Unjudged;
        }
    };
    let view = ProjectView::of(project);
    let list: Vec<&dyn HostRenderer> = adapters.iter().map(AsRef::as_ref).collect();
    let caller = crate::caller::of(false, format, terminal);
    let outcome = if remove {
        install::uninstall(&view, &list, options, caller)
    } else {
        install::install(&view, &list, options, caller)
    };
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => return Refusal::of(format).report(terminal, &error),
    };
    match format {
        OutputFormat::Json => terminal.json(&outcome.document()),
        OutputFormat::Human => print(&outcome, remove, terminal),
    }
    Exit::of_verdict(!outcome.refused() && !(check && outcome.changes()))
}

fn print(outcome: &Outcome, remove: bool, terminal: &Terminal) {
    for plan in &outcome.plans {
        outln!(terminal, "{}", plan.host.as_str());
        for file in &plan.files {
            outln!(terminal, "  {:<9} {}", file.action.as_str(), file.path);
        }
        if plan.files.is_empty() {
            outln!(terminal, "  nothing to do");
        }
        for item in &plan.not_rendered {
            outln!(terminal, "  not rendered: {item}");
        }
        for refused in &plan.refused {
            errln!(
                terminal,
                "error[{}]: {}",
                refused.code.as_str(),
                refused.message
            );
        }
    }
    if outcome.refused() {
        errln!(terminal, "nothing was written");
    } else if outcome.wrote.previewed() {
        outln!(terminal, "dry run: nothing was written");
    } else if outcome.changes() {
        outln!(
            terminal,
            "{}",
            if remove { "uninstalled" } else { "installed" }
        );
    }
}
