use crate::notes::Notes;
use crate::outcome::{Exit, OutputFormat, Refusal};
use crate::terminal::{Terminal, out};
use specforge_ops::read::plan::{AgentPlan, PlanOutcome};
use specforge_ops::read::trace::{
    Ask, Partial, RouteTrace, Trace, TraceChain, TraceLink, TraceOutcome,
};
use specforge_ops::view::ProjectView;

// `specforge trace`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Entity ID to trace (omit to trace every entity)
    entity: Option<String>,

    /// An agent plan file (an AgentPlan's JSON) to check against the graph instead of tracing
    #[arg(long, value_name = "FILE", value_parser = plan_file, conflicts_with = "entity")]
    plan: Option<AgentPlan>,

    /// Follow this route (its qualified name, `<extension>/<route>`); with no entity, trace every entity of its
    /// start kind
    #[arg(long, value_name = "NAME", conflicts_with_all = ["plan", "reach"])]
    route: Option<String>,

    /// Trace the entity's reach both ways, whatever route starts at its kind
    #[arg(long, conflicts_with = "plan")]
    reach: bool,

    /// Print a route's thread even when it is incomplete, with a `no <step>` marker where it falls short
    #[arg(long, conflicts_with_all = ["plan", "reach"])]
    partial: bool,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    /// Output format: human or json (default: human for a route's thread, json otherwise)
    #[arg(long)]
    format: Option<OutputFormat>,
}

/// The AgentPlan the file at `path` holds: clap's refusal (exit 2) when it
/// cannot be read or is not one.
fn plan_file(path: &str) -> Result<AgentPlan, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    AgentPlan::parse(&text).map_err(|error| error.message("plan"))
}

impl Args {
    /// `specforge trace [entity]`: the trace operation over the project
    /// compiled at `path`: one entity's trace, or every entity's when none
    /// is named. An entity of a kind a loaded extension declares a route
    /// from is traced along it (`--route` names one, `--reach` asks for the
    /// chain instead); any other by its chain, whose expected edges come
    /// from the loaded extensions' registries and are reported missing
    /// when lacking. With `--plan`, the plan check of an agent plan file
    /// instead (exit 1 when the plan fails).
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let project = match self.project.compile() {
            Ok(project) => project,
            Err(error) => return Refusal::of(self.asked_or_json()).report(terminal, &error),
        };
        let view = ProjectView::of(&project);
        // clap refuses an entity with --plan first; the reading is the operation's all the same.
        let (target, follow) = match Ask::of(
            self.entity.as_deref(),
            self.plan.as_ref(),
            self.route.as_deref(),
            self.reach,
        ) {
            Err(refusal) => {
                return Refusal::of(self.asked_or_json()).report(terminal, &refusal.into_error());
            }
            Ok(Ask::Plan(plan)) => {
                let outcome = specforge_ops::read::plan::check(&view, plan);
                match self.asked_or_json() {
                    OutputFormat::Human => out!(terminal, "{}", plan_text(&outcome)),
                    OutputFormat::Json => terminal.json(&outcome),
                }
                return Exit::of_verdict(outcome.ok);
            }
            Ok(Ask::Trace(target, follow)) => (target, follow),
        };
        let traced = match specforge_ops::read::trace::trace(&view, target, follow) {
            Ok(traced) => traced,
            Err(refusal) => return Refusal::of(self.asked_or_json()).report(terminal, &refusal),
        };

        // A read's notices are written under every format; the outcome then decides the document's.
        let outcome = Notes::new(terminal, self.asked_or_json()).noticed(traced);
        match self.format_of(&outcome) {
            OutputFormat::Human => out!(terminal, "{}", trace_text(&outcome, self.partial())),
            OutputFormat::Json => terminal.json(&outcome),
        }
        Exit::Passed
    }
}

impl Args {
    /// The format asked for, else json, `trace`'s default since before routes: what prints before any trace (a
    /// refusal, a plan's check).
    fn asked_or_json(&self) -> OutputFormat {
        self.format.unwrap_or(OutputFormat::Json)
    }

    /// The format `outcome` prints in: the one asked for, else human for a route's thread (one routed entity, or every
    /// entity of a route `--route` names) and json for any other trace (ADR 0330 D7, amending ADR 0029 D5b).
    fn format_of(&self, outcome: &TraceOutcome) -> OutputFormat {
        let thread = match outcome {
            TraceOutcome::One(document) => matches!(document.trace, Trace::Route(_)),
            TraceOutcome::Every(_) => self.route.is_some(),
        };
        self.format.unwrap_or(if thread {
            OutputFormat::Human
        } else {
            OutputFormat::Json
        })
    }

    /// Whether a route's incomplete thread prints (`--partial`).
    fn partial(&self) -> Partial {
        if self.partial {
            Partial::Yes
        } else {
            Partial::No
        }
    }
}

