//! When an invariant is due, end to end (`spec/governance.md`, "When an invariant is due"): the twin's invariant
//! judged, then a phase's commit that changes what it guards, recorded by `gsd-git`; `phase.invariants_due`,
//! `gsd/E531` and the writes around them.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;

use crate::clock::utc;
use crate::harness::Project;
use crate::invariants::{TWIN_INVARIANT, author, status};
use crate::parity_thread::{collected_thread, succeeds};
use crate::prompts_support::{payload, prompt, text_of};

/// [`TWIN_INVARIANT`] with its one `holds_from` line naming `holds_from` instead, its indentation kept.
fn invariant_from(holds_from: &str) -> String {
    let lines: Vec<&str> = TWIN_INVARIANT.lines().collect();
    let at: Vec<usize> = (0..lines.len())
        .filter(|&n| lines[n].split_whitespace().next() == Some("holds_from"))
        .collect();
    assert_eq!(at.len(), 1, "one holds_from line in {TWIN_INVARIANT}");
    let line = lines[at[0]];
    let indent = &line[..line.len() - line.trim_start().len()];
    let mut text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    text[at[0]] = format!("{indent}holds_from {holds_from}");
    text.join("\n") + "\n"
}

/// The twin's thread collected, [`TWIN_INVARIANT`] in force from `holds_from` and judged `holds`; then, the same day,
/// a commit of phase 01's plan 01-02 that changes `addTask`, authored once the clock has passed the verdict's second,
/// collected. Answers the project and the commit's full sha.
fn judged_then_changed(holds_from: &str) -> (Project, String) {
    judge_then_change(collected_thread().0, holds_from)
}

/// [`judged_then_changed`] on `p`, a collected thread.
fn judge_then_change(p: Project, holds_from: &str) -> (Project, String) {
    author(&p, &invariant_from(holds_from));
    let run = p.record("ids_are_sequential", "I536", "holds", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    let judged = verdict_at(&p);
    let sha = change_add_task(&p, "01-02/T1", &after(&judged));
    (p, sha)
}

/// The `at` of the invariant's `gsd/I536` verdict as written in `spec/invariants.spec`, read there because the rule
/// reports no judgement on a pending invariant: the second the host recorded it, `YYYY-MM-DDTHH:MM:SSZ`.
pub(super) fn verdict_at(p: &Project) -> String {
    let text = p.read("spec/invariants.spec");
    let at: Vec<&str> = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("at "))
        .map(|at| at.trim().trim_matches('"'))
        .collect();
    assert_eq!(at.len(), 1, "one verdict in {text}");
    assert_eq!(at[0].len(), "2026-11-15T09:00:00Z".len(), "{text}");
    at[0].to_string()
}

