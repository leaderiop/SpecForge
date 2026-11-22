//! The discuss and project researchers' reads, run as the agents hold them under bash with only `specforge`: each
//! names its context pack, and the analyzer's search and anchors find the earlier phases' work; and the route of an
//! open question, recorded open by the phase researcher and resolved later by the user's answer (ADR 0385).

use super::{Shell, asset, block, line, span, typed};
use crate::commands_support::{checked, refused, strict};
use crate::harness::{Project, Run, taskboard};
use crate::prompts_gates::{not_refused_by, refused_by};

/// The command of the code span in agent `name` that starts with `prefix`, its placeholder (from the first `<` or
/// `"` on: `<phase>`, `"$PHASE_ID"`, `"<goal terms>"`) replaced by `subject`.
fn read_of(name: &str, prefix: &str, subject: &str) -> String {
    let text = asset(&format!("agents/{name}.md"));
    let span = span(&text, prefix);
    let at = span.find(['<', '"']).expect("a placeholder");
    format!("{}'{subject}'", &span[..at])
}

/// Every researcher's pack route, as its agent names it, compiles on the taskboard; the phase packs carry the earlier
/// phase's context, and the analyzer's search finds the decision an earlier phase recorded.
#[test]
fn the_discuss_researchers_read_their_packs_and_search() {
    let sh = Shell::only(&[]);
    let p = taskboard();
    for (agent, subject, carries) in [
        (
            "gsd-assumptions-analyzer",
            "v1.0/02",
            Some("v1.0/02/02-CONTEXT"),
        ),
        (
            "gsd-assumptions-analyzer.compact",
            "v1.0/02",
            Some("v1.0/02/02-CONTEXT"),
        ),
        (
            "gsd-advisor-researcher",
            "v1.0/02",
            Some("v1.0/02/02-CONTEXT"),
        ),
        (
            "gsd-advisor-researcher.compact",
            "v1.0/02",
            Some("v1.0/02/02-CONTEXT"),
        ),
        ("gsd-research-synthesizer", "taskboard", None),
        ("gsd-roadmapper", "taskboard", None),
    ] {
        let role = agent
            .trim_end_matches(".compact")
            .trim_start_matches("gsd-");
        let line = read_of(agent, &format!("specforge context gsd/{role}"), subject);
        let (out, err, code) = sh.run_in(p.root(), &line);
        assert_eq!(code, 0, "{agent}: {line}: {err}");
        assert!(!out.trim().is_empty(), "{agent}: {line} printed no pack");
        if let Some(id) = carries {
            assert!(out.contains(id), "{agent}: the pack lacks {id}:\n{out}");
        }
    }
    for agent in [
        "gsd-assumptions-analyzer",
        "gsd-assumptions-analyzer.compact",
    ] {
        let search = read_of(agent, "specforge search", "renamed");
        let (out, err, code) = sh.run_in(p.root(), &search);
        assert_eq!(code, 0, "{agent}: {search}: {err}");
        assert!(out.contains("v1.0/02/D-02"), "{agent}: {search}:\n{out}");
        let anchors = read_of(agent, "specforge anchors <", "v1.0/02/D-02");
        let (_, err, code) = sh.run_in(p.root(), &anchors);
        assert_eq!(code, 0, "{agent}: {anchors}: {err}");
        let by_file = read_of(agent, "specforge anchors --file", "src");
        let (_, err, code) = sh.run_in(p.root(), &by_file);
        assert_eq!(code, 0, "{agent}: {by_file}: {err}");
    }
}

/// The project researcher's pack line, as new-project's and new-milestone's researcher spawns name it, runs on the
/// taskboard and prints the pack.
#[test]
fn the_project_researcher_reads_its_pack() {
    let sh = Shell::only(&[]);
    let p = taskboard();
    for workflow in [
        "gsd-core/workflows/new-project/detail/elaboration.md",
        "gsd-core/workflows/new-milestone.md",
    ] {
        let text = asset(workflow);
        let line = text
            .lines()
            .map(str::trim_start)
            .find_map(|l| l.strip_prefix("- specforge context gsd/project-researcher "))
            .unwrap_or_else(|| panic!("{workflow}: no project researcher pack line"));
        let subject = line.split_whitespace().next().expect("a subject");
        assert_eq!(subject, "{project}", "{workflow}: {line}");
        let (out, err, code) = sh.run_in(
            p.root(),
            "specforge context gsd/project-researcher taskboard",
        );
        assert_eq!(code, 0, "{workflow}: {err}");
        assert!(out.contains("taskboard"), "{workflow}: {out}");
    }
}

