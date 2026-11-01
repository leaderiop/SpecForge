use crate::notes::Notes;
use crate::options;
use crate::outcome::{Exit, Refusal};
use crate::pick::Pick;
use crate::terminal::{Terminal, err, errln, out, outln};
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

    #[arg(long, value_name = "PATH", value_parser = Pick::parse, help = crate::pick::HELP, long_help = crate::pick::QUERY_LONG_HELP)]
    pick: Option<Pick>,
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
        // checked here, not by clap's `conflicts_with`: `--format` has a default, and `--format graph` is taken
        if self.pick.is_some()
            && (self.format != Format::Graph || self.inputs.is_some() || self.include_coverage)
        {
            errln!(
                terminal,
                "error: --pick reads the graph format's JSON: it takes no --format context or brief, \
                 --inputs or --include-coverage"
            );
            return Exit::Unjudged;
        }
        let project = match self.project.compile() {
            Ok(project) => project,
            Err(error) => return Refusal::fixed().report(terminal, &error),
        };
        self.answer(terminal, &ProjectView::of(&project))
    }

    /// The answer over `view`: the neighbourhood, or `--inputs`'.
    fn answer(&self, terminal: &Terminal, view: &ProjectView<'_>) -> Exit {
        if let Some(rule) = &self.inputs {
            return self.run_inputs(terminal, view, rule);
        }
        match query(view, &self.request()) {
            Ok(document) => {
                let document = Notes::fixed(terminal).noticed(document);
                match &self.pick {
                    Some(pick) => match pick.printed(&document.to_string()) {
                        Ok(lines) => out!(terminal, "{lines}"),
                        Err(refused) => {
                            err!(terminal, "{refused}");
                            return Exit::Failed;
                        }
                    },
                    None => outln!(terminal, "{}", document),
                }
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

    /// `specforge query <argv>` over the judged fixture: the exit, stdout and stderr.
    fn queried(argv: &[&str]) -> (Exit, String, String) {
        use crate::terminal::Facts;
        use specforge_project::testing::{gsd_judged_project, gsd_judged_runtime};
        let compiled = gsd_judged_project("").compile_in(std::sync::Arc::new(gsd_judged_runtime()));
        let line = Line::try_parse_from(std::iter::once("query").chain(argv.iter().copied()))
            .unwrap_or_else(|e| panic!("{e}"));
        let (terminal, capture) = Terminal::captured("", Facts::PIPED);
        let exit = line.args.answer(&terminal, &ProjectView::of(&compiled));
        (exit, capture.stdout(), capture.stderr())
    }

    #[test]
    fn pick_prints_one_line_per_value_reached() {
        let (exit, out, _) = queried(&["v1.0/03", "--kinds", "plan", "--pick", ".nodes[].id"]);
        assert!(matches!(exit, Exit::Passed));
        assert_eq!(out, "v1.0/03\nv1.0/03/03-02\nv1.0/03/03-03\n");
        let (_, out, _) = queried(&[
            "v1.0/03",
            "--kinds",
            "plan",
            "--pick",
            ".nodes[kind=plan].id",
        ]);
        assert_eq!(
            out, "v1.0/03/03-02\nv1.0/03/03-03\n",
            "the phase is left out"
        );
        let (_, out, _) = queried(&["v1.0/03", "--depth", "0", "--pick", ".nodes[0].kind"]);
        assert_eq!(out, "phase\n", "a string prints bare");
    }

    #[test]
    fn a_missing_path_prints_nothing_and_passes() {
        let (exit, out, _) = queried(&["v1.0/03", "--pick", ".nodes[0].nope"]);
        assert!(matches!(exit, Exit::Passed));
        assert_eq!(out, "");
    }

    #[test]
    fn pick_takes_the_graph_format_and_its_json_alias() {
        let (exit, out, _) = queried(&[
            "v1.0/03",
            "--depth",
            "0",
            "--format",
            "json",
            "--pick",
            ".nodes[0].id",
        ]);
        assert!(matches!(exit, Exit::Passed));
        assert_eq!(out, "v1.0/03\n");
        let (_, out, _) = queried(&[
            "v1.0/03",
            "--depth",
            "0",
            "--format",
            "graph",
            "--pick",
            ".nodes[0].id",
        ]);
        assert_eq!(out, "v1.0/03\n");
    }

    #[test]
    fn pick_refuses_a_format_that_is_not_json() {
        use crate::terminal::Facts;
        for (format, extra) in [
            ("brief", None),
            ("context", None),
            ("graph", Some("--include-coverage")),
        ] {
            let mut argv = vec!["query", "v1.0/03", "--format", format, "--pick", ".nodes"];
            argv.extend(extra);
            let line = Line::try_parse_from(argv).unwrap_or_else(|e| panic!("{e}"));
            let (terminal, capture) = Terminal::captured("", Facts::PIPED);
            // refused before the project is compiled: `--path` is `.`, which is not the fixture
            let exit = line.args.run(&terminal);
            assert_eq!(exit.code(), 2, "{format}");
            assert!(capture.stdout().is_empty());
            assert!(capture.stderr().contains("--pick"), "{}", capture.stderr());
        }
    }

    #[test]
    fn a_malformed_path_is_a_usage_error_naming_its_byte() {
        let error = match Line::try_parse_from(["query", "x", "--pick", ".nodes[kind=plan"]) {
            Ok(_) => panic!("a malformed path parses"),
            Err(error) => error,
        };
        assert_eq!(error.exit_code(), 2);
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::ValueValidation,
            "{error}"
        );
        assert!(error.to_string().contains("byte 6"), "{error}");
    }

    #[test]
    fn an_exempted_entity_is_not_gated() {
        let text = inputs_of("brief");
        assert!(text.contains(": exempt"), "{text}");
        assert!(!text.contains("gated"), "{text}");
    }
}
