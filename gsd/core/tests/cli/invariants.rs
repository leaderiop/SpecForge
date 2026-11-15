//! The invariant kind, end to end (`spec/governance.md`): an invariant authored over the taskboard twin, its proof and
//! items read through `@code/core`'s facts, and its verdict, `gsd/I536`, with the prompt that judges it.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use specforge_extension_sdk::prelude::TestStatus;

use crate::clock::utc;
use crate::commands_support::plus;
use crate::harness::{Project, is_judged, project, taskboard};
use crate::judged::TWIN_TESTS;
use crate::parity_thread::{collected_thread, succeeds};
use crate::prompts_support::{payload, prompt, render_three_hosts, text_of};

/// One invariant over the taskboard twin, in force from phase 01 (executed): what `addTask` keeps, the test that
/// proves it and the code item it holds over.
pub(super) const TWIN_INVARIANT: &str = r#"invariant ids_are_sequential "Task ids are sequential" {
  statement   "A new task takes the next id"
  holds_from  v1.0/01
  proof       ["js:taskboard/test/tasks.test.js#a task is added with the next id"]
  items       ["js:taskboard/src/tasks.js::addTask"]
  criticality high
}
"#;

/// Writes `text` as the project's `spec/invariants.spec`.
pub(super) fn author(p: &Project, text: &str) {
    p.write("spec/invariants.spec", text);
}

/// Collects the twin's three tests and its source items, so a proof or an item reference resolves or is code/E101.
pub(super) fn collect_facts(p: &Project) {
    for run in [
        p.node_tests(&TWIN_TESTS),
        p.collect(&["--runner", "javascript-source"]),
    ] {
        assert_eq!(run.code, 0, "{}", run.all());
    }
}

/// `(code, severity, message)` of every diagnostic `specforge check` reports on the entity `id`, the severity
/// lower-cased, in report order. A check names it in `data.entity`; a judged rule's finding carries no data, and names
/// it as the quoted id of its message.
pub(super) fn on(p: &Project, id: &str) -> Vec<(String, String, String)> {
    let run = p.run(&["check", "--format", "json"]);
    let found: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    let text = |d: &serde_json::Value, key: &str| d[key].as_str().unwrap_or_default().to_string();
    found
        .as_array()
        .expect("a list")
        .iter()
        .filter(|d| {
            d["data"]["entity"] == id
                || (is_judged(&text(d, "code"))
                    && text(d, "message").split('\'').nth(1) == Some(id))
        })
        .map(|d| {
            (
                text(d, "code"),
                text(d, "severity").to_lowercase(),
                text(d, "message"),
            )
        })
        .collect()
}

/// The diagnostic [`on`] reports on `ids_are_sequential` before any verdict: `gsd/I536` at Info.
fn no_verdict() -> (String, String, String) {
    (
        "gsd/I536".to_string(),
        "info".to_string(),
        "invariant 'ids_are_sequential' has no fresh verdict that it holds: no verdict".to_string(),
    )
}

/// [`on`] without the invariant's own verdict (`gsd/I536`, asserted once): what the checks of its fields report.
fn checks_on(p: &Project, id: &str) -> Vec<(String, String, String)> {
    let (verdict, checks): (Vec<_>, Vec<_>) =
        on(p, id).into_iter().partition(|d| d.0 == "gsd/I536");
    assert_eq!(verdict.len(), 1, "{verdict:#?}");
    checks
}

#[test]
fn an_invariant_is_authored_with_its_proof_and_items() {
    let p = taskboard();
    author(&p, TWIN_INVARIANT);
    p.check_exact(&[]);
    collect_facts(&p);
    assert_eq!(on(&p, "ids_are_sequential"), [no_verdict()]);
    let fields = p.fields_of("ids_are_sequential");
    assert_eq!(
        fields["statement"], "A new task takes the next id",
        "{fields}"
    );
    assert_eq!(fields["holds_from"], "v1.0/01", "{fields}");
    assert_eq!(
        fields["proof"],
        serde_json::json!(["js:taskboard/test/tasks.test.js#a task is added with the next id"]),
        "{fields}"
    );
    assert_eq!(
        fields["items"],
        serde_json::json!(["js:taskboard/src/tasks.js::addTask"]),
        "{fields}"
    );
    assert_eq!(fields["criticality"], "high", "{fields}");
}

