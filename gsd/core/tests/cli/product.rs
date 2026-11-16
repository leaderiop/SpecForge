//! The product kinds, end to end (`spec/product.md`): a persona and its journeys on the `roadmap` project, how
//! `specforge` resolves a journey's steps and milestones, and what it refuses.

use crate::harness::{Project, project};

/// A persona and two journeys, written on the `roadmap` project as `spec/product.spec`: `add_and_finish` ships in
/// `v1.0` and steps through TASK-01 and TASK-02; `share_board` steps through and exercises SHARE-01.
pub(crate) const PRODUCT: &str = r#"persona owner "Board owner" {
  context         "Runs a small team's board"
  goals           ["Never lose a task"]
  pain_points     ["Tasks scattered across chats"]
  technical_level beginner
  tags            ["primary"]
}

journey add_and_finish "Add a task and finish it" {
  persona    owner
  channels   ["cli"]
  steps      [
    { action "adds a task with a title" requirement TASK-01 }
    { action "marks it complete" requirement TASK-02 }
    { action "sees the board empty" }
  ]
  milestones [v1.0]
  priority   must
  tags       ["core"]
}

journey share_board "Share the board" {
  persona      owner
  steps        [ { action "copies the board's link" requirement SHARE-01 } ]
  requirements [SHARE-01]
}
"#;

/// The `roadmap` project with [`PRODUCT`] written beside it.
pub(crate) fn product() -> Project {
    let p = project("roadmap");
    p.write("spec/product.spec", PRODUCT);
    p
}