/// The plan check as `specforge trace --plan --format human` prints it.
fn plan_text(outcome: &PlanOutcome) -> String {
    let mut text = String::new();
    if outcome.resolved_entries.is_empty() {
        text.push_str("resolved: none\n");
    } else {
        text.push_str(&format!(
            "resolved: {}\n",
            outcome.resolved_entries.join(", ")
        ));
    }
    if outcome.gaps.is_empty() {
        text.push_str("gaps: none\n");
    } else {
        text.push_str(&format!("gaps ({}):\n", outcome.gaps.len()));
        for gap in &outcome.gaps {
            text.push_str(&format!("  - {}: {}\n", gap.kind.as_str(), gap.context));
        }
    }
    text.push_str(if outcome.ok {
        "plan check: passed\n"
    } else {
        "plan check: failed\n"
    });
    text
}

/// The traces as terminal text, one block per trace, separated by a blank line.
fn trace_text(outcome: &TraceOutcome, partial: Partial) -> String {
    outcome
        .traces()
        .iter()
        .map(|trace| match trace {
            Trace::Chain(chain) => chain_text(chain),
            Trace::Route(route) => route_text(route, partial),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A route's thread as terminal text: the thread `partial` prints, else the line that says it is incomplete.
fn route_text(route: &RouteTrace, partial: Partial) -> String {
    let thread = route
        .thread(partial)
        .unwrap_or_else(|| route.incomplete_line());
    format!("{thread}\n")
}

/// A chain as terminal text: the entity, then its upstream, downstream and missing links, one per line. Missing
/// links are marked `MISSING`.
fn chain_text(chain: &TraceChain) -> String {
    let mut out = format!("{} [{}]\n", chain.entity_id, chain.entity_kind);
    let section = |out: &mut String, name: &str, links: &[TraceLink], arrow: fn(&str) -> String| {
        out.push_str(&format!("  {name}:\n"));
        if links.is_empty() {
            out.push_str("    (none)\n");
        }
        for link in links {
            out.push_str(&format!(
                "    {} {} [{}] (depth {})\n",
                arrow(&link.edge_label),
                link.entity_id,
                link.entity_kind,
                link.depth
            ));
        }
    };
    section(&mut out, "upstream", &chain.upstream, |l| format!("<-{l}-"));
    section(&mut out, "downstream", &chain.downstream, |l| {
        format!("-{l}->")
    });
    if !chain.missing.is_empty() {
        out.push_str("  missing:\n");
        for link in &chain.missing {
            out.push_str(&format!(
                "    MISSING {} -{}-> [{}]{} (depth {})\n",
                link.from,
                link.edge_label,
                link.expected_kind,
                if link.required { " required" } else { "" },
                link.depth
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_ops::read::plan::{PlanGap, PlanGapKind};
    use specforge_ops::read::trace::{MissingLink, TraceLinkStatus};

    fn plan(ok: bool, resolved: &[&str], gaps: Vec<PlanGap>) -> PlanOutcome {
        PlanOutcome {
            ok,
            resolved_entries: resolved.iter().map(ToString::to_string).collect(),
            gaps,
        }
    }

    fn gap(kind: PlanGapKind, context: &str) -> PlanGap {
        PlanGap {
            source: String::new(),
            target: String::new(),
            kind,
            context: context.into(),
        }
    }

    #[test]
    fn plan_text_lists_resolved_entries_gaps_and_the_verdict() {
        let failed = plan(
            false,
            &["b", "c"],
            vec![
                gap(
                    PlanGapKind::UnresolvedEntity,
                    "E003: unresolved entity 'ghost' in plan — not found in graph",
                ),
                gap(
                    PlanGapKind::Ordering,
                    "'b' depends on 'c' (via depends_on), but 'c' appears later in the plan",
                ),
            ],
        );
        assert_eq!(
            plan_text(&failed),
            "resolved: b, c\n\
             gaps (2):\n  \
             - unresolved_entity: E003: unresolved entity 'ghost' in plan — not found in graph\n  \
             - ordering: 'b' depends on 'c' (via depends_on), but 'c' appears later in the plan\n\
             plan check: failed\n"
        );
        assert_eq!(
            plan_text(&plan(true, &["c", "b"], vec![])),
            "resolved: c, b\ngaps: none\nplan check: passed\n"
        );
        assert!(
            plan_text(&plan(
                false,
                &[],
                vec![gap(PlanGapKind::MissingPlanEntry, "x")]
            ))
            .starts_with("resolved: none\ngaps (1):")
        );
    }

    #[test]
    fn chain_text_marks_missing_links() {
        let chain = TraceChain {
            entity_id: "b".into(),
            entity_kind: "behavior".into(),
            upstream: vec![],
            downstream: vec![TraceLink {
                entity_id: "a".into(),
                entity_kind: "feature".into(),
                edge_label: "features".into(),
                depth: 1,
                status: TraceLinkStatus::Resolved,
            }],
            missing: vec![MissingLink {
                from: "b".into(),
                from_kind: "behavior".into(),
                edge_label: "invariants".into(),
                edge_type: None,
                expected_kind: "invariant".into(),
                depth: 1,
                required: false,
                status: TraceLinkStatus::Missing,
            }],
        };
        assert_eq!(
            chain_text(&chain),
            concat!(
                "b [behavior]\n",
                "  upstream:\n",
                "    (none)\n",
                "  downstream:\n",
                "    -features-> a [feature] (depth 1)\n",
                "  missing:\n",
                "    MISSING b -invariants-> [invariant] (depth 1)\n",
            )
        );
    }

    /// The trace of AUTH-07 over the routed GSD fixture whose spec is `spec`, along its kind's route.
    fn routed(spec: &str) -> TraceOutcome {
        use specforge_project::SharedRuntime;
        use specforge_project::testing::{gsd_routed_project, gsd_routed_runtime};
        let project = gsd_routed_project(spec);
        let compiled =
            project.compile_in(std::sync::Arc::new(gsd_routed_runtime()) as SharedRuntime);
        let traced = specforge_ops::read::trace::trace(
            &ProjectView::of(&compiled),
            specforge_ops::read::trace::Target::Entity("AUTH-07"),
            specforge_ops::read::trace::Follow::Declared,
        );
        let (terminal, _) = crate::terminal::Terminal::captured("", crate::terminal::Facts::PIPED);
        Notes::new(&terminal, OutputFormat::Human).noticed(traced.unwrap())
    }

    #[specforge_test_macros::test(
        behavior = "render_trace_thread",
        verify = "specforge trace prints a route trace's thread, and --reach the generic chain"
    )]
    fn a_route_trace_prints_its_thread_text() {
        use specforge_project::testing::GSD_ROUTED_SPEC;
        assert_eq!(
            trace_text(&routed(GSD_ROUTED_SPEC), Partial::No),
            "AUTH-07 → plan 03-02 → T1 → 5e2a91c0d3b4 (Gsd-Task) → js:app/test/auth.test.js#keeps a \
             session (passed) → TR1 missing\n"
        );
    }

    #[test]
    fn an_incomplete_route_trace_prints_its_one_line_unless_partial() {
        use specforge_project::testing::GSD_ROUTED_SPEC;
        let spec = GSD_ROUTED_SPEC.replace(
            "  requirements [AUTH-07]\n  files_modified",
            "  files_modified",
        );
        let outcome = routed(&spec);
        assert_eq!(
            trace_text(&outcome, Partial::No),
            "AUTH-07: incomplete thread (no plan); print it with: specforge trace AUTH-07 --partial\n"
        );
        assert_eq!(
            trace_text(&outcome, Partial::Yes),
            "AUTH-07 → no plan → js:app/test/auth.test.js#keeps a session (passed) → TR1 missing\n"
        );
    }

    #[specforge_test_macros::test(
        behavior = "render_trace_thread",
        verify = "specforge trace prints a route's thread as human text unless --format json is given, and any other trace as json"
    )]
    fn the_default_format_is_human_for_a_route_trace_and_json_otherwise() {
        use clap::Parser;
        use specforge_project::testing::GSD_ROUTED_SPEC;
        #[derive(clap::Parser)]
        struct Line {
            #[command(flatten)]
            args: Args,
        }
        let args = |line: &[&str]| {
            Line::try_parse_from(std::iter::once("trace").chain(line.iter().copied()))
                .unwrap()
                .args
        };
        let route = routed(GSD_ROUTED_SPEC);
        let chain = TraceOutcome::Every(specforge_ops::read::trace::Traces {
            schema_version: "1".into(),
            traces: Vec::new(),
        });
        assert_eq!(args(&["AUTH-07"]).format_of(&route), OutputFormat::Human);
        assert_eq!(
            args(&["AUTH-07", "--format", "json"]).format_of(&route),
            OutputFormat::Json
        );
        assert_eq!(args(&[]).format_of(&chain), OutputFormat::Json);
        assert_eq!(
            args(&["--route", "gsd/requirement-thread"]).format_of(&chain),
            OutputFormat::Human
        );
        // With --plan: json unless asked, the check and any outcome alike.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("plan.json");
        std::fs::write(&file, r#"{"plan_id":"p","entries":[]}"#).unwrap();
        let file = file.to_str().unwrap();
        let plan = args(&["--plan", file]);
        assert_eq!(plan.asked_or_json(), OutputFormat::Json);
        assert_eq!(plan.format_of(&chain), OutputFormat::Json);
        let human = args(&["--plan", file, "--format", "human"]);
        assert_eq!(human.format_of(&chain), OutputFormat::Human);
        // A refusal and a plan's check are decided before any outcome: json, unless asked.
        assert_eq!(args(&["--reach"]).asked_or_json(), OutputFormat::Json);
        assert_eq!(
            args(&["--format", "human"]).asked_or_json(),
            OutputFormat::Human
        );
    }
}