#[test]
fn an_in_force_invariants_missing_test_is_code_e101() {
    let p = taskboard();
    author(
        &p,
        &TWIN_INVARIANT.replace("#a task is added with the next id", "#no such test"),
    );
    collect_facts(&p);
    let found = checks_on(&p, "ids_are_sequential");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "code/E101", "{found:#?}");
    assert!(found[0].2.contains("#no such test"), "{found:#?}");
}

/// The status of an entity, as `specforge query` answers it.
pub(super) fn status(p: &Project, id: &str) -> String {
    p.fields_of(id)["status"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn a_pending_invariants_proof_is_not_checked() {
    let p = taskboard();
    let missing = TWIN_INVARIANT.replace("#a task is added with the next id", "#no such test");
    author(&p, &missing.replace("v1.0/01", "v1.0/02"));
    collect_facts(&p);
    let phase = status(&p, "v1.0/02");
    assert!(
        !["executed", "needs_review", "complete"].contains(&phase.as_str()),
        "phase 02 is not complete: {phase}"
    );
    assert_eq!(status(&p, "ids_are_sequential"), "pending");
    // nor is its verdict owed: I536 waits for it to be in force
    assert_eq!(on(&p, "ids_are_sequential"), Vec::new());
    let phase = status(&p, "v1.0/01");
    assert!(
        ["executed", "needs_review", "complete"].contains(&phase.as_str()),
        "phase 01 is complete: {phase}"
    );
    author(&p, &missing);
    assert_eq!(status(&p, "ids_are_sequential"), "unverified");
    let found = checks_on(&p, "ids_are_sequential");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "code/E101", "{found:#?}");
}

/// The twin's invariant in force and a second one pending until phase 02 completes.
const WITH_A_PENDING_ONE: &str = r#"invariant ids_stay_unique "Task ids stay unique" {
  statement  "No two tasks share an id"
  holds_from v1.0/02
}
"#;

/// `verify_invariant` is our own text, rendered for each host with no shell-ids line and no attribution; its tail
/// names the invariant given, else every invariant in force that does not hold, else says none awaits a verdict.
#[test]
fn the_verify_invariant_prompt_lists_the_invariants_awaiting_a_verdict() {
    let p = taskboard();
    author(&p, &format!("{TWIN_INVARIANT}\n{WITH_A_PENDING_ONE}"));
    assert_eq!(status(&p, "ids_stay_unique"), "pending");
    let heading = "## The invariants to judge";
    for (host, rendered) in
        render_three_hosts(&p, "verify_invariant", &[], "(no arguments)", heading)
    {
        let text = text_of(&rendered);
        for word in [
            "specforge gsd record-verdict",
            "--inputs gsd/I536",
            "- `ids_are_sequential`",
        ] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
        for word in ["`ids_stay_unique`", "In the shell blocks above"] {
            assert!(!text.contains(word), "{host}: '{word}'");
        }
        assert!(
            rendered["payload"]
                .get("attribution")
                .is_none_or(|a| a.is_null()),
            "{host}: {}",
            rendered["payload"]
        );
    }
    let given = ["--invariant", "ids_stay_unique"];
    let text = text_of(&payload(&prompt(
        &p,
        "verify_invariant",
        &given,
        "claude-code",
    )));
    assert!(text.contains("- `ids_stay_unique`"), "{text}");
    assert!(
        text.contains("`ids_stay_unique` is pending until its `holds_from` phase has run"),
        "{text}"
    );
    assert!(!text.contains("`ids_are_sequential`"), "{text}");
    let plain = taskboard();
    let text = text_of(&payload(&prompt(
        &plain,
        "verify_invariant",
        &[],
        "claude-code",
    )));
    assert!(text.contains("No invariant awaits a verdict."), "{text}");
    assert!(!text.contains(heading), "{text}");
}

/// The twin with [`TWIN_INVARIANT`] and its facts collected: in force, no verdict yet.
fn authored() -> Project {
    let p = taskboard();
    author(&p, TWIN_INVARIANT);
    collect_facts(&p);
    p
}

#[test]
fn an_invariant_without_a_fresh_holds_verdict_is_i536() {
    for profile in ["gsd-compat", "gsd-strict", "gsd-plus"] {
        let p = authored();
        p.settings(json!({ "profile": profile }));
        assert_eq!(on(&p, "ids_are_sequential"), [no_verdict()], "{profile}");
    }
}

