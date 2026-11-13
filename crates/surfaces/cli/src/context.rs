use crate::options;
use crate::outcome::{Exit, OutputFormat, Refusal};
use crate::pick::Picked;
use crate::terminal::{Terminal, err, out, outln};
use specforge_ops::authoring::apply::{self, Origin, RECORD_REFUSED};
use specforge_ops::read::context::{self, ContextRequest, PACK_FORMAT};
use specforge_ops::read::export::Format;
use specforge_ops::view::ProjectView;
use specforge_ops::{OpError, OpErrorKind};
use specforge_protocol_types::{EditPlan, PlanOp};

// `specforge context`'s command line.
#[derive(clap::Args)]
#[command(mut_arg(crate::pick::PICK, |arg| arg.long_help(crate::pick::CONTEXT_LONG_HELP)))]
pub(crate) struct Args {
    /// The context route to read: its qualified name, `<extension>/<route>`
    route: String,

    /// The entity to read it for: one of the route's subject kind
    subject: String,

    /// Token budget for the pack (default: the route's own). The subject and the core sections are never cut, and a
    /// budget below them fails (E062); enrich sections are served whole in declared order, the first that does not fit
    /// keeps its most connected entities that fit, and the pack lists what was cut under `budget`
    #[arg(long)]
    max_tokens: Option<usize>,

    /// Format of the pack's graph
    #[arg(
        long,
        value_parser = options::choice(&PACK_FORMAT),
        default_value = PACK_FORMAT.default_name()
    )]
    format: Format,

    /// Record the pack on ENTITY, a kind the route records whose subject is <SUBJECT>: the host writes the route, the
    /// pack's digest and its budget, with its ledger line, then prints the pack it recorded (in the context format,
    /// which a record is fitted in)
    #[arg(long, value_name = "ENTITY", conflicts_with = "format")]
    record: Option<String>,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    picked: Picked,
}

impl Args {
    /// `specforge context <route> <subject>`: the pack of a context route for one entity (ADR 0356) over the project
    /// compiled at `path`, the document `specforge.context` answers, on stdout as compact JSON. A refusal is the error
    /// document on stdout (exit 1).
    pub(crate) fn run(self, terminal: &Terminal, ports: &specforge_effects::Ports) -> Exit {
        let project = match self.project.compile(ports) {
            Ok(project) => project,
            Err(error) => return Refusal::of(OutputFormat::Json).report(terminal, &error),
        };
        self.answer(terminal, &ProjectView::of(&project))
    }

    fn answer(&self, terminal: &Terminal, view: &ProjectView) -> Exit {
        let request = ContextRequest {
            route: &self.route,
            subject: &self.subject,
            max_tokens: self.max_tokens,
            format: self.format,
        };
        let pack = match context::context(view, &request) {
            Ok(pack) => pack,
            Err(refusal) => {
                // With --record, a pack read for the record fails as the record does in an edit plan.
                let error = match self.record {
                    Some(_) => apply::record_read_error(1, refusal.into_error()),
                    None => refusal.into_error(),
                };
                return Refusal::of(OutputFormat::Json).report(terminal, &error);
            }
        };
        if let Some(entity) = &self.record
            && let Err(error) = self.record(terminal, view, entity, &pack.subject)
        {
            return Refusal::of(OutputFormat::Json).report(terminal, &error);
        }
        match self.picked.printed(&pack.text) {
            Some(Ok(lines)) => out!(terminal, "{lines}"),
            Some(Err(refused)) => {
                err!(terminal, "{refused}");
                return Exit::Failed;
            }
            None => outln!(terminal, "{}", pack.text),
        }
        Exit::Passed
    }

    /// Record the route's pack on `entity`, whose record must derive `subject` (by path), as a core operation.
    fn record(
        &self,
        terminal: &Terminal,
        view: &ProjectView,
        entity: &str,
        subject: &str,
    ) -> Result<(), OpError> {
        let derived = apply::recorded(view, 1, &self.route, entity)?.subject(view, 1)?;
        if derived != subject {
            return Err(OpError::new(
                OpErrorKind::InvalidInput,
                RECORD_REFUSED,
                format!(
                    "the route '{}' reads '{derived}' for '{entity}', not '{subject}'",
                    self.route
                ),
            )
            .with_data(serde_json::json!({"reason": "subject_mismatch"})));
        }
        let plan = EditPlan {
            ops: vec![PlanOp::RecordContext {
                entity: entity.to_string(),
                route: self.route.clone(),
                max_tokens: self
                    .max_tokens
                    .map(|n| u32::try_from(n).unwrap_or(u32::MAX)),
            }],
            title: None,
        };
        apply::apply_plan(
            view,
            &plan,
            &Origin::cli(terminal.facts().interactive),
            false,
            crate::caller::of(false, OutputFormat::Json, terminal),
        )
        .map(|_| ())
        .map_err(|refusal| refusal.into_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::Facts;
    use clap::Parser;
    use specforge_project::SharedRuntime;
    use specforge_project::testing::{CONTEXT_PLAN, gsd_context_project, gsd_context_runtime};

    #[derive(Parser)]
    struct Line {
        #[command(flatten)]
        args: Args,
    }

    fn args(words: &[&str]) -> Args {
        Line::try_parse_from(std::iter::once("context").chain(words.iter().copied()))
            .expect("the words parse")
            .args
    }

    #[specforge_test_macros::test(
        behavior = "serve_context_pack",
        verify = "specforge context prints a route's pack and --pick reads its digest"
    )]
    fn specforge_context_prints_the_pack_and_pick_reads_its_digest() {
        let project = gsd_context_project();
        let compiled =
            project.compile_in(std::sync::Arc::new(gsd_context_runtime()) as SharedRuntime);
        let view = ProjectView::of(&compiled);

        let (terminal, printed) = Terminal::captured("", Facts::PIPED);
        let exit = args(&["gsd/executor", CONTEXT_PLAN]).answer(&terminal, &view);
        assert_eq!(exit, Exit::Passed);
        let pack: serde_json::Value =
            serde_json::from_str(printed.stdout().trim_end()).expect("the pack is JSON");
        assert_eq!(pack["route"], "gsd/executor");
        assert_eq!(pack["subject"], CONTEXT_PLAN);
        let digest = pack["digest"].as_str().expect("a digest");
        assert!(digest.starts_with("sha256:"), "{digest}");

        let (terminal, picked) = Terminal::captured("", Facts::PIPED);
        let exit =
            args(&["gsd/executor", CONTEXT_PLAN, "--pick", ".digest"]).answer(&terminal, &view);
        assert_eq!(exit, Exit::Passed);
        assert_eq!(picked.stdout(), format!("{digest}\n"));

        let (terminal, refused) = Terminal::captured("", Facts::PIPED);
        let exit = args(&["gsd/executor", "v1.0/02"]).answer(&terminal, &view);
        assert_eq!(exit, Exit::Failed);
        assert!(
            refused.stdout().contains("route_of_another_kind"),
            "{}",
            refused.stdout()
        );
    }

