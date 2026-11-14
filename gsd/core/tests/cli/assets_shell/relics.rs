//! Upstream's state, read here as the graph's: the UI gate reads a computed field, and closing a milestone writes no
//! roadmap file (ADR 0343 D3); the review agent's return is written by the model and only checked here (D6).

use super::{Shell, asset, block, fence_after, line, q};
use crate::harness::taskboard;

/// After `/gsd-complete-milestone`, autonomous checks the close by reading the milestone's computed `state`: it is
/// `shipped` once the close recorded the milestone's completion, and anything else routes to the blocker. No archive
/// file is looked up: a close writes none.
#[test]
fn the_autonomous_close_check_reads_the_milestones_completion() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = crate::cmd_milestone::ready();
    let check = fence_after(
        &asset("gsd-core/workflows/autonomous.md"),
        "After complete-milestone returns, verify it produced output:",
    );
    let state = || {
        let (out, err, _) = sh.run_in(
            p.root(),
            &format!("milestone_version=v1.0\n{check}\nprintf %s \"$MILESTONE_STATE\""),
        );
        (out, err)
    };
    let (before, err) = state();
    assert_eq!(before, "in_progress", "{err}");
    let close = p.run(&["gsd", "close-milestone", "v1.0"]);
    assert_eq!(close.code, 0, "{}", close.all());
    let (after, err) = state();
    assert_eq!(after, "shipped", "{err}");
}

/// Transition's Route A asks whether the next phase has a context: it reads the phase's `context` entity, whose id it
/// prints, and prints nothing for a phase that has none. A context is an entity, never a `*-CONTEXT.md` file.
#[test]
fn the_next_phases_context_is_read_from_the_graph() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let entity = p.run(&[
        "query",
        "v1.0/02",
        "--depth",
        "1",
        "--kinds",
        "context",
        "--pick",
        ".nodes[kind=context].id",
    ]);
    assert_eq!(
        entity.stdout.trim(),
        "v1.0/02/02-CONTEXT",
        "{}",
        entity.all()
    );
    let check = fence_after(
        &asset("gsd-core/workflows/transition.md"),
        "**Check if next phase has CONTEXT.md:**",
    );
    for (next, want) in [("02", "v1.0/02/02-CONTEXT"), ("03", "")] {
        let script = format!(
            "MILESTONE=v1.0\n{}\nprintf %s \"$NEXT_CONTEXT\"",
            check.replace("{next_phase}", next)
        );
        let (out, err, _) = sh.run_in(p.root(), &script);
        assert_eq!(out, want, "phase {next}: {err}");
    }
}

/// The milestone audit's 5c reads every summary of the milestone in one pick: a row per summary, its id, then its
/// requirement ids as a JSON list. No guard over a file path is left to skip an entity id.
#[test]
fn the_audit_reads_every_summarys_requirements() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let check = fence_after(&asset("gsd-core/workflows/audit-milestone.md"), "### 5c.");
    let (out, err, _) = sh.run_in(p.root(), &format!("MILESTONE=v1.0\n{check}"));
    let rows: Vec<Vec<&str>> = out.lines().map(|l| l.split('\u{1f}').collect()).collect();
    assert_eq!(
        rows,
        [
            ["v1.0/01/01-01/01-01-SUMMARY", "[\"TASK-01\"]"],
            ["v1.0/01/01-02/01-02-SUMMARY", "[\"TASK-02\"]"],
        ],
        "{err}"
    );
}

/// A `ship:post` hook that consumes `UAT.md` is handed the phase's `uat_session` id, read once in the step's first
/// fence, and is skipped only when the phase has none. Ship names no phase directory.
#[test]
fn a_ship_hook_is_handed_the_phases_uat_session() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/ship.md");
    assert!(!text.contains("PHASE_DIR"), "ship.md names PHASE_DIR");
    let read = line(&text, "UAT_SESSION=");
    for (phase, want) in [("v1.0/01", "v1.0/01/01-UAT"), ("v1.0/02", "")] {
        let (out, err, _) = sh.run_in(
            p.root(),
            &format!("PHASE_ID={phase}\n{read}\nprintf %s \"$UAT_SESSION\""),
        );
        assert_eq!(out, want, "{phase}: {err}");
    }
    // The hook's agent prompts hand it that id, and every variable they name is assigned before them (`PHASE_ID` is
    // the prompt's subject, bound by its closing line).
    let var = regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("a regex");
    let agents: Vec<(usize, &str)> = text
        .match_indices('\n')
        .map(|(at, _)| at + 1)
        .map(|at| (at, text[at..].lines().next().unwrap_or_default()))
        .filter(|(_, l)| l.trim_start().starts_with("`Agent("))
        .collect();
    assert_eq!(agents.len(), 2, "{agents:?}");
    for (at, agent) in agents {
        assert!(agent.contains("Consume: ${UAT_SESSION}."), "{agent}");
        for name in var.captures_iter(agent).map(|c| c[1].to_string()) {
            let assigned = text[..at]
                .lines()
                .any(|l| l.trim_start().starts_with(&format!("{name}=")));
            assert!(
                name == "PHASE_ID" || assigned,
                "{name} is unset before: {agent}"
            );
        }
    }
}

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
    let step = block(
        &text,
        "<step name=\"reorganize_roadmap_and_delete_originals\">",
        "**Safety commit",
    );
    let code: Vec<String> = gsd_kit::markdown::fences(&step)
        .into_iter()
        .filter(|f| f.language == "bash")
        .map(|f| f.text)
        .collect();
    let tmp = tempfile::tempdir().expect("a dir");
    let (_, err, _) = sh.run_in(
        tmp.path(),
        &format!("specforge() {{ :; }}\n{}", code.concat()),
    );
    let left: Vec<_> = std::fs::read_dir(tmp.path())
        .expect("the dir")
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert!(left.is_empty(), "{left:?}: {err}");
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