#[test]
fn a_holds_verdict_sets_status_and_last_verified() {
    let p = authored();
    assert_eq!(status(&p, "ids_are_sequential"), "unverified");
    let run = p.record("ids_are_sequential", "I536", "holds", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(status(&p, "ids_are_sequential"), "holds");
    let at = p.inputs("ids_are_sequential", "I536")["verdict"]["at"].clone();
    assert!(at.is_string(), "{at}");
    assert_eq!(p.fields_of("ids_are_sequential")["last_verified"], at);
    assert_eq!(on(&p, "ids_are_sequential"), Vec::new());
}

#[test]
fn a_violated_verdict_stays_info() {
    let p = authored();
    let run = p.record("ids_are_sequential", "I536", "violated", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(status(&p, "ids_are_sequential"), "violated");
    assert!(p.fields_of("ids_are_sequential")["last_verified"].is_null());
    let found = on(&p, "ids_are_sequential");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        (found[0].0.as_str(), found[0].1.as_str()),
        ("gsd/I536", "info")
    );
}

/// A test whose name tags the invariant is one of its `tests`: its new result stales the verdict.
#[test]
fn a_new_test_result_stales_the_verdict() {
    let p = authored();
    let tagged = (
        "test/tasks.test.js",
        "ids stay sequential [gsd:ids_are_sequential]",
        TestStatus::Passed,
    );
    let mut cases = TWIN_TESTS.to_vec();
    cases.push(tagged);
    let run = p.node_tests(&cases);
    assert_eq!(run.code, 0, "{}", run.all());
    let run = p.record("ids_are_sequential", "I536", "holds", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(status(&p, "ids_are_sequential"), "holds");
    *cases.last_mut().expect("the tagged test") = (tagged.0, tagged.1, TestStatus::Failed);
    let run = p.node_tests(&cases);
    assert_eq!(run.code, 0, "{}", run.all());
    let inputs = p.inputs("ids_are_sequential", "I536");
    assert_eq!(inputs["verdict"]["state"], "stale", "{inputs}");
    assert!(
        inputs["verdict"]["changed"].to_string().contains("tests"),
        "{inputs}"
    );
    assert_eq!(status(&p, "ids_are_sequential"), "stale");
    assert!(p.fields_of("ids_are_sequential")["last_verified"].is_null());
}

/// Under `gsd-plus` a stale verdict anywhere refuses a scoped gate (gsd/E516): a stale I536 refuses `plan_phase`.
#[test]
fn a_stale_i536_refuses_a_gate_under_gsd_plus() {
    let p = authored();
    let tagged = (
        "test/tasks.test.js",
        "ids stay sequential [gsd:ids_are_sequential]",
    );
    let mut cases = TWIN_TESTS.to_vec();
    cases.push((tagged.0, tagged.1, TestStatus::Passed));
    assert_eq!(p.node_tests(&cases).code, 0);
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    *cases.last_mut().expect("the tagged test") = (tagged.0, tagged.1, TestStatus::Failed);
    assert_eq!(p.node_tests(&cases).code, 0);
    assert_eq!(status(&p, "ids_are_sequential"), "stale");
    plus(&p);
    let run = prompt(&p, "plan_phase", &["v1.0/02"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E516"), "{}", run.all());
    // a fresh verdict over the new result lifts it
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    let rendered = payload(&prompt(&p, "plan_phase", &["v1.0/02"], "claude-code"));
    assert!(
        rendered["payload"]["prompt"]
            .as_str()
            .is_some_and(|s| s.ends_with(":plan_phase")),
        "{rendered}"
    );
}

/// [`TWIN_INVARIANT`] pending until phase 02 completes, the only invariant of the twin.
fn pending_only() -> Project {
    let p = taskboard();
    author(&p, &TWIN_INVARIANT.replace("v1.0/01", "v1.0/02"));
    assert_eq!(status(&p, "ids_are_sequential"), "pending");
    p
}

/// A pending invariant owes no verdict yet: the prompt that judges invariants does not ask for one, and the rule
/// does not report one missing.
#[test]
fn a_pending_invariant_is_asked_for_no_verdict() {
    let p = pending_only();
    let text = text_of(&payload(&prompt(
        &p,
        "verify_invariant",
        &[],
        "claude-code",
    )));
    assert!(
        text.contains(
            "No invariant in force awaits a verdict. 1 invariant is pending until its `holds_from` phase has \
             run: it owes no verdict yet."
        ),
        "{text}"
    );
    assert!(!text.contains("`ids_are_sequential`"), "{text}");
    assert_eq!(on(&p, "ids_are_sequential"), Vec::new());
    assert!(p.inputs("ids_are_sequential", "I536")["verdict"].is_null());
    // judged early on request, and told so
    let given = ["--invariant", "ids_are_sequential"];
    let text = text_of(&payload(&prompt(
        &p,
        "verify_invariant",
        &given,
        "claude-code",
    )));
    assert!(text.contains("- `ids_are_sequential`"), "{text}");
    assert!(
        text.contains("`ids_are_sequential` is pending until its `holds_from` phase has run"),
        "{text}"
    );
}

/// A verdict recorded early on a pending invariant (`verify_invariant --invariant`) goes stale with its tests; the
/// invariant owes no verdict yet, so no gate is refused for it.
#[test]
fn a_stale_verdict_on_a_pending_invariant_refuses_no_gate() {
    let p = pending_only();
    let tagged = (
        "test/tasks.test.js",
        "ids stay sequential [gsd:ids_are_sequential]",
    );
    let mut cases = TWIN_TESTS.to_vec();
    cases.push((tagged.0, tagged.1, TestStatus::Passed));
    assert_eq!(p.node_tests(&cases).code, 0);
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    *cases.last_mut().expect("the tagged test") = (tagged.0, tagged.1, TestStatus::Failed);
    assert_eq!(p.node_tests(&cases).code, 0);
    assert_eq!(status(&p, "ids_are_sequential"), "pending");
    plus(&p);
    let rendered = payload(&prompt(&p, "plan_phase", &["v1.0/02"], "claude-code"));
    assert!(
        rendered["payload"]["prompt"]
            .as_str()
            .is_some_and(|s| s.ends_with(":plan_phase")),
        "{rendered}"
    );
    // in force from phase 01, the verdict is owed, stale, and refuses the gate
    let text = p.read("spec/invariants.spec");
    author(
        &p,
        &text.replace("holds_from  v1.0/02", "holds_from  v1.0/01"),
    );
    let inputs = p.inputs("ids_are_sequential", "I536");
    assert_eq!(inputs["verdict"]["state"], "stale", "{inputs}");
    let run = prompt(&p, "plan_phase", &["v1.0/02"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E516"), "{}", run.all());
}

/// Without `@code/core` the `test` kind is not loaded: both chains of I536's `tests` input are inert, the rule is
/// registered (no W112) and a verdict is recorded fresh.
#[test]
fn i536_without_code_core_says_no_w112() {
    let p = project("execute");
    author(
        &p,
        r#"invariant ids_are_sequential "Task ids are sequential" {
  statement "A new task takes the next id"
  proof     ["js:taskboard/test/tasks.test.js#a task is added with the next id"]
}
"#,
    );
    let codes: Vec<String> = p.checked().into_iter().map(|(code, _)| code).collect();
    assert!(!codes.iter().any(|c| c == "W112"), "{codes:?}");
    assert!(codes.iter().any(|c| c == "gsd/I536"), "{codes:?}");
    let run = p.record("ids_are_sequential", "I536", "holds", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        p.inputs("ids_are_sequential", "I536")["verdict"]["state"],
        "fresh"
    );
    assert_eq!(status(&p, "ids_are_sequential"), "holds");
}

/// A risk the twin's invariant controls: rpn 175, post_rpn 35 once mitigated.
const RISK_ON_THE_INVARIANT: &str = r#"risk RK1 "A new task takes a used id" {
  severity     7
  occurrence   5
  detection    5
  post {
    occurrence 1
  }
  mitigated_by [ids_are_sequential]
}
"#;

/// `status` and `blocking` of the risk `RK1`.
fn risk(p: &Project) -> (String, bool) {
    let fields = p.fields_of("RK1");
    (
        fields["status"].as_str().unwrap_or_default().to_string(),
        fields["blocking"]
            .as_bool()
            .unwrap_or_else(|| panic!("no blocking in {fields}")),
    )
}

/// GSD's gap closure (`plan-phase N --gaps`): a plan added to a phase its verification found gaps in, not run yet.
const GAP_CLOSURE_PLAN: &str = r#"plan 01-03 "Close the gaps of phase 01" {
  phase     v1.0/01
  type      execute
  objective "Completing an unknown id fails with a one-line message."
}
"#;

/// Once its phase was executed and verified, an invariant stays in force: a gap-closure plan that moves the phase
/// back to `in_progress` hides no verdict and unmitigates no risk.
#[test]
fn an_invariant_stays_in_force_when_its_phase_gains_a_gap_closure_plan() {
    let p = authored();
    p.write("spec/risks.spec", RISK_ON_THE_INVARIANT);
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    assert_eq!(status(&p, "ids_are_sequential"), "holds");
    assert_eq!(risk(&p), ("mitigated".into(), false));
    let run = p.record("v1.0/01/01-02/TR2", "E501", "failed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        p.fields_of("v1.0/01/01-VERIFICATION")["status"],
        "gaps_found"
    );
    assert_eq!(status(&p, "v1.0/01"), "executed");
    assert_eq!(status(&p, "ids_are_sequential"), "holds");
    p.write("spec/gaps.spec", GAP_CLOSURE_PLAN);
    assert_eq!(status(&p, "v1.0/01"), "in_progress");
    assert_eq!(status(&p, "ids_are_sequential"), "holds");
    assert_eq!(risk(&p), ("mitigated".into(), false));
    // in force, its proof is still checked
    p.write(
        "spec/invariants.spec",
        &TWIN_INVARIANT.replace("#a task is added with the next id", "#no such test"),
    );
    let found = checks_on(&p, "ids_are_sequential");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "code/E101", "{found:#?}");
}

/// An invariant mitigates a risk while its status is `holds`: not before a verdict, not once violated, and not while
/// it is pending, whatever its verdict says.
#[test]
fn a_risk_mitigated_by_an_invariant_counts_it_while_it_holds() {
    let p = authored();
    p.write("spec/risks.spec", RISK_ON_THE_INVARIANT);
    assert_eq!(risk(&p), ("mitigating".into(), true), "no verdict yet");
    assert_eq!(p.record("ids_are_sequential", "I536", "holds", &[]).code, 0);
    assert_eq!(risk(&p), ("mitigated".into(), false), "post_rpn 35");
    assert_eq!(
        p.record("ids_are_sequential", "I536", "violated", &[]).code,
        0
    );
    assert_eq!(risk(&p), ("mitigating".into(), true), "violated");

    let pending = taskboard();
    author(&pending, &TWIN_INVARIANT.replace("v1.0/01", "v1.0/02"));
    pending.write("spec/risks.spec", RISK_ON_THE_INVARIANT);
    let run = pending.record("ids_are_sequential", "I536", "holds", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(status(&pending, "ids_are_sequential"), "pending");
    assert_eq!(risk(&pending), ("mitigating".into(), true), "pending");
}

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

/// The twin's thread collected, [`TWIN_INVARIANT`] in force from `holds_from` and judged `holds`; then a commit of
/// phase 01's plan 01-02 that changes `addTask`, dated a day after now so it follows the verdict (it orders, nothing is
/// timed), collected. Answers the project and the commit's full sha.
fn judged_then_changed(holds_from: &str) -> (Project, String) {
    judge_then_change(collected_thread().0, holds_from)
}

/// [`judged_then_changed`] on `p`, a collected thread.
fn judge_then_change(p: Project, holds_from: &str) -> (Project, String) {
    author(&p, &invariant_from(holds_from));
    let run = p.record("ids_are_sequential", "I536", "holds", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    let source = p.read("src/tasks.js");
    let changed = source.replace("board.nextId += 1;", "board.nextId = task.id + 1;");
    assert_ne!(changed, source, "the twin's addTask moved");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("a clock after 1970")
        .as_secs();
    let tomorrow = utc(i64::try_from(now).expect("seconds fit") + 86_400);
    let sha = p.git_commit(
        "fix: the next id follows the task\n\nGsd-Task: 01-02/T1",
        &[("src/tasks.js", &changed)],
        &tomorrow,
    );
    succeeds(&p.collect(&["--runner", "javascript-source"]));
    succeeds(&p.collect(&["--runner", "gsd-git"]));
    (p, sha)
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
            "judge it again (the verify_invariant prompt records its gsd/I536 verdict): commit {} changed \
             js:taskboard/src/tasks.js::addTask",
            &sha[..12]
        )
    );
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
