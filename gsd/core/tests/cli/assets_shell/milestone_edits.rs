//! The milestone family's steps (audit, close, transition, autonomous, graduation, next), run as the assets hold them
//! on a project copy, and the answers they read.

use super::{Shell, asset, block, fence_after, typed};
use crate::harness::taskboard;

/// The code of the first bash fence of `text` holding `mark`.
fn fence_holding(text: &str, mark: &str) -> String {
    let fence = gsd_kit::markdown::fences(text)
        .into_iter()
        .find(|f| f.language == "bash" && f.text.contains(mark))
        .unwrap_or_else(|| panic!("no bash fence holds {mark}"));
    super::code(&fence.text).to_string()
}

/// `/gsd-audit-milestone` as the asset holds it.
fn audit() -> String {
    asset("gsd-core/workflows/audit-milestone.md")
}

/// The audit's step-1 fence, with `$MILESTONE` set to `milestone`.
fn resolve_audit(milestone: &str) -> String {
    format!(
        "MILESTONE={milestone}\n{}",
        fence_holding(&audit(), "specforge new milestone_audit")
    )
}

/// The "Extract from result" list of `text`: the code spans of its first line that starts so.
fn extracted(text: &str) -> Vec<String> {
    let line = super::line(text, "Extract from result");
    line.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// The transition's roadmap step reads the milestone's phases from `progress-report`: on a taskboard its fence prints
/// JSON whose every `phases` item carries each key the step's "Extract from result" line names.
#[test]
fn the_transition_reads_the_next_phase_from_progress() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let text = asset("gsd-core/workflows/transition.md");
    let step = block(&text, "<step name=\"update_roadmap_and_state\">", "</step>");
    let code = fence_after(&step, "update_roadmap_and_state");
    let p = crate::harness::taskboard();
    let (out, err, status) = sh.run_in(p.root(), &code);
    assert_eq!(status, 0, "{err}");
    let answer: serde_json::Value = serde_json::from_str(&out).expect("a JSON answer");
    let phases = answer["phases"].as_array().expect("the phases");
    assert!(phases.len() > 1, "{answer}");
    let keys = extracted(&step);
    assert_eq!(keys.len(), 3, "{keys:?}");
    for key in &keys {
        let key = key.strip_prefix("phases[].").expect("a phases key");
        for phase in phases {
            assert!(phase.get(key).is_some(), "{key}: {phase}");
        }
    }
}

/// The transition's `evolve_project` fences, filled, on a taskboard: an invalidated requirement moves out of scope
/// with its reason, an emerged one is recorded in the phase that builds it, and the project's `what` is rewritten;
/// the check gains no finding.
#[test]
fn the_project_evolution_is_typed_edits() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/transition.md");
    let step = block(&text, "<step name=\"evolve_project\">", "</step>");
    let fences = [
        typed::filled(
            &fence_holding(&step, "tier out_of_scope"),
            &[
                ("REQ_ID", "LIST-01"),
                ("REASON", "Search replaces the filter"),
            ],
        ),
        typed::filled(
            &fence_holding(&step, "specforge new requirement"),
            &[
                ("REQ_ID", "LIST-02"),
                ("TEXT", "A user can search every board by a word"),
                ("CATEGORY", "Listing"),
                ("BUILT_BY", "v1.0/03"),
            ],
        ),
        typed::filled(
            &fence_holding(&step, "what \"$WHAT\""),
            &[(
                "WHAT",
                "A task board for small teams, searchable across boards.",
            )],
        ),
    ];
    for fence in &fences {
        let done = typed::apply(&sh, &p, fence);
        assert_eq!(done.code, 0, "{}{}", done.out, done.err);
        assert!(done.added.is_empty(), "{:?}\n{fence}", done.added);
    }
    let list = p.fields_of("LIST-01");
    assert_eq!(list["tier"], "out_of_scope", "{list}");
    assert_eq!(list["reason"], "Search replaces the filter", "{list}");
    assert_eq!(p.fields_of("LIST-02")["tier"], "v1");
    let built = p.fields_of("v1.0/03")["requirements"].to_string();
    assert!(built.contains("LIST-02"), "{built}");
    assert_eq!(
        p.fields_of("taskboard")["what"],
        "A task board for small teams, searchable across boards."
    );
}

