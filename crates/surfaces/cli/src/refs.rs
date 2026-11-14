use crate::outcome::{Exit, OutputFormat, Refusal};
use crate::pick::PickedOutputArg;
use crate::terminal::{Terminal, outln};
use specforge_ops::read::navigate::{
    DIRECTION, Direction, Navigator, ReferenceLocations, ReferenceQuery, Role,
};
use specforge_ops::view::ProjectView;

// `specforge refs`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Entity ID whose references to list
    entity: String,

    /// Which references: other entities' to it, its own to others, or both
    #[arg(
        long,
        value_parser = crate::options::choice(&DIRECTION),
        default_value = DIRECTION.default_name()
    )]
    direction: Direction,

    /// Also list the entity's own declaration (its name)
    #[arg(long)]
    include_declaration: bool,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    output: PickedOutputArg,
}

impl Args {
    /// `specforge refs <ENTITY>`: the references of an entity (ADR 0355) over the project compiled at `path`, the
    /// document `specforge.find_references` replies. Human output is one line per occurrence, by file and position;
    /// `--format json` the document. An unknown entity is E003 (exit 1).
    pub(crate) fn run(self, terminal: &Terminal, ports: &specforge_effects::Ports) -> Exit {
        let format = match self.output.format(terminal) {
            Ok(format) => format,
            Err(exit) => return exit,
        };
        let project = match self.project.compile(ports) {
            Ok(project) => project,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let query = ReferenceQuery {
            direction: self.direction,
            include_declaration: self.include_declaration,
        };
        let found = match Navigator::new(ProjectView::of(&project))
            .reference_locations(&self.entity, query)
        {
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

/// One line per location, `<file>:<line>:<col>  <holder>.<field> → <target> (<role>)`, the declaration marked
/// `(declaration)`; none says so.
fn print_human(terminal: &Terminal, found: &ReferenceLocations) {
    if found.locations.is_empty() {
        outln!(terminal, "no reference to '{}'", found.entity_id);
    }
    for location in &found.locations {
        let span = &location.source_span;
        let at = format!("{}:{}:{}", span.file, span.start_line, span.start_col);
        let holder = &location.referencing_entity_id;
        match (location.role, &location.field) {
            (Role::Reference, Some(field)) => outln!(
                terminal,
                "{at}  {holder}.{field} → {} (reference)",
                location.referenced_entity_id
            ),
            _ => outln!(terminal, "{at}  {holder} (declaration)"),
        }
    }
}
