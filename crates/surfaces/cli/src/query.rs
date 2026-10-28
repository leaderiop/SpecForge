use crate::notes::Notes;
use crate::options;
use crate::outcome::{Exit, Refusal};
use crate::terminal::{Terminal, outln};
use specforge_ops::read::export::Format;
use specforge_ops::read::judged::{InputsRequest, judged_inputs};
use specforge_ops::read::query::{QueryRequest, query};
use specforge_ops::view::ProjectView;
use specforge_protocol_types::Cite;

// `specforge query`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Entity ID to query
    entity: String,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    /// Number of hops from the entity (0 = entity only)
    #[arg(long, default_value_t = specforge_ops::read::query::DEFAULT_DEPTH)]
    depth: usize,

    /// Keep only entities of these kinds (comma-separated, or the flag repeated); the queried
    /// entity always stays
    #[arg(long)]
    kinds: Vec<String>,

    /// Output detail level
    #[arg(
        long,
        value_parser = options::choice(&specforge_ops::read::export::AGENT_FORMAT),
        default_value = specforge_ops::read::export::AGENT_FORMAT.default_name()
    )]
    format: Format,

    /// Give every entity its coverage status
    #[arg(long, conflicts_with = "inputs")]
    include_coverage: bool,

    /// Instead of the neighbourhood, list what this judged rule reads for the entity: each input
    /// with its digest, the verdict's state, and the basis `set_verdict` checks
    #[arg(long, value_name = "RULE", conflicts_with_all = ["depth", "kinds"])]
    inputs: Option<String>,

    /// With --inputs: an entity the verdict will cite (the flag repeated)
    #[arg(long, requires = "inputs", value_name = "ENTITY")]
    cite: Vec<String>,

    /// With --inputs: a file the verdict will cite, from the project root
    #[arg(long, requires = "inputs", value_name = "FILE")]
    cite_file: Vec<String>,

    /// With --inputs: a test the verdict will cite, by name
    #[arg(long, requires = "inputs", value_name = "TEST")]
    cite_test: Vec<String>,
}

impl Args {
    /// The query the flags ask for.
    fn request(&self) -> QueryRequest<'_> {
        QueryRequest {
            entity_id: &self.entity,
            depth: Some(self.depth),
            kinds: specforge_ops::view::kinds_of_texts(&self.kinds).collect(),
            format: Some(self.format),
            include_coverage: self.include_coverage,
        }
    }

    /// `specforge query <entity>`: the query read view over the project
    /// compiled at `path`. The document on stdout; the notices (I020) on
    /// stderr; a refusal as `error[CODE]` with the closest entity as its
    /// hint (exit 1).
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let project = match self.project.compile() {
            Ok(project) => project,
            Err(error) => return Refusal::fixed().report(terminal, &error),
        };
        if let Some(rule) = &self.inputs {
            return self.run_inputs(terminal, &ProjectView::of(&project), rule);
        }
        match query(&ProjectView::of(&project), &self.request()) {
            Ok(document) => {
                let document = Notes::fixed(terminal).noticed(document);
                outln!(terminal, "{}", document);
                Exit::Passed
            }
            Err(error) => Refusal::fixed().report(terminal, &error),
        }
    }

    /// `--inputs <rule>`: the whole answer as JSON (the `graph` format, the default); with `--format context` or
    /// `brief`, one line per input, then what is cited, then the basis as one JSON line.
    fn run_inputs(&self, terminal: &Terminal, view: &ProjectView<'_>, rule: &str) -> Exit {
        let cited: Vec<Cite> = self
            .cite
            .iter()
            .map(|entity| Cite::Entity {
                entity: entity.clone(),
            })
            .chain(
                self.cite_file
                    .iter()
                    .map(|file| Cite::File { file: file.clone() }),
            )
            .chain(
                self.cite_test
                    .iter()
                    .map(|test| Cite::Test { test: test.clone() }),
            )
            .collect();
        let request = InputsRequest {
            entity: &self.entity,
            rule,
            cited: &cited,
        };
        let answer = match judged_inputs(view, &request) {
            Ok(answer) => answer,
            Err(error) => return Refusal::fixed().report(terminal, &error),
        };
        if self.format == Format::Graph {
            outln!(
                terminal,
                "{}",
                serde_json::to_string_pretty(&answer).expect("serializes")
            );
            return Exit::Passed;
        }
        // no verdict state: the rule's setting is off, or the entity is exempted by `unless_set`
        let state = match (&answer.verdict, answer.gated) {
            (Some(verdict), _) => verdict.state.as_str(),
            (None, true) => "gated",
            (None, false) => "exempt",
        };
        outln!(
            terminal,
            "{} {} ({}): {state}",
            answer.entity,
            answer.rule,
            answer.judge
        );
        for input in &answer.inputs {
            let moved = if input.changed { " changed" } else { "" };
            outln!(
                terminal,
                "  {} [{}] {} {}{moved}  {}",
                input.name,
                input.kind,
                input.members.len(),
                input.selector,
                input.digest
            );
        }
        outln!(terminal, "  cited: {}", answer.cited.digest);
        outln!(
            terminal,
            "basis {}",
            serde_json::to_string(&answer.basis).expect("serializes")
        );
        Exit::Passed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Line {
        #[command(flatten)]
        args: Args,
    }

    #[test]
    fn the_kinds_flag_is_a_kinds_list() {
        let line =
            Line::try_parse_from(["query", "x", "--kinds", "behavior, type", "--kinds", "adr"])
                .unwrap();
        assert_eq!(line.args.request().kinds, ["behavior", "type", "adr"]);
    }

    /// `query --inputs` on an exempted entity (`unless_set`), as the CLI prints it.
    fn inputs_of(format: &str) -> String {
        use crate::terminal::Facts;
        use specforge_project::testing::{gsd_judged_project, gsd_judged_runtime};
        let project = gsd_judged_project(
            "\nkey_link KL \"Link\" {\n  plan 03-02\n  from \"a\"\n  to \"b\"\n  pattern \"x\"\n}\n",
        );
        let compiled = project.compile_in(std::sync::Arc::new(gsd_judged_runtime()));
        let line = Line::try_parse_from([
            "query",
            "v1.0/03/03-02/KL",
            "--inputs",
            "gsd/E501",
            "--format",
            format,
        ])
        .unwrap();
        let (terminal, capture) = Terminal::captured("", Facts::PIPED);
        let exit = line
            .args
            .run_inputs(&terminal, &ProjectView::of(&compiled), "gsd/E501");
        assert!(matches!(exit, Exit::Passed));
        capture.stdout()
    }

    #[test]
    fn inputs_print_json_in_the_graph_format() {
        let text = inputs_of("graph");
        let answer: serde_json::Value = serde_json::from_str(&text).expect(&text);
        assert_eq!(answer["rule"], "gsd/E501");
    }

    #[test]
    fn an_exempted_entity_is_not_gated() {
        let text = inputs_of("brief");
        assert!(text.contains(": exempt"), "{text}");
        assert!(!text.contains("gated"), "{text}");
    }
}