/// The close step reads `close-milestone`'s answer and the milestone's computed stats: every key its "Extract from
/// result" line names is in the answer, and its stats fence prints an object with `phases` and `plans` and the state
/// `shipped`.
#[test]
fn the_close_reads_the_answer_and_the_milestones_stats() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = crate::cmd_milestone::ready();
    let run = p.run(&["gsd", "close-milestone", "v1.0", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let answer: serde_json::Value = serde_json::from_str(&run.stdout).expect("a JSON answer");
    let answer = &answer["output"];
    let text = asset("gsd-core/workflows/complete-milestone.md");
    let keys = extracted(&text);
    assert!(!keys.is_empty());
    for key in &keys {
        assert!(answer.get(key).is_some(), "{key}: {answer}");
    }
    let stats = fence_holding(&text, "STATS=$(");
    let (out, err, code) = sh.run_in(p.root(), &format!("MILESTONE=v1.0\n{stats}"));
    assert_eq!(code, 0, "{err}");
    let line = out
        .lines()
        .find(|l| l.starts_with("stats: "))
        .expect("the stats");
    let stats: serde_json::Value =
        serde_json::from_str(&line["stats: ".len()..]).expect("a stats object");
    assert!(
        stats.get("phases").is_some() && stats.get("plans").is_some(),
        "{stats}"
    );
    assert!(out.contains("state: shipped"), "{out}");
}

/// The milestone summary's decisions line, run on a taskboard, prints the phase decision `v1.0/02/D-01`.
#[test]
fn the_milestone_summary_reads_decisions_from_the_graph() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/milestone-summary.md");
    let line = text
        .lines()
        .find(|l| l.contains("--kinds decision,deferred_idea"))
        .expect("the decisions line");
    let start = line.find("specforge query").expect("the query");
    let end = line[start..].find('`').expect("its end") + start;
    let (out, err, code) = sh.run_in(p.root(), &format!("VERSION=1.0\n{}", &line[start..end]));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("v1.0/02/D-01"), "{out}");
}

