use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal};
use crate::terminal::{Terminal, errln, outln};
use specforge_ops::extension;
use specforge_ops::view::ProjectView;

// `specforge settings`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    output: OutputArg,

    /// List the settings nothing sets and that have no default too
    #[arg(long)]
    all: bool,

    /// List only this extension's settings
    #[arg(long, value_name = "EXTENSION")]
    extension: Option<String>,
}

impl Args {
    /// `specforge settings`: each loaded extension's settings as `specforge.json` and the presets resolve them
    /// (ADR 0288), then the diagnostics reading them produced. Exit 1 when one is an error.
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let format = self.output.format;
        let project = match self.project.compile() {
            Ok(project) => project,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let listing = extension::settings(
            &ProjectView::of(&project),
            self.all,
            self.extension.as_deref(),
        );

        // A setting `specforge.json` gets wrong (E086, E087) fails the run, as `check` does: CI may run this as a
        // check. A W166 does not.
        let exit = Exit::of_verdict(
            !listing
                .diagnostics
                .0
                .iter()
                .any(|d| d.severity == specforge_common::Severity::Error),
        );
        match format {
            OutputFormat::Json => {
                terminal.json(&listing);
            }
            OutputFormat::Human => {
                for diagnostic in &listing.diagnostics.0 {
                    let severity = match diagnostic.severity {
                        specforge_common::Severity::Error => "error",
                        specforge_common::Severity::Warning => "warning",
                        specforge_common::Severity::Info => "info",
                    };
                    errln!(
                        terminal,
                        "{severity}[{}]: {}",
                        diagnostic.code,
                        diagnostic.message
                    );
                }
                if listing.extensions.is_empty() {
                    outln!(terminal, "No settings.");
                    return exit;
                }
                for row in &listing.extensions {
                    outln!(terminal, "{}", row.extension);
                    for setting in &row.settings {
                        let value = setting
                            .value
                            .as_ref()
                            .map_or_else(|| "(unset)".to_string(), ToString::to_string);
                        let from = match &setting.preset {
                            Some(preset) => format!("preset {preset}"),
                            None => setting.origin.clone(),
                        };
                        outln!(
                            terminal,
                            "  {} = {value}  [{}, {from}]",
                            setting.name,
                            setting.field_type
                        );
                        if !setting.gates.is_empty() {
                            outln!(terminal, "      gates {}", setting.gates.join(", "));
                        }
                    }
                    for secret in &row.secrets {
                        let state = if secret.set { "set" } else { "not set" };
                        outln!(
                            terminal,
                            "  secret {} ({}): {state}",
                            secret.name,
                            secret.env
                        );
                    }
                }
                if !listing.gated.is_empty() {
                    outln!(terminal);
                    outln!(
                        terminal,
                        "Reported by no source: {}",
                        listing.gated.join(", ")
                    );
                }
                for raised in &listing.raised {
                    outln!(terminal, "Raised to {}: {}", raised.severity, raised.code);
                }
            }
        }

        exit
    }
}
