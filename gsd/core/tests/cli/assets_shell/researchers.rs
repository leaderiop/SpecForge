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

/// `fence`, a record-the-answer fence of a workflow, filled for `question` of `research`, answered by `decision`, in
/// the scope `(variable, id)` the fence names.
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
            ("QUESTION", QUESTION),
            ("UNCLEAR", "which order users expect"),
        ],
    )
}

/// Runs `fence` under gsd-strict on `p`: it records the user's answer as decision `decision` and the question
/// `question` resolved by it, adding no code.
fn records_the_answer_then_the_question(
    sh: &Shell,
    p: &Project,
    fence: &str,
    decision: &str,
    question: &str,
) {
    let applied = typed::apply(sh, p, fence);
    assert_eq!(applied.code, 0, "{}{}", applied.out, applied.err);
    assert!(applied.added.is_empty(), "{:?}", applied.added);
    assert_eq!(p.fields_of(decision)["text"], ANSWER);
    let fields = p.fields_of(question);
    assert_eq!(fields["question"], QUESTION);
    let resolved = fields["resolved_by"].to_string();
    assert!(
        resolved.contains(decision.rsplit('/').next().expect("a name")),
        "{question} is resolved by {resolved}"
    );
}

/// The open question gsd-strict refuses at research is the user's to answer, on plan-phase's path: the planner is
/// told to record no decision for it and to list it in its return, and plan-phase's step 9d asks the user and records
/// the answer, then the question; that fence, run as written under gsd-strict, is refused without its decision line
/// and records both with it.
#[test]
fn plan_phase_records_the_users_answer_then_the_question() {
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
    assert!(status.contains("open question"), "{status}");
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
    let fence = answered(
        &typed::typed_fence(&plan_phase, "## 9d."),
        ("PHASE_ID", "v1.0/02"),
        "v1.0/02/02-RESEARCH",
        "D-04",
        "02-Q1",
    );
    records_the_answer_then_the_question(
        &sh,
        &p,
        &fence,
        "v1.0/02/D-04",
        "v1.0/02/02-RESEARCH/02-Q1",
    );
}

/// The quick plan `261121-abc` with its research, minted under gsd-strict as quick's init step and research step write
/// them (the mint reports `gsd/W309`, settled later by the planner).
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
    ] {
        let run = p.run(args);
        assert_eq!(run.code, 0, "{}", run.all());
    }
    p
}

/// On quick's and quick-batch's paths the research step hands a returned question to the planner's prompt, which
/// carries it with the same rule, and the step after the planner asks the user and records the answer, then the
/// question: each workflow's fence, run as written under gsd-strict on a quick plan with research, is refused without
/// its decision line and records both with it.
#[test]
fn quick_and_quick_batch_record_the_users_answer_then_the_question() {
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
            "261121-abc-Q1",
        );
        records_the_answer_then_the_question(
            &sh,
            &p,
            &fence,
            "261121-abc-D-01",
            "261121-abc-RESEARCH/261121-abc-Q1",
        );
    }
}
