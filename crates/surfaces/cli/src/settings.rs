use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal, Written};
use crate::terminal::{Terminal, errln, outln};
use specforge_ops::extension::{self, SetRequest, SettingChange};
use specforge_ops::view::ProjectView;
use specforge_project::{CompiledProject, SharedRuntime};

// `specforge settings`'s command line: the listing, or `set` and `unset` (ADR 0296).
#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct Args {
    #[command(subcommand)]
    action: Option<Action>,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    output: crate::pick::PickedOutputArg,

    /// List the settings nothing sets and that have no default too
    #[arg(long)]
    all: bool,

    /// List only this extension's settings
    #[arg(long, value_name = "EXTENSION")]
    extension: Option<String>,
}

#[derive(clap::Subcommand)]
enum Action {
    /// Write one setting of specforge.json, admitted by its declared type; never a secret
    Set(SetArgs),
    /// Remove one setting from specforge.json: it reverts to its preset or default
    Unset(UnsetArgs),
}

#[derive(clap::Args)]
struct SetArgs {
    /// The extension, as `specforge settings` lists it
    extension: String,

    /// The setting's name, or a family's key (`models.planning`)
    name: String,

    /// The value; a list takes every remaining word as an item
    #[arg(
        value_name = "VALUE",
        conflicts_with = "json",
        allow_negative_numbers = true
    )]
    value: Vec<String>,

    /// The value as one JSON document (a block, a list of blocks)
    #[arg(long = "value-json", value_name = "JSON")]
    json: Option<String>,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    /// Report what would be written; write nothing
    #[arg(long)]
    dry_run: bool,

    #[command(flatten)]
    output: OutputArg,
}

#[derive(clap::Args)]
struct UnsetArgs {
    /// The extension, as `specforge settings` lists it
    extension: String,

    /// The setting's name, or a family's key (`models.planning`)
    name: String,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    /// Report what would be written; write nothing
    #[arg(long)]
    dry_run: bool,

    #[command(flatten)]
    output: OutputArg,
}

fn print_diagnostic(terminal: &Terminal, diagnostic: &specforge_common::Diagnostic) {
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

/// What a `settings set` or `unset` line names: the project, the format, the setting and whether to only preview.
struct Target<'a> {
    project: &'a crate::project::ProjectArg,
    output: &'a OutputArg,
    extension: &'a str,
    name: &'a str,
    dry_run: bool,
}

/// `settings set` and `unset`: the one write, printed as the setting it left. The project is compiled with
/// `runtime` when given (the tests' in-process extensions), else with the installed ones.
fn write(
    terminal: &Terminal,
    runtime: Option<SharedRuntime>,
    target: Target<'_>,
    change: SettingChange,
) -> Exit {
    let Target {
        project,
        output,
        extension,
        name,
        dry_run,
    } = target;
    let format = output.format;
    let root = match project.root() {
        Ok(root) => root,
        Err(error) => return Refusal::of(format).report(terminal, &error),
    };
    let path = root.dir();
    let compiled = match runtime {
        Some(runtime) => CompiledProject::compile(&root, Some(runtime)),
        None => crate::project::compile_at(&root),
    };
    let request = SetRequest {
        extension,
        name,
        change,
        dry_run,
        caller: crate::caller::of(false, format, terminal),
    };
    let outcome = match extension::set_setting(&ProjectView::of(&compiled), &request) {
        Ok(outcome) => outcome,
        Err(error) => return Refusal::of(format).at(path).report(terminal, &error),
    };
    let document = &outcome.document;
    match format {
        OutputFormat::Json => terminal.json(&Written::of(document, &outcome.wrote, path)),
        OutputFormat::Human => {
            for diagnostic in &document.diagnostics.0 {
                print_diagnostic(terminal, diagnostic);
            }
            let verb = if dry_run { "Would write" } else { "Wrote" };
            match (&document.value, document.changed) {
                (_, false) => outln!(terminal, "{extension} {name}: unchanged"),
                (Some(value), true) => outln!(terminal, "{verb} {extension} {name} = {value}"),
                (None, true) => outln!(
                    terminal,
                    "{} {extension} {name}: back to its preset or default",
                    if dry_run { "Would remove" } else { "Removed" }
                ),
            }
        }
    }
    Exit::Passed
}

impl Args {
    /// `specforge settings`: each loaded extension's settings as `specforge.json` and the presets resolve them
    /// (ADR 0288), then the diagnostics reading them produced. Exit 1 when one is an error.
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        self.run_in(terminal, None)
    }

    /// [`Self::run`] with the extensions in `runtime` when given.
    pub(crate) fn run_in(self, terminal: &Terminal, runtime: Option<SharedRuntime>) -> Exit {
        match self.action {
            Some(Action::Set(a)) => {
                let json = match a
                    .json
                    .as_deref()
                    .map(serde_json::from_str::<serde_json::Value>)
                {
                    Some(Ok(value)) => Some(value),
                    Some(Err(error)) => {
                        return Refusal::of(a.output.format).report(
                            terminal,
                            &specforge_ops::OpError::new(
                                specforge_ops::OpErrorKind::InvalidInput,
                                "invalid_input",
                                format!("--value-json is not JSON: {error}"),
                            ),
                        );
                    }
                    None => None,
                };
                let change = json.map_or_else(
                    || SettingChange::Words(a.value.clone()),
                    SettingChange::Json,
                );
                return write(
                    terminal,
                    runtime,
                    Target {
                        project: &a.project,
                        output: &a.output,
                        extension: &a.extension,
                        name: &a.name,
                        dry_run: a.dry_run,
                    },
                    change,
                );
            }
            Some(Action::Unset(a)) => {
                return write(
                    terminal,
                    runtime,
                    Target {
                        project: &a.project,
                        output: &a.output,
                        extension: &a.extension,
                        name: &a.name,
                        dry_run: a.dry_run,
                    },
                    SettingChange::Unset,
                );
            }
            None => {}
        }
        let format = match self.output.format(terminal) {
            Ok(format) => format,
            Err(exit) => return exit,
        };
        let compiled = match runtime {
            Some(runtime) => self.project.compile_in(runtime),
            None => self.project.compile(),
        };
        let project = match compiled {
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
            OutputFormat::Json => crate::pick::print_json(terminal, self.output.pick(), &listing),
            OutputFormat::Human => {
                for diagnostic in &listing.diagnostics.0 {
                    print_diagnostic(terminal, diagnostic);
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
                        if !setting.gates_when_on.is_empty() {
                            outln!(
                                terminal,
                                "      gates while on {}",
                                setting.gates_when_on.join(", ")
                            );
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
