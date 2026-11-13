use crate::notes::Notes;
use crate::outcome::{Exit, OutputFormat, Refusal};
use crate::pick::PickedOutputArg;
use crate::terminal::{Terminal, outln};
use specforge_ops::read::query::{
    DEFAULT_SEARCH_LIMIT, EntityHits, FieldHolds, SearchRequest, search,
};
use specforge_ops::view::ProjectView;

// `specforge search`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Text to find in an entity's id, title or field text (empty: every entity the filters keep)
    text: String,

    /// Keep only entities of these kinds (comma-separated, or the flag repeated)
    #[arg(long)]
    kinds: Vec<String>,

    /// Keep only entities whose field of this name contains --value
    #[arg(long, value_name = "FIELD", requires = "value")]
    field: Option<String>,

    /// With --field: the text the field contains, ignoring case
    #[arg(long, value_name = "TEXT", requires = "field")]
    value: Option<String>,

    /// Keep only entities that reference this entity
    #[arg(long, value_name = "ENTITY")]
    references: Option<String>,

    /// At most this many entities, best first
    #[arg(long, default_value_t = DEFAULT_SEARCH_LIMIT)]
    limit: usize,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    output: PickedOutputArg,
}

impl Args {
    /// `specforge search <text>`: the search read view (ADR 0355) over the project compiled at `path`, the document
    /// `specforge.search` replies. Human output is one line per entity, best first, with the field text it matched
    /// under it; `--format json` the document. The notices (I020) go to stderr under every format; a refusal (an
    /// unknown `--references` entity, E003) is `error[CODE]` (exit 1).
    pub(crate) fn run(self, terminal: &Terminal, ports: &specforge_effects::Ports) -> Exit {
        let format = match self.output.format(terminal) {
            Ok(format) => format,
            Err(exit) => return exit,
        };
        let project = match self.project.compile(ports) {
            Ok(project) => project,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let view = ProjectView::of(&project);
        // clap requires --field and --value together; the pairing is the operation's all the same.
        let field = match FieldHolds::of(self.field.as_deref(), self.value.as_deref()) {
            Ok(field) => field,
            Err(refusal) => return Refusal::of(format).report(terminal, &refusal),
        };
        let request = SearchRequest {
            text: &self.text,
            kinds: specforge_ops::view::kinds_of_texts(&self.kinds).collect(),
            field,
            referencing: self.references.as_deref(),
            limit: Some(self.limit),
        };
        let hits = match search(&view, &request) {
            Ok(hits) => Notes::new(terminal, format).noticed(hits),
            Err(refusal) => return Refusal::of(format).report(terminal, &refusal),
        };
        match format {
            OutputFormat::Json => self.output.picked().print_json(terminal, &hits),
            OutputFormat::Human => print_human(terminal, &self.text, &hits),
        }
        Exit::Passed
    }
}

/// One line per hit, `<kind> <id>  <title>  <file>:<line>`, and under a field-text match `  <field>: <text>`; no hit
/// says so.
fn print_human(terminal: &Terminal, text: &str, hits: &EntityHits) {
    if hits.results.is_empty() {
        outln!(terminal, "no entity matches '{text}'");
    }
    for hit in &hits.results {
        let title = hit.title.as_deref().unwrap_or_default();
        outln!(
            terminal,
            "{} {}  {title}  {}:{}",
            hit.kind,
            hit.entity_id,
            hit.file_path,
            hit.line
        );
        if let (Some(field), Some(snippet)) = (&hit.match_field, &hit.match_snippet) {
            outln!(terminal, "  {field}: {snippet}");
        }
    }
}