/// The milestone template's fences create `v1.1` with its first phase on a taskboard and record what it delivered;
/// the retrospective template's fences close a ready milestone with its lists and record a lesson of it.
#[test]
fn the_milestone_templates_record_a_milestone_its_close_and_a_lesson() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let template = asset("gsd-core/templates/milestone.md");
    let create = typed::filled(
        &fence_holding(&template, "specforge new milestone"),
        &[
            ("VERSION", "v1.1"),
            ("NAME", "Sharing"),
            ("GOAL", "Share a board with a link"),
            ("PHASE_NAME", "Links"),
            ("PHASE_GOAL", "A board has a share link"),
        ],
    );
    let done = typed::apply(&sh, &p, &create);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert_eq!(p.fields_of("v1.1")["goal"], "Share a board with a link");
    assert_eq!(p.fields_of("v1.1/01")["name"], "Links");
    let shipped = typed::filled(
        &fence_holding(&template, "whats_next"),
        &[
            ("MILESTONE", "v1.1"),
            ("DELIVERED", "Boards shared by link"),
            ("WHATS_NEXT", "Project complete"),
        ],
    );
    let done = typed::apply(&sh, &p, &shipped);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert_eq!(p.fields_of("v1.1")["delivered"], "Boards shared by link");

    let p = crate::cmd_milestone::ready();
    let retro = asset("gsd-core/templates/retrospective.md");
    let close = typed::filled(
        &fence_holding(&retro, "close-milestone"),
        &[
            ("MILESTONE", "v1.0"),
            ("BUILT", r#"["Core"]"#),
            ("WORKED", r#"["Small plans"]"#),
            ("INEFFICIENT", r#"["Late tests"]"#),
            ("PATTERNS", r#"["One fence per edit"]"#),
        ],
    );
    let done = typed::apply(&sh, &p, &close);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert_eq!(
        p.fields_of("v1.0/v1.0-RETROSPECTIVE")["worked"][0],
        "Small plans"
    );
    let lesson = typed::filled(
        &fence_holding(&retro, "new lesson"),
        &[
            ("MILESTONE", "v1.0"),
            ("LESSON_ID", "L1"),
            ("KIND", "lesson"),
            ("TITLE", "Test first"),
            ("LESSON", "Write the failing test before the fix"),
        ],
    );
    let done = typed::apply(&sh, &p, &lesson);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert_eq!(
        p.fields_of("v1.0/v1.0-RETROSPECTIVE/L1")["text"],
        "Write the failing test before the fix"
    );
}

/// Autonomous keeps a deferred verification in a `## Deferred Verification` section of STATE.md and, on re-entry,
/// parses that section out of `progress-report`'s answer, which never holds it: a deferred phase runs again.
#[test]
fn a_deferred_verification_is_kept_where_progress_never_prints_it() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/autonomous.md");
    let read = super::line(&text, "STATE_CONTENT=$(");
    let p = crate::harness::taskboard();
    let (out, err, code) = sh.run_in(p.root(), &format!("{read}\nprintf '%s' \"$STATE_CONTENT\""));
    assert_eq!(code, 0, "{err}");
    assert!(!out.trim().is_empty(), "{err}");
    assert!(!out.contains("Deferred Verification"), "{out}");
    assert!(!out.contains("phase_complete"), "{out}");
    assert!(text.contains("Append/update this STATE.md section"));
}

/// The two lessons graduation reads in `review.rs`'s window test, on a taskboard copy.
fn with_lessons() -> crate::harness::Project {
    let p = taskboard();
    p.write(
        "spec/learning.spec",
        concat!(
            "retrospective v1.0-RETROSPECTIVE \"Taskboard 1.0 retrospective\" {\n  milestone v1.0\n}\n\n",
            "lesson L2 \"Test the reload\" {\n  retrospective v1.0-RETROSPECTIVE\n  text \"Reload loses ids\"\n}\n\n",
            "lesson L1 \"Plan the reload first\" {\n  retrospective v1.0-RETROSPECTIVE\n  text \"Persistence decides\"\n}\n",
        ),
    );
    p
}

const LESSONS: &str = r#"["v1.0/v1.0-RETROSPECTIVE/L1", "v1.0/v1.0-RETROSPECTIVE/L2"]"#;

/// graduation's X fence, filled for both lessons, dismisses them: Step 4's open filter lists neither on the next run,
/// both read `status` `dismissed`, and the workflow no longer names `graduation_backlog`.
#[test]
fn a_dismissed_cluster_stays_dismissed_on_the_next_run() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = with_lessons();
    let text = asset("gsd-core/workflows/graduation.md");
    assert!(!text.contains("graduation_backlog"));
    let open = fence_holding(&text, "fields.status=open");
    let (out, err, code) = sh.run_in(p.root(), &format!("MILESTONE=v1.0\n{open}"));
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().count(), 2, "{out}");
    let dismiss = typed::filled(
        &fence_holding(&text, "--disposition dismissed"),
        &[("LESSONS", LESSONS)],
    );
    let done = typed::apply(&sh, &p, &dismiss);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    let (out, err, code) = sh.run_in(p.root(), &format!("MILESTONE=v1.0\n{open}"));
    assert_eq!(code, 0, "{err}");
    assert!(out.trim().is_empty(), "{out}");
    for n in [1, 2] {
        let status = &p.fields_of(&format!("v1.0/v1.0-RETROSPECTIVE/L{n}"))["status"];
        assert_eq!(status, "dismissed");
    }
}