/// A question holding a double quote, an em dash and a non-ASCII letter.
const QUESTION: &str = "Sort by \"due\" — or by é?";

/// The file of phase v1.0/02, found by its id; the phase researcher's read of it (a depth-0 query of the id) gives the
/// same file.
fn phase_file(p: &Project) -> String {
    let phase = p
        .nodes_of("phase")
        .into_iter()
        .find(|n| n["id"] == "v1.0/02")
        .expect("phase v1.0/02");
    let file = phase["file"]
        .as_str()
        .expect("the phase's file")
        .to_string();
    let read = p.run(&[
        "query",
        "v1.0/02",
        "--depth",
        "0",
        "--pick",
        ".nodes[0].file",
    ]);
    assert_eq!(read.code, 0, "{}", read.all());
    assert_eq!(
        read.stdout.trim(),
        file,
        "the researcher's read finds the phase's file"
    );
    file
}

/// `specforge new open_question OQ2` in `file` for the research of v1.0/02, with these extra arguments.
fn record_question(p: &Project, file: &str, extra: &[&str]) -> Run {
    let mut args = vec![
        "new",
        "open_question",
        "OQ2",
        "Sort order",
        "--in",
        file,
        "--research",
        "v1.0/02/02-RESEARCH",
        "--question",
        QUESTION,
        "--unclear",
        "which order users expect",
    ];
    args.extend_from_slice(extra);
    p.run(&args)
}

/// An open question no decision resolves is `gsd/W204`, settled later (ADR 0385): the researcher records it open, in
/// every profile. Under gsd-strict the write reports it as an error, `check` fails on it, and the plan-phase entry gate
/// refuses the next run on the phase; the user's answer, recorded as a decision and set as the question's
/// `resolved_by`, settles it and adds nothing. Unsetting that `resolved_by` again creates nothing, so it is refused
/// (E080 with gsd/W204) and changes nothing. Under gsd-compat the question is a warning.
#[test]
fn an_open_question_is_recorded_open_and_its_answer_resolves_it_later() {
    let id = "v1.0/02/02-RESEARCH/OQ2";
    let held = |p: &Project| {
        p.nodes_of("open_question")
            .iter()
            .any(|n| n["id"].as_str() == Some(id))
    };
    let p = taskboard();
    strict(&p);
    let file = phase_file(&p);
    let run = record_question(&p, &file, &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("gsd/W204"), "{}", run.all());
    assert!(held(&p));
    assert_eq!(p.fields_of(id)["question"], QUESTION);
    assert_eq!(
        checked(&p, "gsd/W204"),
        (true, vec![("Error".to_string(), id.to_string())])
    );
    refused_by(&p, "plan_phase", &["v1.0/02"], "W204");

    let decided = p.run(&[
        "new",
        "decision",
        "D-04",
        ANSWER,
        "--in",
        &file,
        "--phase",
        "v1.0/02",
        "--text",
        ANSWER,
        "--origin",
        "checkpoint",
    ]);
    assert_eq!(decided.code, 0, "{}", decided.all());
    let set = p.run(&["set", id, "resolved_by", "D-04"]);
    assert_eq!(set.code, 0, "{}", set.all());
    assert_eq!(checked(&p, "gsd/W204").1, Vec::new());
    not_refused_by(&p, "plan_phase", &["v1.0/02"], "W204");

    let kept = p.read(&file);
    let unset = p.run(&["unset", id, "resolved_by"]);
    refused(&unset, "E080", &["gsd/W204", "nothing was written"]);
    assert_eq!(p.read(&file), kept, "the refused unset changed {file}");

    let p = taskboard();
    let run = record_question(&p, &phase_file(&p), &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("gsd/W204"), "{}", run.all());
    assert_eq!(
        checked(&p, "gsd/W204").1,
        [("Warning".to_string(), id.to_string())]
    );
}

