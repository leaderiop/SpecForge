//! `specforge explain <CODE>`: print a code the code book has ([`specforge_diagnostics`]): a
//! catalogued diagnostic code, or one a project's loaded extension catalogs (ADR 0286).

use std::sync::Arc;

use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal};
use crate::project::ProjectArg;
use crate::terminal::{Terminal, outln};
use specforge_component::ComponentRuntime;
use specforge_diagnostics::{WRAP_WIDTH, wrap};
use specforge_ops::read::explain::{Entry, Explanation, explain};
use specforge_project::Environment;

// `specforge explain`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Diagnostic code (e.g., E001, W010), any case
    code: String,

    #[command(flatten)]
    project: ProjectArg,

    #[command(flatten)]
    output: OutputArg,
}

impl Args {
    /// Print the explanation of a diagnostic code: the entry as text, or under `--format json` the
    /// document `specforge.explain` answers. An extension's code is asked under its
    /// namespace and answered from the extension's catalog; a code the code book neither has nor
    /// retired is refused. A loose root (no `specforge.json`) loads no extension: the catalog
    /// alone answers. The
    /// environment's own diagnostics are not printed: explain reports a code, not the project.
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let format = self.output.format;
        let root = match self.project.root() {
            Ok(root) => root,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let env = Environment::load(&root, Some(Arc::new(ComponentRuntime::with_user_cache())));
        match explain(&self.code, env.registries().codes()) {
            Ok(explanation) => {
                match format {
                    OutputFormat::Json => terminal.json(&explanation),
                    OutputFormat::Human => print(&explanation, terminal),
                }
                Exit::Passed
            }
            Err(error) => Refusal::of(format).report(terminal, &error),
        }
    }
}

/// The explanation as text.
fn print(explanation: &Explanation, terminal: &Terminal) {
    match explanation {
        Explanation::Catalogued(found) => print_entry(&found.entry, terminal),
        Explanation::Retired(gone) => match &gone.replaced_by {
            Some(entry) => {
                outln!(
                    terminal,
                    "{} is retired; it was renumbered to {}.\n",
                    gone.code,
                    entry.code
                );
                print_entry(entry, terminal);
            }
            None => outln!(terminal, "{} is retired and no longer emitted.", gone.code),
        },
    }
}

/// The entry, its code bold only where stdout is coloured, its docs link last.
fn print_entry(entry: &Entry, terminal: &Terminal) {
    if terminal.facts().color_out {
        outln!(terminal, "\x1b[1m{}\x1b[0m: {}\n", entry.code, entry.title);
    } else {
        outln!(terminal, "{}: {}\n", entry.code, entry.title);
    }
    outln!(terminal, "{}", wrap(&entry.explanation, WRAP_WIDTH));
    outln!(terminal, "\nOwner: {}", entry.owner);
    outln!(terminal, "Level: {}", entry.level);
    if let Some(docs) = &entry.docs {
        outln!(terminal, "Docs: {docs}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::Facts;

    fn args(code: &str) -> Args {
        Args {
            code: code.into(),
            project: ProjectArg::at("."),
            output: OutputArg {
                format: OutputFormat::Human,
            },
        }
    }

    #[specforge_test_macros::test(
        behavior = "print_diagnostics_structured",
        verify = "specforge explain bolds the code only where its output is coloured"
    )]
    fn explain_bolds_the_code_only_where_stdout_is_coloured() {
        let coloured = Facts {
            color_out: true,
            ..Facts::PIPED
        };
        let (terminal, capture) = Terminal::captured("", coloured);
        let exit = args("e001").run(&terminal);
        assert_eq!(exit, Exit::Passed);
        assert!(
            capture
                .stdout()
                .starts_with("\x1b[1mE001\x1b[0m: Parse error\n"),
            "{:?}",
            capture.stdout()
        );

        let (terminal, capture) = Terminal::captured("", Facts::PIPED);
        let exit = args("e001").run(&terminal);
        assert_eq!(exit, Exit::Passed);
        assert!(
            capture.stdout().starts_with("E001: Parse error\n"),
            "{:?}",
            capture.stdout()
        );
    }
}