/// `(label, source, target)` of every edge out of `id`, as `specforge query --format json` reports them.
pub(crate) fn edges_from(p: &Project, id: &str) -> Vec<(String, String, String)> {
    let query = p.run(&["query", id, "--format", "json"]);
    let graph: serde_json::Value = serde_json::from_str(&query.stdout)
        .unwrap_or_else(|e| panic!("query is JSON ({e}): {}", query.all()));
    graph["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .filter(|e| e["source"] == id)
        .map(|e| {
            let text = |key: &str| e[key].as_str().unwrap_or_default().to_string();
            (text("label"), text("source"), text("target"))
        })
        .collect()
}

fn edge(label: &str, source: &str, target: &str) -> (String, String, String) {
    (label.into(), source.into(), target.into())
}

#[test]
fn a_step_names_its_requirement() {
    let p = product();
    assert_eq!(p.codes(), Vec::<String>::new());
    let found = edges_from(&p, "add_and_finish");
    for expected in [
        edge("steps.requirement", "add_and_finish", "TASK-01"),
        edge("steps.requirement", "add_and_finish", "TASK-02"),
        edge("milestones", "add_and_finish", "v1.0"),
        edge("persona", "add_and_finish", "owner"),
    ] {
        assert!(found.contains(&expected), "{expected:?} in {found:#?}");
    }
    assert!(
        edges_from(&p, "share_board").contains(&edge(
            "steps.requirement",
            "share_board",
            "SHARE-01"
        )),
        "share_board's step"
    );
}

#[test]
fn a_journey_lists_every_milestone_it_ships_in() {
    let p = product();
    p.write(
        "spec/v2.0.spec",
        "milestone v2.0 \"Taskboard 2.0\" {\n  name \"Taskboard 2.0\"\n}\n",
    );
    p.edit(
        "spec/product.spec",
        "milestones [v1.0]",
        "milestones [v1.0, v2.0]",
    );
    // v2.0 has no phase yet: its planning has not begun, so no gsd/W530 asks it for TASK-01 or TASK-02 (ADR 0357
    // D24); `a_step_outside_the_milestone_is_w530` judges a milestone that has a phase.
    let check = p.run(&["check"]);
    assert_eq!(p.codes(), Vec::<String>::new(), "{}", check.all());
    let mut shipped: Vec<String> = edges_from(&p, "add_and_finish")
        .into_iter()
        .filter(|(label, _, _)| label == "milestones")
        .map(|(_, _, target)| target)
        .collect();
    shipped.sort();
    assert_eq!(shipped, ["v1.0", "v2.0"]);
}

#[test]
fn a_step_naming_an_unknown_requirement_is_e003() {
    let p = product();
    p.edit(
        "spec/product.spec",
        "requirement SHARE-01 }",
        "requirement SHARE-09 }",
    );
    let found = p.report();
    assert_eq!(
        found.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E003"],
        "{found:#?}"
    );
    assert!(found[0].1.contains("SHARE-09"), "{found:#?}");
}

#[test]
fn the_reverse_of_a_journeys_persona_or_milestones_is_e079() {
    for (file, anchor) in [
        ("spec/product.spec", "  technical_level beginner\n"),
        ("spec/v1.0.spec", "  name \"Taskboard 1.0\"\n"),
    ] {
        let p = product();
        p.edit(file, anchor, &format!("{anchor}  journeys [share_board]\n"));
        assert_eq!(p.codes(), ["E079"], "{file}: {:#?}", p.report());
    }
}

#[test]
fn a_journey_is_created_with_its_steps_by_a_typed_edit() {
    let p = product();
    let run = p.run(&[
        "new",
        "journey",
        "checkout",
        "Check out",
        "--persona",
        "owner",
        "--milestones",
        "v1.0",
        "--steps",
        r#"[{"action":"adds a task","requirement":"TASK-01"}]"#,
        "--in",
        "spec/product.spec",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.codes(), Vec::<String>::new());
    assert!(
        edges_from(&p, "checkout").contains(&edge("steps.requirement", "checkout", "TASK-01")),
        "{:#?}",
        edges_from(&p, "checkout")
    );
}

#[test]
fn a_journey_priority_is_must_should_or_could() {
    let p = product();
    p.edit(
        "spec/product.spec",
        "priority   must",
        "priority   critical",
    );
    let found = p.report();
    assert_eq!(
        found.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E061"],
        "{found:#?}"
    );
    for word in ["must", "should", "could"] {
        assert!(found[0].1.contains(word), "{word} in {found:#?}");
    }
}

/// A journey names the requirements it exercises as a whole, an edge apart from its steps' finer ones.
#[test]
fn a_journey_names_the_requirements_it_exercises() {
    let p = product();
    assert_eq!(p.codes(), Vec::<String>::new());
    let found = edges_from(&p, "share_board");
    for expected in [
        edge("requirements", "share_board", "SHARE-01"),
        edge("steps.requirement", "share_board", "SHARE-01"),
    ] {
        assert!(found.contains(&expected), "{expected:?} in {found:#?}");
    }
    p.edit(
        "spec/product.spec",
        "requirements [SHARE-01]",
        "requirements [SHARE-09]",
    );
    assert_eq!(p.codes(), ["E003"], "{:#?}", p.report());
}

/// The kinds `gsd/README.md` says `@specforge/product` declares beside `@gsd/core`: the code spans between
/// `@specforge/product` and `@specforge/governance` in the paragraph that names E026, sorted.
fn kinds_the_readme_names() -> Vec<String> {
    let readme = include_str!("../../../README.md");
    let block = gsd_kit::markdown::inline_blocks(readme)
        .into_iter()
        .find(|b| b.text.contains("E026"))
        .expect("the README's E026 paragraph");
    let spans: Vec<String> = gsd_kit::markdown::inline_code(&block.text)
        .into_iter()
        .map(|(_, span)| span)
        .collect();
    let from = spans
        .iter()
        .rposition(|s| s == "@specforge/product")
        .expect("the paragraph names @specforge/product");
    let mut kinds: Vec<String> = spans[from + 1..]
        .iter()
        .take_while(|s| !s.starts_with('@'))
        .cloned()
        .collect();
    kinds.sort();
    kinds
}

/// Loading `@specforge/product` beside `@gsd/core` registers four kinds twice: E026 on each, the first registration
/// (`@gsd/core`'s) winning, exactly as `gsd/README.md` says.
#[test]
fn specforge_product_beside_gsd_core_is_e026_on_four_kinds() {
    let p = project("empty").with_builtins(&["@specforge/product"]);
    let quoted = regex::Regex::new(r"^entity kind '([a-z_]+)' registered by").expect("a pattern");
    let mut kinds: Vec<String> = p
        .checked()
        .into_iter()
        .filter(|(code, _)| code == "E026")
        .map(|(_, message)| {
            quoted
                .captures(&message)
                .unwrap_or_else(|| panic!("an E026 message names its kind: {message}"))[1]
                .to_string()
        })
        .collect();
    kinds.sort();
    assert_eq!(kinds, ["deliverable", "journey", "milestone", "persona"]);
    assert_eq!(kinds, kinds_the_readme_names());
}

/// A persona and the journey `add_a_task`, one step on TASK-01, for the `verify` and `execute` projects.
const ADD_A_TASK: &str = r#"persona owner "Board owner" {
  context "Runs a small team's board"
}

journey add_a_task "Add a task" {
  persona    owner
  steps      [ { action "adds a task with a title" requirement TASK-01 } ]
  milestones [v1.0]
}
"#;

/// A UAT test that names the journey it walks makes its session the journey's `last_uat`, and the test holds the edge
/// `journey`.
#[test]
fn a_journey_reads_the_uat_session_that_tested_it() {
    let p = project("verify");
    p.write("spec/product.spec", ADD_A_TASK);
    assert_eq!(p.fields_of("add_a_task").get("last_uat"), None);
    p.edit(
        "spec/uat.spec",
        "  expected  \"A task added from the form is listed\"\n",
        "  expected  \"A task added from the form is listed\"\n  journey   add_a_task\n",
    );
    assert_eq!(p.fields_of("add_a_task")["last_uat"], "v1.0/01/01-UAT");
    let found = edges_from(&p, "v1.0/01/01-UAT/U1");
    assert!(
        found.contains(&edge("journey", "v1.0/01/01-UAT/U1", "add_a_task")),
        "{found:#?}"
    );
}

/// The `execute` project, committed, with [`ADD_A_TASK`]: TASK-01 is complete, its task has no commit and no test.
fn execute_with_a_journey() -> Project {
    let p = crate::execute_rules::repository();
    p.write("spec/product.spec", ADD_A_TASK);
    p
}

/// TASK-01 traces: a commit names its task, the git collector records it, and a passing test tagged with the task is
/// collected.
fn trace_task_01(p: &Project) {
    p.git_commit(
        "feat(01-01): Store module",
        &[("src/tasks.js", "a\n")],
        crate::execute_rules::T0,
    );
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    crate::execute_rules::collect_tests(p, &[("[gsd:v1.0/01/01-01/T1] adds a task", true)]);
}

/// A journey is covered once each requirement it names is proven by the requirement thread: TASK-01 is complete
/// before it traces, and the journey is not covered until a commit and a passing test prove it.
#[test]
fn a_journey_is_covered_once_its_requirement_traces() {
    let p = execute_with_a_journey();
    assert_eq!(p.fields_of("TASK-01")["status"], "complete");
    assert_eq!(p.fields_of("add_a_task")["covered"], false);
    trace_task_01(&p);
    assert_eq!(p.fields_of("TASK-01")["proof"], "proven");
    assert_eq!(p.fields_of("add_a_task")["covered"], true);
}

/// The milestone a journey lists counts it, and counts it covered once its requirement traces.
#[test]
fn the_milestone_counts_its_covered_journey() {
    let p = execute_with_a_journey();
    assert_eq!(
        p.fields_of("v1.0")["journeys_covered"],
        serde_json::json!({ "journeys": 1, "covered": 0 })
    );
    trace_task_01(&p);
    assert_eq!(
        p.fields_of("v1.0")["journeys_covered"],
        serde_json::json!({ "journeys": 1, "covered": 1 })
    );
}

/// `add_and_finish` lists v1.0, whose phase 01 holds TASK-01 and TASK-02: no gsd/W530, nor for `share_board`, which
/// lists no milestone. Once `share_board` lists v1.0, SHARE-01 (in no phase) is one W530 on it, a warning under
/// gsd-compat and an error under gsd-strict and gsd-plus.
#[test]
fn a_step_outside_the_milestone_is_w530() {
    let p = product();
    assert!(
        !p.codes().iter().any(|code| code == "gsd/W530"),
        "{:#?}",
        p.report()
    );
    p.edit(
        "spec/product.spec",
        "  requirements [SHARE-01]\n",
        "  requirements [SHARE-01]\n  milestones   [v1.0]\n",
    );
    for (profile, exit) in [("gsd-compat", 0), ("gsd-strict", 1), ("gsd-plus", 1)] {
        p.settings(serde_json::json!({ "profile": profile }));
        assert_eq!(
            p.findings(),
            [(
                "gsd/W530".to_string(),
                "journey 'share_board' lists milestone 'v1.0', but no phase of it lists SHARE-01, named by \
                 step 1 and the journey's requirements"
                    .to_string()
            )],
            "{profile}"
        );
        let check = p.run(&["check"]);
        assert_eq!(check.code, exit, "{profile}: {}", check.all());
    }
}

/// A test whose name tags the journey is listed in its `tests`, and the journey stays uncovered: TASK-01's thread has
/// no commit, and a tagged test is evidence beside the thread, never part of it.
#[test]
fn a_journey_lists_its_tagged_test() {
    let p = execute_with_a_journey();
    crate::execute_rules::collect_tests(&p, &[("[gsd:add_a_task] adds a task", true)]);
    let tests = p.fields_of("add_a_task")["tests"].clone();
    let ids = tests
        .as_array()
        .unwrap_or_else(|| panic!("a list: {tests}"));
    assert_eq!(ids.len(), 1, "{tests}");
    let fact = p.fields_of(ids[0].as_str().expect("an id"));
    assert_eq!(fact["status"], "passed", "{fact}");
    assert_eq!(p.fields_of("add_a_task")["covered"], false);
}