/// The user's answer to [`QUESTION`].
const ANSWER: &str = "Sort by due date — then by title";

/// `fence`, a record-the-answer fence of a workflow, filled for the recorded question `question` of `research`,
/// answered by `decision`, in the scope `(variable, id)` the fence names.
fn answered(
    fence: &str,
    scope: (&str, &str),
    research: &str,
    decision: &str,
    question: &str,
) -> String {
    typed::filled(
        fence,
        &[
            scope,
            ("RESEARCH_ID", research),
            ("DECISION_ID", decision),
            ("QUESTION_ID", question),
            ("ANSWER", ANSWER),
        ],
    )
}

/// Runs `fence` under gsd-strict on `p`, where the research recorded `question` open (gsd/W204, an error): without its
/// decision line the set names no decision and is refused, leaving the question open; whole, it records the user's
/// answer as decision `decision`, sets it as the question's `resolved_by`, settles its gsd/W204 and adds no code.
fn the_answer_resolves_the_recorded_question(
    sh: &Shell,
    p: &Project,
    fence: &str,
    decision: &str,
    question: &str,
) {
    let open = |p: &Project| {
        checked(p, "gsd/W204")
            .1
            .iter()
            .any(|(severity, entity)| severity == "Error" && entity == question)
    };
    assert!(open(p), "{question} is not recorded open");
    let alone: String = fence
        .lines()
        .filter(|l| !l.starts_with("specforge new decision "))
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(alone, fence, "the fence records a decision");
    let refused = typed::apply(sh, p, &alone);
    assert_ne!(refused.code, 0, "{}{}", refused.out, refused.err);
    assert!(p.fields_of(question)["resolved_by"].is_null());

    let applied = typed::apply(sh, p, fence);
    assert_eq!(applied.code, 0, "{}{}", applied.out, applied.err);
    assert!(applied.added.is_empty(), "{:?}", applied.added);
    assert_eq!(p.fields_of(decision)["text"], ANSWER);
    let resolved = p.fields_of(question)["resolved_by"].to_string();
    assert!(
        resolved.contains(decision.rsplit('/').next().expect("a name")),
        "{question} is resolved by {resolved}"
    );
    assert!(!open(p), "{question} is still open");
}

/// The phase researcher's record fence (its rule 1), filled for the research `03-RESEARCH` of phase `phase` (one
/// numbered 03) and its open question `Q1`, recorded open.
pub(super) fn research_fence(phase: &str) -> String {
    typed::filled(
        &typed::typed_fence(
            &asset("agents/gsd-phase-researcher.md"),
            "1. **Create the entity first**",
        ),
        &[
            ("PHASE_ID", phase),
            ("RESEARCH", "03-RESEARCH"),
            ("TITLE", "Listing research"),
            ("CONFIDENCE", "high"),
            ("SUMMARY", "Filter in memory; the board is small"),
            ("FINDING", "The board fits in memory"),
            ("PITFALL", "Case-sensitive filters"),
            ("GOES_WRONG", "A filter misses a title in another case"),
            ("WHY", "String includes is case-sensitive"),
            ("AVOID", "Lower-case both sides"),
            ("PATTERN", "Filter then sort"),
            ("WHAT", "Filter the open tasks, then sort them by id"),
            ("QUESTION", QUESTION),
            ("KNOWN", "users sort by hand today"),
            ("UNCLEAR", "which order users expect"),
            ("RECOMMENDATION", "ask the user"),
        ],
    )
}

