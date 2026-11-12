use crate::notes::Notes;
use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal};
use crate::terminal::{Terminal, outln};
use specforge_ops::read::navigate::{anchors_of_entity, anchors_of_file};
use specforge_ops::view::ProjectView;

// `specforge anchors`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Entity ID whose anchors to list
    #[arg(required_unless_present = "file", conflicts_with = "file")]
    entity: Option<String>,

    /// List the anchors of the located facts of this source file (a directory lists the files under it; a trailing
    /// part of a path matches it)
    #[arg(long, value_name = "PATH")]
    file: Option<String>,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    output: OutputArg,
}

impl Args {
    /// `specforge anchors <ENTITY> | --file <PATH>`: the anchors read view (ADR 0349) over the project compiled at
    /// `path`, the documents `specforge.find_implementation` and `specforge.find_spec_for_source` reply. Human output
    /// is one line per anchor; `--format json` the reply document. The notices (I212, W198) go to stderr under every
    /// format; a refusal (an unknown entity, E003; a path outside the project) is `error[CODE]` (exit 1).
    pub(crate) fn run(self, terminal: &Terminal, ports: &specforge_effects::Ports) -> Exit {
        let format = self.output.format;
        let project = match self.project.compile(ports) {
            Ok(project) => project,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let view = ProjectView::of(&project);
        let notes = Notes::new(terminal, format);
        match (&self.entity, &self.file) {
            (_, Some(file)) => match anchors_of_file(&view, file) {
                Ok(found) => {
                    let found = notes.noticed(found);
                    match format {
                        OutputFormat::Json => terminal.json(&found),
                        OutputFormat::Human => {
                            for row in &found.entities {
                                let anchor = &row.anchor;
                                let at = (anchor.line, anchor.end_line);
                                let fact = [
                                    &anchor.file,
                                    &anchor.item_kind,
                                    &anchor.fact_id,
                                    &anchor.via,
                                ];
                                let (kind, id) = (&row.kind, &row.entity_id);
                                outln!(terminal, "{}  [{kind} {id}]", line(fact, at));
                            }
                            none(terminal, found.count, file);
                        }
                    }
                    Exit::Passed
                }
                Err(error) => Refusal::of(format).report(terminal, &error),
            },
            (Some(entity), None) => match anchors_of_entity(&view, entity) {
                Ok(found) => {
                    let found = notes.noticed(found);
                    match format {
                        OutputFormat::Json => terminal.json(&found),
                        OutputFormat::Human => {
                            for row in &found.implementations {
                                let at = (row.line, row.end_line);
                                let fact = [&row.file, &row.item_kind, &row.fact_id, &row.via];
                                outln!(terminal, "{}", line(fact, at));
                            }
                            none(terminal, found.count, &found.entity_id);
                        }
                    }
                    Exit::Passed
                }
                Err(error) => Refusal::of(format).report(terminal, &error),
            },
            // clap requires one of the two.
            (None, None) => Exit::Failed,
        }
    }
}

/// `file:line-end_line  <item_kind> <fact_id>  via <via>`, of a row's `[file, item_kind, fact_id, via]` at `(line,
/// end_line)`.
fn line([file, item_kind, fact_id, via]: [&String; 4], (line, end_line): (usize, usize)) -> String {
    format!("{file}:{line}-{end_line}  {item_kind} {fact_id}  via {via}")
}

/// The line that says an answer is empty.
fn none(terminal: &Terminal, count: usize, asked: &str) {
    if count == 0 {
        outln!(terminal, "no anchor for '{asked}'");
    }
}
