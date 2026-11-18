use crate::outcome::{Exit, OutputFormat, Refusal};
use crate::pick::PickedOutputArg;
use crate::terminal::{Terminal, outln};
use specforge_ops::read::diff::{DiffAnswer, DiffRange, DiffRequest, diff};
use specforge_ops::view::ProjectView;

// `specforge diff`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// What to compare: none is HEAD against the working tree, <REV> that revision against it, <A>..<B> two
    /// revisions
    #[arg(value_parser = parse_range)]
    range: Option<String>,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    output: PickedOutputArg,
}

/// `range` as written, once the one range parser takes it: `<a>...<b>` is a usage error (exit 2).
fn parse_range(range: &str) -> Result<String, String> {
    DiffRange::parse(Some(range))
        .map(|_| range.to_string())
        .map_err(|error| error.message)
}

impl Args {
    /// `specforge diff [<from>[..<to>]]`: the nodes and edges that changed between two snapshots of the project
    /// compiled at `path` (ADR 0376 D5). Human output is one line per node and per edge, then the notes; `--format
    /// json` the document. It exits 0 whether or not anything changed, 1 when a revision cannot be read.
    pub(crate) fn run(self, terminal: &Terminal, ports: &specforge_effects::Ports) -> Exit {
        let format = match self.output.format(terminal) {
            Ok(format) => format,
            Err(exit) => return exit,
        };
        let project = match self.project.compile(ports) {
            Ok(project) => project,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let request = DiffRequest { range: self.range };
        let found = match diff(&ProjectView::of(&project), &request) {
            Ok(found) => found,
            Err(refusal) => return Refusal::of(format).report(terminal, &refusal),
        };
        match format {
            OutputFormat::Json => self.output.picked().print_json(terminal, &found),
            OutputFormat::Human => print_human(terminal, &found),
        }
        Exit::Passed
    }
}

/// `+ kind id` for an added node, `- kind id` for a removed one, `~ kind id  fields: ..  aspects: ..` for a modified
/// one, then `+ source -label-> target` (or `-`) for each edge, then each note; `no change` when nothing changed.
fn print_human(terminal: &Terminal, found: &DiffAnswer) {
    if found.nodes.is_empty() && found.edges.is_empty() {
        outln!(terminal, "no change");
    }
    for node in &found.nodes {
        let mark = match node.change.as_str() {
            "added" => "+",
            "removed" => "-",
            _ => "~",
        };
        let mut line = format!("{mark} {} {}", node.kind, node.id);
        if !node.fields.is_empty() {
            line.push_str(&format!("  fields: {}", node.fields.join(", ")));
        }
        if !node.aspects.is_empty() {
            line.push_str(&format!("  aspects: {}", node.aspects.join(", ")));
        }
        outln!(terminal, "{line}");
    }
    for edge in &found.edges {
        let mark = if edge.change == "added" { "+" } else { "-" };
        outln!(
            terminal,
            "{mark} {} -{}-> {}",
            edge.source,
            edge.label,
            edge.target
        );
    }
    for note in &found.notes {
        outln!(terminal, "note ({}): {}", note.side, note.message);
    }
}
