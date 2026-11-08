//! Upstream's state, read here as the graph's: the UI gate reads a computed field, and closing a milestone writes no
//! roadmap file (ADR 0343 D3); the review agent's return is written by the model and only checked here (D6).

use super::{Shell, asset, block, q};
use crate::harness::taskboard;

/// The UI gate reads the phase's computed `frontend` field (`@gsd/ui` computes it): no field holds upstream's
/// `**UI hint**: yes`, so a grep of the phase's text for it is always false.
#[test]
fn the_ui_gate_reads_the_phases_frontend_field() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/progress.md");
    let lines: Vec<&str> = text.lines().map(str::trim_start).collect();
    let at = lines
        .iter()
        .position(|l| l.starts_with("PHASE_SECTION=") || l.starts_with("PHASE_HAS_UI="))
        .expect("the UI gate");
    let gate = lines[at..at + 2].join("\n");
    for (answer, want) in [("true", "true"), ("", "false")] {
        let stub = format!(
            "specforge() {{ case \"$*\" in *fields.frontend*) printf '%s' {a};; *) [ -n {a} ] && printf '## Phase 1\\n';; esac; }}",
            a = q(answer)
        );
        let out = sh.run(&format!(
            "{stub}\nCURRENT_PHASE=v1.0/01\n{gate}\nprintf '%s' \"$PHASE_HAS_UI\""
        ));
        assert_eq!(out, want, "{answer:?}");
    }
}

/// Closing a milestone writes no roadmap file: the roadmap is the `milestone` and `phase` entities. Upstream's sentinel
/// line `printf … > <path>`, with its path swapped for prose, wrote a file named `a`.
#[test]
fn the_milestone_close_writes_no_scratch_file() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/complete-milestone.md");
    let step = block(&text, "**Reorganize ROADMAP.md**", "**Safety commit");
    let mut code = Vec::new();
    let mut fenced = false;
    for l in step.lines() {
        if l.trim_start().starts_with("```") {
            fenced = !fenced && l.trim() == "```bash";
            continue;
        }
        if fenced {
            code.push(l);
        }
    }
    let tmp = tempfile::tempdir().expect("a dir");
    let (_, err, _) = sh.run_in(
        tmp.path(),
        &format!("specforge() {{ :; }}\n{}", code.join("\n")),
    );
    let left: Vec<_> = std::fs::read_dir(tmp.path())
        .expect("the dir")
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert!(left.is_empty(), "{left:?}: {err}");
}

/// The model writes the review agent's counts and sections into the block; the shell checks the counts are whole
/// numbers and parses nothing.
#[test]
fn the_review_counts_are_written_by_the_model_and_checked() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/plan-review-convergence.md");
    let counts = block(&text, "HIGH_COUNT=", "UNRESOLVED_COUNT=");
    let show = "printf '[%s][%s][%s]' \"$UNRESOLVED_COUNT\" \"$HIGH_LINES\" \"$ACTIONABLE_LINES\"";
    let written = |high: &str| {
        let mut sections = [Some("- a\n- b"), None].into_iter();
        let filled = counts
            .lines()
            .filter_map(|l| match l {
                "<those lines; no line when the section is absent>" => sections.next().unwrap(),
                _ => Some(l),
            })
            .collect::<Vec<_>>()
            .join("\n")
            .replace("{current_high}", high)
            .replace("{current_actionable}", "1");
        sh.run(&format!("{filled}\n{show}"))
    };
    assert_eq!(written("2"), "[3][- a\n- b\n][]");
    assert!(
        written("two")
            .starts_with("CYCLE_SUMMARY present but current_high is missing or malformed")
    );
}

/// Chunked planning resumes past a plan whose entity exists, not past a file with a frontmatter fence; `--reviews`
/// replans it.
#[test]
fn a_chunked_plan_resumes_past_its_entity() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md");
    let check = block(&text, "PLAN_ENTITY=", "fi");
    for (plan, reviews, want) in [
        ("01-01", "false", "skip"),
        ("09-09", "false", "plan"),
        ("01-01", "true", "plan"),
    ] {
        let script = format!(
            "PHASE_ID=v1.0/01; plan_id={plan}; REVIEWS={reviews}; S=plan\n{}\nprintf %s \"$S\"",
            check.replace(": # resume safety", "S=skip # resume safety")
        );
        let (out, err, _) = sh.run_in(p.root(), &script);
        assert_eq!(out, want, "{plan} reviews={reviews}: {err}");
    }
}

/// The verification step reads its own inputs: the phase's number and requirement ids from the graph and the response
/// language from the settings. Neither caller hands it a bundle, and nothing without a source (a model, a section
/// manifest, a phase directory or a requirements file) is left for the model to fill.
#[test]
fn the_verify_step_reads_its_own_inputs() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    p.settings(serde_json::json!({ "response_language": "fr" }));
    let text = asset("gsd-core/workflows/execute-phase/steps/verify-phase-goal.md");
    let resolve = block(&text, "PHASE_NUMBER=", "PHASE_GOAL=");
    let (out, err, _) = sh.run_in(
        p.root(),
        &format!(
            "PHASE_ID=v1.0/01\n{resolve}\nprintf '%s|%s|%s' \"$PHASE_NUMBER\" \"$(printf '%s' \"$PHASE_REQ_IDS\" | tr '\\n' ,)\" \"$RESPONSE_LANGUAGE\""
        ),
    );
    assert_eq!(out, "01|TASK-01,TASK-02|fr", "{err}");
    for unsourced in [
        "section_manifest",
        "{verifier_model}",
        "{phase_dir}",
        "{requirements_path}",
        "{phase_req_ids}",
        "{response_language}",
    ] {
        assert!(
            !text.contains(unsourced),
            "the step still reads {unsourced}"
        );
    }
}