/// graduation's P fence, filled, records the cluster as a project decision (origin project, area "Validated
/// Decisions") linked into the project's `key_decisions`, and both lessons read `promoted` with the decision as their
/// target; the check gains no finding.
#[test]
fn a_promoted_cluster_becomes_a_project_decision() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = with_lessons();
    let promote = typed::filled(
        &fence_holding(
            &asset("gsd-core/workflows/graduation.md"),
            "--disposition promoted",
        ),
        &[
            ("DECISION_ID", "D-07"),
            ("AREA", "Validated Decisions"),
            ("LESSONS", LESSONS),
            ("TITLE", "Plan the reload first"),
            (
                "BODY",
                "Persistence decides the data model; reload loses ids unless it is planned.",
            ),
            ("SOURCES", "Sources: Phase 01, Phase 02"),
        ],
    );
    let done = typed::apply(&sh, &p, &promote);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    let decision = p.fields_of("D-07");
    assert_eq!(decision["origin"], "project", "{decision}");
    assert_eq!(decision["area"], "Validated Decisions", "{decision}");
    let keys = p.fields_of("taskboard")["key_decisions"].to_string();
    assert!(keys.contains("D-07"), "{keys}");
    for n in [1, 2] {
        let lesson = p.fields_of(&format!("v1.0/v1.0-RETROSPECTIVE/L{n}"));
        assert_eq!(lesson["status"], "promoted", "{lesson}");
        assert!(lesson["graduated"].to_string().contains("D-07"), "{lesson}");
    }
}

/// The audit's step 2 traces the milestone's requirements along their threads: the trace names each requirement and
/// the plan that claims it.
#[test]
fn the_audit_reads_requirements_through_their_threads() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let code = fence_holding(&audit(), "trace --route gsd/requirement-thread");
    let (out, err, status) = sh.run_in(p.root(), &format!("MILESTONE=v1.0\n{code}"));
    assert_eq!(status, 0, "{err}");
    for id in ["TASK-01", "plan 01-01", "LIST-01", "no plan"] {
        assert!(out.contains(id), "{id}: {out}");
    }
}

/// Step 1 creates the milestone's audit `A1` once (a second run reuses it) and stops, creating nothing, for a milestone
/// that does not exist. Step 3b's record, filled with the basis the rule's inputs give, writes the audit's
/// `integration`, and step 6's fence writes its `tech_debt` and reads the computed `status`; the check gains no
/// finding.
#[test]
fn the_audit_creates_its_entity_once_and_writes_only_tech_debt() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let count = |p: &crate::harness::Project| p.nodes_of("milestone_audit").len();
    let (_, err, code) = sh.run_in(p.root(), &resolve_audit("v9.9"));
    assert_ne!(code, 0, "{err}");
    assert_eq!(count(&p), 0);
    for _ in 0..2 {
        let done = typed::apply(&sh, &p, &resolve_audit("v1.0"));
        assert_eq!(done.code, 0, "{}", done.err);
        assert!(done.out.contains("AUDIT=v1.0/A1"), "{}", done.out);
        assert!(done.added.is_empty(), "{:?}", done.added);
    }
    assert_eq!(count(&p), 1);
    let inputs = p.run(&[
        "query", "v1.0/A1", "--inputs", "gsd/E518", "--format", "json",
    ]);
    let inputs: serde_json::Value = serde_json::from_str(&inputs.stdout).expect("the inputs");
    let basis = inputs["basis"].to_string();
    let record = typed::filled(
        &fence_holding(&audit(), "record-verdict \"$AUDIT\" E518"),
        &[
            ("OUTCOME", "issues"),
            ("BASIS", &basis),
            (
                "FINDINGS",
                r#"[{"severity": "warning", "message": "the list reads no saved file"}]"#,
            ),
        ],
    );
    let done = typed::apply(&sh, &p, &format!("MILESTONE=v1.0\n{record}"));
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert_eq!(p.fields_of("v1.0/A1")["integration"], "issues");
    let debt = typed::filled(
        &fence_holding(&audit(), "tech_debt \"$TECH_DEBT\""),
        &[(
            "TECH_DEBT",
            r#"[{"phase": "v1.0/01", "item": "TODO: add rate limiting"}]"#,
        )],
    );
    let done = typed::apply(&sh, &p, &format!("MILESTONE=v1.0\n{debt}"));
    assert_eq!(done.code, 0, "{}", done.err);
    assert_eq!(done.out.lines().last(), Some("gaps_found"), "{}", done.out);
    assert!(done.added.is_empty(), "{:?}", done.added);
    let fields = p.fields_of("v1.0/A1");
    assert_eq!(fields["tech_debt"][0]["item"], "TODO: add rate limiting");
    assert_eq!(fields["status"], "gaps_found");
}