/// The phase researcher records every open question open: its rule names no exception and no return in place of the
/// record, and its fence records `Q1` with no `resolved_by`. Under gsd-strict the fence is accepted and adds gsd/W204
/// only; plan-phase displays the returned ids and the quick steps pass them on as questions the research recorded.
#[test]
fn the_researcher_records_every_open_question_open() {
    let researcher = asset("agents/gsd-phase-researcher.md");
    let rule = block(
        &researcher,
        "4. **One item per call.**",
        "5. **If a call is still refused",
    )
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ");
    for words in [
        "record every `open_question` open",
        "no `--resolved_by`",
        "`gsd/W204`",
        "is not refused for it",
        "never return one in place of recording it",
        "by its own id (`Q1`, the last segment of its path `$RESEARCH_ID/Q1`), under `### Open Questions`",
    ] {
        assert!(rule.contains(words), "rule 4 lacks {words}:\n{rule}");
    }
    assert!(!rule.contains("send it again without"), "{rule}");
    let template = block(&researcher, "### Open Questions", "### Ready for Planning");
    assert!(
        template.contains("you recorded, by its own id under the research, e.g. `Q1`"),
        "{template}"
    );
    let handle = block(
        &asset("gsd-core/workflows/plan-phase.md"),
        "### Handle Researcher Return",
        "<!-- gsd:section id=\"research-only-early-exit\"",
    );
    assert!(handle.contains("the research recorded"), "{handle}");
    for step in [
        "gsd-core/workflows/quick/steps/research-phase.md",
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
    ] {
        let text = asset(step).split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            text.contains("questions the research recorded open"),
            "{step}"
        );
    }

    let sh = Shell::only(&[]);
    let p = taskboard();
    strict(&p);
    let run = typed::apply(&sh, &p, &research_fence("v1.0/03"));
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    assert_eq!(run.added, ["gsd/W204"]);
    let question = "v1.0/03/03-RESEARCH/Q1";
    assert_eq!(p.fields_of(question)["question"], QUESTION);
    assert!(p.fields_of(question)["resolved_by"].is_null());
}

/// A recorded open question is the user's to answer, on plan-phase's path: the planner is told to record no decision
/// for it and to list it in its return, and plan-phase's step 9d asks the user, records the answer as a decision and
/// sets it as the question's `resolved_by`. Under gsd-strict the researcher's fence records the question open, the
/// plan-phase entry gate then refuses the phase, step 9d's fence settles it, and the gate lets the phase through.
#[test]
fn plan_phase_resolves_the_recorded_question_with_the_users_answer() {
    let sh = Shell::only(&[]);
    let plan_phase = asset("gsd-core/workflows/plan-phase.md");
    let context = block(&plan_phase, "<planning_context>", "</planning_context>");
    for words in [
        "**Open questions the research returned:**",
        "the user's to answer",
        "record no decision",
    ] {
        assert!(
            context.contains(words),
            "the planner's prompt lacks {words}"
        );
    }
    assert!(
        !plan_phase.contains("Answer each with a decision"),
        "plan-phase has the planner answer"
    );
    let complete = line(&plan_phase, "- **`## PLANNING COMPLETE`:**");
    assert!(complete.contains("step 9d"), "{complete}");
    let status = block(&plan_phase, "## 14. Present Final Status", "## 15.");
    assert!(
        status.contains("open in the graph") && status.contains("entry gate"),
        "{status}"
    );
    let answers = block(&plan_phase, "## 9d.", "## 10.");
    assert!(
        answers.contains("stays open in the graph") && answers.contains("entry gate of plan-phase"),
        "{answers}"
    );
    let planner = asset("agents/gsd-planner.md");
    let write = block(&planner, "<step name=\"write_phase_prompt\">", "</step>");
    for words in [
        "is the user's to answer, not yours",
        "Record no decision",
        "`### Open Questions`",
    ] {
        assert!(
            write.contains(words),
            "the planner's write step lacks {words}:\n{write}"
        );
    }
    let returned = block(
        &asset("gsd-core/references/planner-guidance.md"),
        "## PLANNING COMPLETE",
        "### Next Steps",
    );
    assert!(returned.contains("### Open Questions"), "{returned}");

    let p = taskboard();
    strict(&p);
    let run = typed::apply(&sh, &p, &research_fence("v1.0/03"));
    assert_eq!(run.code, 0, "{}{}", run.out, run.err);
    refused_by(&p, "plan_phase", &["v1.0/03"], "W204");
    let fence = answered(
        &typed::typed_fence(&plan_phase, "## 9d."),
        ("PHASE_ID", "v1.0/03"),
        "v1.0/03/03-RESEARCH",
        // Not D-01: a bare D-01 would resolve to the project's own decision while the phase has none.
        "D-04",
        "Q1",
    );
    the_answer_resolves_the_recorded_question(
        &sh,
        &p,
        &fence,
        "v1.0/03/D-04",
        "v1.0/03/03-RESEARCH/Q1",
    );
    not_refused_by(&p, "plan_phase", &["v1.0/03"], "W204");
}