/// The time now, `YYYY-MM-DDTHH:MM:SSZ`, once the clock has passed `at`, a second the host recorded: a commit
/// authored then follows it. The wait is the rest of that one second.
fn after(at: &str) -> String {
    loop {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_secs();
        let now = utc(i64::try_from(now).expect("seconds fit"));
        if now.as_str() > at {
            return now;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Commits, authored at `at`, a change of `task` (`01-02/T1`, its `Gsd-Task` trailer) to `addTask`, then collects the
/// items and the commit. Answers the commit's full sha.
fn change_add_task(p: &Project, task: &str, at: &str) -> String {
    let source = p.read("src/tasks.js");
    let changed = source.replace("board.nextId += 1;", "board.nextId = task.id + 1;");
    assert_ne!(changed, source, "the twin's addTask moved");
    let sha = p.git_commit(
        &format!("fix: the next id follows the task\n\nGsd-Task: {task}"),
        &[("src/tasks.js", &changed)],
        at,
    );
    succeeds(&p.collect(&["--runner", "javascript-source"]));
    succeeds(&p.collect(&["--runner", "gsd-git"]));
    sha
}

/// `(code, message)` of each `gsd/E531` `specforge check` reports.
fn e531(p: &Project) -> Vec<(String, String)> {
    p.report_all()
        .into_iter()
        .filter(|(code, _)| code == "gsd/E531")
        .collect()
}

/// The suggestion of the one `gsd/E531` of `specforge check --format json`.
fn e531_suggestion(p: &Project) -> String {
    let run = p.run(&["check", "--format", "json"]);
    let found: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    let e531: Vec<&serde_json::Value> = found
        .as_array()
        .expect("a list")
        .iter()
        .filter(|d| d["code"] == "gsd/E531")
        .collect();
    assert_eq!(e531.len(), 1, "{e531:#?}");
    e531[0]["suggestion"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The invariants a phase owes a judgement, as `specforge query` answers `invariants_due`; none when it is absent.
fn due_in(p: &Project, phase: &str) -> Vec<String> {
    let fields = p.fields_of(phase);
    fields["invariants_due"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|id| id.as_str().expect("an id").to_string())
        .collect()
}

/// On the JavaScript twin no item runs as a test (its source pack writes no `test_id`): a commit that changes only the
/// invariant's proof test, collected with its `items_changed` written, is due by the test's file.
#[test]
fn a_change_to_its_proof_test_on_a_javascript_project_is_due() {
    let (p, _) = collected_thread();
    author(&p, TWIN_INVARIANT);
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    let judged = verdict_at(&p);
    let source = p.read("test/tasks.test.js");
    let changed = source.replace(
        "  assert.equal(addTask(board, \"Plan phase 1\").id, 2);\n",
        "",
    );
    assert_ne!(changed, source, "the twin's proof test moved");
    p.git_commit(
        "test: one id is enough\n\nGsd-Task: 01-02/T1",
        &[("test/tasks.test.js", &changed)],
        &after(&judged),
    );
    succeeds(&p.collect(&["--runner", "javascript-source"]));
    succeeds(&p.collect(&["--runner", "gsd-git"]));
    assert_eq!(due_in(&p, "v1.0/01"), ["ids_are_sequential"]);
    let suggestion = e531_suggestion(&p);
    assert!(
        suggestion.contains("touched test/tasks.test.js"),
        "{suggestion}"
    );
}

/// An item listed by its rooted path (`/js:…`) is the item its reference resolves to: a commit that changes it is due.
#[test]
fn a_change_to_an_item_listed_by_its_rooted_path_is_due() {
    let (p, _) = collected_thread();
    let listed = "\"js:taskboard/src/tasks.js::addTask\"";
    let rooted = TWIN_INVARIANT.replace(listed, "\"/js:taskboard/src/tasks.js::addTask\"");
    assert_ne!(rooted, TWIN_INVARIANT, "the twin's invariant lists addTask");
    author(&p, &rooted);
    let found = p.report_all();
    assert!(
        !found.iter().any(|(code, _)| code == "code/E101"),
        "{found:?}"
    );
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    let judged = verdict_at(&p);
    change_add_task(&p, "01-02/T1", &after(&judged));
    assert_eq!(due_in(&p, "v1.0/01"), ["ids_are_sequential"]);
}

/// Judged, then changed the same day: the commit follows the verdict by its second, so the phase owes a judgement; a
/// verdict recorded after the commit covers it, and the phase owes nothing.
#[test]
fn a_change_the_same_day_is_due_until_the_invariant_is_judged_again() {
    let (p, _) = judged_then_changed("v1.0/01");
    let judged = verdict_at(&p);
    assert_eq!(due_in(&p, "v1.0/01"), ["ids_are_sequential"]);
    assert_eq!(e531(&p).len(), 1);
    let run = p.record("ids_are_sequential", "I536", "holds", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    let again = verdict_at(&p);
    assert!(again > judged, "{again} after {judged}");
    assert_eq!(&again[..10], &judged[..10], "the same day");
    assert_eq!(due_in(&p, "v1.0/01"), Vec::<String>::new());
    assert_eq!(e531(&p), Vec::new());
}

#[test]
fn a_change_after_the_last_judgement_makes_the_invariant_due() {
    let (p, sha) = judged_then_changed("v1.0/01");
    assert_eq!(
        p.fields_of(&sha[..12])["items_changed"],
        json!(["js:taskboard/src/tasks.js::addTask"])
    );
    assert_eq!(due_in(&p, "v1.0/01"), ["ids_are_sequential"]);
    assert_eq!(due_in(&p, "v1.0/02"), Vec::<String>::new());
    assert_eq!(
        e531(&p),
        [(
            "gsd/E531".to_string(),
            "invariant 'ids_are_sequential' is due in phase 'v1.0/01': the phase changed what it guards after it \
             was last judged"
                .to_string()
        )]
    );
    let suggestion = e531_suggestion(&p);
    assert_eq!(
        suggestion,
        format!(
            "judge it again (verify_invariant --invariant ids_are_sequential records its gsd/I536 verdict): \
             commit {} changed \
             js:taskboard/src/tasks.js::addTask",
            &sha[..12]
        )
    );
}

/// A due invariant usually still `holds` (its verdict reads no code it did not cite): the prompt E531 sends the agent
/// to lists it all the same, with the phases that owe it, and judging it again clears them.
#[test]
fn the_verify_invariant_prompt_lists_a_due_invariant_that_holds() {
    let rendered =
        |p: &Project| text_of(&payload(&prompt(p, "verify_invariant", &[], "claude-code")));
    let (p, _) = judged_then_changed("v1.0/01");
    assert_eq!(status(&p, "ids_are_sequential"), "holds");
    let text = rendered(&p);
    assert!(text.contains("- `ids_are_sequential`"), "{text}");
    assert!(
        text.contains(
            "`ids_are_sequential` is due (gsd/E531): `v1.0/01` changed what it guards after its last verdict."
        ),
        "{text}"
    );
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    let text = rendered(&p);
    assert!(text.contains("No invariant awaits a verdict."), "{text}");
}

/// A verification of phase 02, which has not run: recorded, it puts an invariant holding from phase 02 in force.
const PHASE_02_VERIFIED: &str = r#"verification 02-VERIFICATION "Phase 02 verification" {
  phase v1.0/02
}
"#;

/// Phase 02 verified, so an invariant holding from it is in force: the commit of phase 01 is before its window.
#[test]
fn a_change_before_holds_from_makes_nothing_due() {
    let (p, _) = collected_thread();
    p.write("spec/verify-02.spec", PHASE_02_VERIFIED);
    let (p, _) = judge_then_change(p, "v1.0/02");
    assert_eq!(status(&p, "ids_are_sequential"), "holds");
    for phase in ["v1.0/01", "v1.0/02"] {
        assert_eq!(due_in(&p, phase), Vec::<String>::new(), "{phase}");
    }
    assert_eq!(e531(&p), Vec::new());
}

#[test]
fn a_change_judged_since_makes_nothing_due() {
    let (p, _) = collected_thread();
    author(&p, TWIN_INVARIANT);
    // never judged: the thread's commit changed addTask, and nothing is owed (no gsd/E531)
    assert_eq!(due_in(&p, "v1.0/01"), Vec::<String>::new());
    assert_eq!(e531(&p), Vec::new());
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    // the thread's commit (2026-10-28) is older than the verdict
    assert_eq!(due_in(&p, "v1.0/01"), Vec::<String>::new());
    assert_eq!(e531(&p), Vec::new());
}

/// A change to what a pending invariant guards owes it nothing, as it owes `gsd/I536` no verdict yet.
#[test]
fn a_change_to_a_pending_invariant_makes_nothing_due() {
    let (p, _) = judged_then_changed("v1.0/02");
    assert_eq!(status(&p, "ids_are_sequential"), "pending");
    for phase in ["v1.0/01", "v1.0/02"] {
        assert_eq!(due_in(&p, phase), Vec::<String>::new(), "{phase}");
    }
    assert_eq!(e531(&p), Vec::new());
}

/// Judged early, while it is pending (`verify_invariant --invariant`); then phase 02, its `holds_from`, changes what it
/// guards; then the write that brings it into force, phase 02's verification created by `specforge new`, is not
/// refused for the E531 it reveals. E531 then fires, and judging again clears it.
#[test]
fn the_write_that_brings_an_early_judged_invariant_into_force_reveals_its_e531() {
    let (p, _) = collected_thread();
    author(&p, &invariant_from("v1.0/02"));
    assert_eq!(status(&p, "ids_are_sequential"), "pending");
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    let judged = verdict_at(&p);
    change_add_task(&p, "02-01/T1", &after(&judged));
    assert_eq!(due_in(&p, "v1.0/02"), Vec::<String>::new());
    assert_eq!(e531(&p), Vec::new());
    let run = p.run(&[
        "new",
        "verification",
        "02-VERIFICATION",
        "Phase 02 verification",
        "--in",
        "spec/verify-02.spec",
        "--phase",
        "v1.0/02",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!run.all().contains("E080"), "{}", run.all());
    assert_eq!(status(&p, "ids_are_sequential"), "holds");
    assert_eq!(due_in(&p, "v1.0/02"), ["ids_are_sequential"]);
    assert_eq!(e531(&p).len(), 1);
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    assert_eq!(due_in(&p, "v1.0/02"), Vec::<String>::new());
    assert_eq!(e531(&p), Vec::new());
}

/// In force, a write that makes an invariant due is refused (E080): widening a judged invariant's `items` over code a
/// phase changed since its verdict.
#[test]
fn widening_a_judged_invariant_over_changed_code_is_e080() {
    let (p, _) = collected_thread();
    author(
        &p,
        &TWIN_INVARIANT.replace("src/tasks.js::addTask", "src/tasks.js::completeTask"),
    );
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    let judged = verdict_at(&p);
    change_add_task(&p, "01-02/T1", &after(&judged));
    assert_eq!(e531(&p), Vec::new());
    let run = p.run(&[
        "set",
        "ids_are_sequential",
        "items",
        "js:taskboard/src/tasks.js::completeTask",
        "js:taskboard/src/tasks.js::addTask",
    ]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.all().contains("E080") && run.all().contains("gsd/E531"),
        "{}",
        run.all()
    );
}
