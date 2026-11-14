//! The product kinds, end to end (`spec/product.md`): a persona and its journeys on the `roadmap` project, how
//! `specforge` resolves a journey's steps and milestones, and what it refuses.

use crate::harness::{Project, project};

/// A persona, two journeys and two terms, written on the `roadmap` project as `spec/product.spec`: `add_and_finish`
/// ships in `v1.0` and steps through TASK-01 and TASK-02; `share_board` steps through and exercises SHARE-01; the term
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
  persona      owner
  steps        [ { action "copies the board's link" requirement SHARE-01 } ]
  requirements [SHARE-01]
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

/// A term's `avoid` holding a word the glossary accepts, its own alias or another term's name, is `gsd/W538` on the
/// avoiding term, a warning under every profile, compared case-insensitively.
#[test]
fn a_term_that_avoids_an_accepted_word_is_w538() {
    for profile in ["gsd-compat", "gsd-strict", "gsd-plus"] {
        let p = product();
        p.settings(serde_json::json!({ "profile": profile }));
        p.edit(
            "spec/product.spec",
            "avoid      [\"ticket\"]",
            "avoid      [\"ticket\", \"To-Do\", \"Board\"]",
        );
        let found = p.findings();
        let codes: Vec<&str> = found.iter().map(|(code, _)| code.as_str()).collect();
        assert_eq!(codes, ["gsd/W538", "gsd/W538"], "{profile}: {found:#?}");
        for expected in [
            "term 'work_item' avoids \"Board\", which is the name of term 'board'",
            "term 'work_item' avoids \"To-Do\", which is its own alias",
        ] {
            assert!(
                found.iter().any(|(_, message)| message.contains(expected)),
                "{profile}: {expected} in {found:#?}"
            );
        }
        let check = p.run(&["check"]);
        assert_eq!(check.code, 0, "{profile}: a warning: {}", check.all());
    }
}

/// The fixture's glossary avoids no word it accepts: no `gsd/W538`.
#[test]
fn a_glossary_that_avoids_no_accepted_word_is_not_w538() {
    let p = product();
    assert!(
        !p.codes().iter().any(|code| code == "gsd/W538"),
        "{:#?}",
        p.report()
    );
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

/// Loading `@specforge/product` beside `@gsd/core` registers five kinds twice: E026 on each, the first registration
/// (`@gsd/core`'s) winning, exactly as `gsd/README.md` says.
#[test]
fn specforge_product_beside_gsd_core_is_e026_on_five_kinds() {
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
    assert_eq!(
        kinds,
        ["deliverable", "journey", "milestone", "persona", "term"]
    );
    assert_eq!(kinds, kinds_the_readme_names());
}
