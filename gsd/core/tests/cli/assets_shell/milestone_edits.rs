//! The milestone family's steps (audit, close, transition, autonomous, graduation, next), run as the assets hold them
//! on a project copy, and the answers they read.

use super::{Shell, asset, block, fence_after};

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