/// The quick plan `261121-abc` with its research and the research's open question, minted under gsd-strict as quick's
/// init step and research step write them (the mint reports `gsd/W309`, settled later by the planner; the question
/// `gsd/W204`, settled later by the user's answer).
fn quick_with_research() -> Project {
    let p = taskboard();
    strict(&p);
    for args in [
        &[
            "new",
            "--in",
            "spec/quick.spec",
            "--field",
            "type=execute",
            "--field",
            "files_modified=",
            "--field",
            "objective=Trim titles",
            "--field",
            "slug=trim-titles",
            "--",
            "plan",
            "261121-abc",
            "Trim titles",
        ][..],
        &[
            "new",
            "research",
            "261121-abc-RESEARCH",
            "Trim titles",
            "--in",
            "spec/quick.spec",
            "--plan",
            "261121-abc",
            "--researched",
            "2026-11-21",
            "--confidence",
            "medium",
            "--summary",
            "Titles are trimmed",
        ],
        &[
            "new",
            "open_question",
            "Q1",
            QUESTION,
            "--in",
            "spec/quick.spec",
            "--research",
            "261121-abc-RESEARCH",
            "--question",
            QUESTION,
            "--unclear",
            "which order users expect",
        ],
    ] {
        let run = p.run(args);
        assert_eq!(run.code, 0, "{}", run.all());
    }
    p
}

/// On quick's and quick-batch's paths the research step hands a recorded question to the planner's prompt, which
/// carries it with the same rule, and the step after the planner asks the user, records the answer as a decision and
/// sets it as the question's `resolved_by`: each workflow's fence, run as written under gsd-strict on a quick plan
/// whose research recorded the question open, is refused without its decision line and settles the question with it.
#[test]
fn quick_and_quick_batch_resolve_the_recorded_question_with_the_users_answer() {
    let sh = Shell::only(&[]);
    for (research, prompt, after, heading) in [
        (
            "gsd-core/workflows/quick/steps/research-phase.md",
            block(
                &asset("gsd-core/workflows/quick.md"),
                "<planning_context>",
                "</planning_context>",
            ),
            asset("gsd-core/workflows/quick.md"),
            "4. When the return lists `### Open Questions`",
        ),
        (
            "gsd-core/workflows/quick-batch/steps/research-phase.md",
            block(
                &asset("gsd-core/workflows/quick-batch/steps/planner-wave.md"),
                "<planning_context>",
                "</planning_context>",
            ),
            asset("gsd-core/workflows/quick-batch/steps/planner-wave.md"),
            "When an item's planner lists `### Open Questions`",
        ),
    ] {
        let step = asset(research)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            step.contains("**Open questions the research returned:**"),
            "{research}"
        );
        assert!(
            !step.contains("answers each with a"),
            "{research} has the planner answer"
        );
        for words in [
            "**Open questions the research returned:**",
            "the user's to answer",
            "record no decision",
        ] {
            assert!(
                prompt.contains(words),
                "{research}: the planner's prompt lacks {words}"
            );
        }
        let p = quick_with_research();
        let fence = answered(
            &typed::typed_fence(&after, heading),
            ("QUICK_ID", "261121-abc"),
            "261121-abc-RESEARCH",
            "261121-abc-D-01",
            "Q1",
        );
        the_answer_resolves_the_recorded_question(
            &sh,
            &p,
            &fence,
            "261121-abc-D-01",
            "261121-abc-RESEARCH/Q1",
        );
    }
}
