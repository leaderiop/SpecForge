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

/// The transition's roadmap step sets `TRANSITION` to an empty object, then reads the next phase and whether this was
/// the last one from it: the keys it names are in no answer.
#[test]
fn the_transition_extracts_keys_its_step_never_prints() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/transition.md");
    let step = block(&text, "<step name=\"update_roadmap_and_state\">", "</step>");
    let code = fence_after(&step, "update_roadmap_and_state");
    let p = crate::harness::taskboard();
    let (out, err, _) = sh.run_in(p.root(), &format!("{code}\nprintf '%s' \"$TRANSITION\""));
    assert_eq!(out, "{}", "{err}");
    let keys = extracted(&step);
    assert!(
        keys.iter().any(|k| k == "next_phase") && keys.iter().any(|k| k == "is_last_phase"),
        "{keys:?}"
    );
}

/// The close step reads the milestone's version, date, counts and accomplishments from `close-milestone`'s answer,
/// whose output holds only what it wrote, the completion and `archived`.
#[test]
fn the_close_extracts_keys_close_milestone_never_prints() {
    let p = crate::cmd_milestone::ready();
    let run = p.run(&["gsd", "close-milestone", "v1.0", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let answer: serde_json::Value = serde_json::from_str(&run.stdout).expect("a JSON answer");
    let answer = &answer["output"];
    for key in ["wrote", "completion", "archived"] {
        assert!(answer.get(key).is_some(), "{key}: {answer}");
    }
    let keys = extracted(&asset("gsd-core/workflows/complete-milestone.md"));
    for key in [
        "version",
        "date",
        "phases",
        "plans",
        "tasks",
        "accomplishments",
    ] {
        assert!(keys.iter().any(|k| k == key), "{key}: {keys:?}");
        assert!(answer.get(key).is_none(), "{key}: {answer}");
    }
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

/// Graduation keeps defer and dismiss under `graduation_backlog`, a field no kind declares: a dismissed cluster is
/// offered again on every transition.
#[test]
fn a_graduation_disposition_is_kept_in_a_field_no_kind_declares() {
    let p = crate::harness::taskboard();
    let run = p.run(&["schema"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let schema: serde_json::Value = serde_json::from_str(&run.stdout).expect("a JSON schema");
    let fields: Vec<&str> = schema["entity_kinds"]
        .as_array()
        .expect("kinds")
        .iter()
        .flat_map(|k| k["fields"].as_array().into_iter().flatten())
        .filter_map(|f| f["name"].as_str())
        .collect();
    assert!(
        fields.contains(&"graduated"),
        "the lesson's fields are listed"
    );
    assert!(!fields.contains(&"graduation_backlog"));
    assert!(asset("gsd-core/workflows/graduation.md").contains("under `graduation_backlog`"));
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