    #[specforge_test_macros::test(
        behavior = "record_context_pack",
        verify = "specforge context --record writes the digest it prints"
    )]
    fn specforge_context_record_writes_the_digest_it_prints() {
        let project = gsd_context_project();
        let compiled =
            project.compile_in(std::sync::Arc::new(gsd_context_runtime()) as SharedRuntime);
        let view = ProjectView::of(&compiled);
        // The formatter aligns a block's values: runs of spaces read as one.
        let spec = || {
            let text = std::fs::read_to_string(project.root().join("a.spec")).expect("the spec");
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        };

        // Another subject than the one the record derives is refused, and nothing is written.
        let before = spec();
        let (terminal, refused) = Terminal::captured("", Facts::PIPED);
        let exit = args(&["gsd/executor", "v1.0/01/01-01", "--record", CONTEXT_PLAN])
            .answer(&terminal, &view);
        assert_eq!(exit, Exit::Failed);
        assert!(
            refused.stdout().contains("record_refused")
                && refused.stdout().contains("not 'v1.0/01/01-01'"),
            "{}",
            refused.stdout()
        );
        assert_eq!(spec(), before);

        let (terminal, printed) = Terminal::captured("", Facts::PIPED);
        let exit = args(&[
            "gsd/executor",
            CONTEXT_PLAN,
            "--record",
            CONTEXT_PLAN,
            "--max-tokens",
            "5000",
            "--pick",
            ".digest",
        ])
        .answer(&terminal, &view);
        assert_eq!(exit, Exit::Passed, "{}", printed.stdout());
        let digest = printed.stdout().trim_end().to_string();
        assert!(digest.starts_with("sha256:"), "{digest}");
        let written = spec();
        assert!(
            written.contains(&format!("digest \"{digest}\"")),
            "{written}"
        );
        assert!(written.contains("max_tokens 5000"), "{written}");
    }

    #[specforge_test_macros::test(
        behavior = "record_context_pack",
        verify = "specforge context --record below the core sections is refused as the record, with nothing written"
    )]
    fn a_record_below_the_core_sections_is_refused_as_the_record() {
        let project = gsd_context_project();
        let compiled =
            project.compile_in(std::sync::Arc::new(gsd_context_runtime()) as SharedRuntime);
        let view = ProjectView::of(&compiled);
        let before = std::fs::read_to_string(project.root().join("a.spec")).expect("the spec");
        let words = ["gsd/executor", CONTEXT_PLAN, "--max-tokens", "1"];

        let (terminal, read) = Terminal::captured("", Facts::PIPED);
        assert_eq!(args(&words).answer(&terminal, &view), Exit::Failed);
        assert!(read.stdout().contains("E062"), "{}", read.stdout());

        let (terminal, refused) = Terminal::captured("", Facts::PIPED);
        let recording = [&words[..], &["--record", CONTEXT_PLAN]].concat();
        assert_eq!(args(&recording).answer(&terminal, &view), Exit::Failed);
        let error: serde_json::Value =
            serde_json::from_str(refused.stdout().trim_end()).expect("the refusal is JSON");
        assert_eq!(error["code"], RECORD_REFUSED, "{error}");
        let message = error["error"].as_str().expect("a message");
        assert!(message.contains("cannot hold route"), "{message}");
        assert_eq!(
            std::fs::read_to_string(project.root().join("a.spec")).expect("the spec"),
            before
        );
    }

    #[test]
    fn record_and_format_conflict() {
        let parsed = Line::try_parse_from([
            "context",
            "gsd/executor",
            CONTEXT_PLAN,
            "--record",
            CONTEXT_PLAN,
            "--format",
            "graph",
        ]);
        let error = parsed.err().expect("--record and --format conflict");
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}
