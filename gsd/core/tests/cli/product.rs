//! The product kinds, end to end (`spec/product.md`): a persona and its journeys on the `roadmap` project, how
//! `specforge` resolves a journey's steps and milestones, and what it refuses.

use crate::harness::{Project, project};

/// A persona, two journeys and two terms, written on the `roadmap` project as `spec/product.spec`: `add_and_finish`
/// ships in `v1.0` and steps through TASK-01 and TASK-02; `share_board` steps through SHARE-01; the term
/// `work_item`, whose word is "task" (a keyword is no entity id, E013), is related to `board`.
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
  persona owner
  steps   [ { action "copies the board's link" requirement SHARE-01 } ]
}

term work_item "Task" {
  name       "task"
  definition "A unit of work on the board, with a title"
  avoid      ["ticket"]
  aliases    ["to-do"]
  related    [board]
  context    "the board's lists"
  tags       ["core"]
}

term board "Board" {
  name       "board"
  definition "The tasks a team shares"
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
    assert_eq!(p.codes(), Vec::<String>::new());
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

#[test]
fn a_term_names_its_related_terms() {
    let p = product();
    assert_eq!(p.codes(), Vec::<String>::new());
    assert_eq!(
        edges_from(&p, "work_item"),
        [edge("related", "work_item", "board")]
    );
    p.edit(
        "spec/product.spec",
        "related    [board]",
        "related    [nope]",
    );
    let found = p.report();
    assert_eq!(
        found.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E003"],
        "{found:#?}"
    );
    assert!(found[0].1.contains("nope"), "{found:#?}");
}

#[test]
fn a_term_without_its_word_is_e006() {
    let p = product();
    p.edit("spec/product.spec", "  name       \"task\"\n", "");
    let found = p.report();
    assert_eq!(
        found.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
        ["E006"],
        "{found:#?}"
    );
    assert!(found[0].1.contains("name"), "{found:#?}");
}
