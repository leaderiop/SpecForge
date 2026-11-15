use serde_json::{Value, json};

use super::*;
use crate::passes::thread::fixture::add_test;

const X: &str = "ids_are_sequential";
const ITEM_A: &str = "js:app/src/a.js::A";
const ITEM_B: &str = "js:app/src/b.js::B";
/// When [`X`] is judged, unless a test says otherwise.
const JUDGED: &str = "2026-11-01T00:00:00Z";
const BEFORE: &str = "2026-10-30T00:00:00Z";
const AFTER: &str = "2026-11-02T00:00:00Z";

/// Milestones `v1.0` and `v2.0`, their phases (one plan each, `<phase>/p`) and a quick plan `Q-1`; invariants and
/// commits are added to it.
struct Fixture {
    entities: Vec<Value>,
    edges: Vec<Value>,
}

const PHASES: [&str; 7] = [
    "v1.0/01",
    "v1.0/02",
    "v1.0/02.1",
    "v1.0/03",
    "v1.0/polish",
    "v1.0/999.1",
    "v2.0/01",
];

impl Fixture {
    fn new() -> Self {
        let mut f = Fixture {
            entities: vec![json!({ "id": "Q-1", "kind": "plan" })],
            edges: Vec::new(),
        };
        for phase in PHASES {
            f.entities.push(json!({ "id": phase, "kind": "phase" }));
            f.entities
                .push(json!({ "id": format!("{phase}/p"), "kind": "plan" }));
            f.edge(&format!("{phase}/p"), phase, "phase");
        }
        f
    }

    fn edge(&mut self, source: &str, target: &str, label: &str) {
        self.edges
            .push(json!({ "source": source, "target": target, "label": label }));
    }

    /// The invariant `id`, in force (`unverified`), listing `items`.
    fn invariant(&mut self, id: &str, items: &[&str]) -> &mut Value {
        self.entities.push(json!({
            "id": id, "kind": "invariant",
            "fields": { "statement": "it holds", "status": "unverified", "items": items.join(", ") },
            "values": { "items": items },
        }));
        self.entities.last_mut().expect("the invariant")
    }

    /// [`X`] over [`ITEM_A`], judged `holds` (fresh) at [`JUDGED`].
    fn judged(&mut self) -> &mut Value {
        let x = self.invariant(X, &[ITEM_A]);
        judge(x, "fresh", Some("holds"), Some(JUDGED));
        x
    }

    /// [`X`]'s `holds_from` (written and resolved) and, when given, its `scope`.
    fn holds_from(&mut self, phase: &str, scope: Option<&str>) {
        let x = self.entities.iter_mut().find(|e| e["id"] == X).expect("X");
        x["fields"]["holds_from"] = phase.into();
        if let Some(scope) = scope {
            x["fields"]["scope"] = scope.into();
        }
        self.edge(X, phase, "holds_from");
    }

    /// A commit `id` of `plan` at `at` that touched `files`; `items` is its `items_changed`, `None` when unwritten.
    fn commit(&mut self, id: &str, plan: &str, at: &str, items: Option<&[&str]>, files: &[&str]) {
        let mut commit = json!({
            "id": id, "kind": "commit",
            "fields": { "at": at, "plan": plan, "files": files.join(", ") },
            "values": { "files": files },
        });
        if let Some(items) = items {
            commit["fields"]["items_changed"] = items.join(", ").into();
            commit["values"]["items_changed"] = json!(items);
        }
        self.entities.push(commit);
        self.edge(id, plan, "plan");
    }

    /// A commit of `phase`'s plan, as [`Fixture::commit`].
    fn in_phase(&mut self, id: &str, phase: &str, at: &str, items: &[&str]) {
        self.commit(id, &format!("{phase}/p"), at, Some(items), &[]);
    }

    fn input(&self) -> PassInput {
        serde_json::from_value(json!({ "entities": self.entities, "edges": self.edges }))
            .expect("a pass input")
    }

    fn rows(&self) -> Vec<Due> {
        due(&self.input())
    }
}

/// Sets `invariant`'s `gsd/I536` judgement as the host reports it.
fn judge(invariant: &mut Value, state: &str, outcome: Option<&str>, at: Option<&str>) {
    invariant["judgements"] = json!([{
        "rule": "gsd/I536", "state": state, "passed": state == "fresh" && outcome == Some("holds"),
        "outcome": outcome, "at": at,
    }]);
}

/// `(phase, commit, item, file)` of each row on [`X`].
fn on_x(rows: &[Due]) -> Vec<(&str, &str, &str, Option<&str>)> {
    rows.iter()
        .filter(|r| r.invariant == X)
        .map(|r| {
            (
                r.phase.as_str(),
                r.commit.as_str(),
                r.item.as_str(),
                r.file.as_deref(),
            )
        })
        .collect()
}

/// The commits of the rows on [`X`].
fn commits_due(f: &Fixture) -> Vec<String> {
    let mut found: Vec<String> = f
        .rows()
        .into_iter()
        .filter(|r| r.invariant == X)
        .map(|r| r.commit)
        .collect();
    found.dedup();
    found
}

#[test]
fn an_invariant_never_judged_owes_nothing() {
    let mut f = Fixture::new();
    f.invariant(X, &[ITEM_A]);
    f.in_phase("c1", "v1.0/01", AFTER, &[ITEM_A]);
    assert_eq!(f.rows(), []);
    let x = f.entities.iter_mut().find(|e| e["id"] == X).expect("X");
    judge(x, "missing", None, None);
    assert_eq!(f.rows(), []);
}

/// A pending invariant owes no judgement, as `gsd/I536` owes it no verdict: a verdict recorded early is kept, and no
/// commit is due on it until it is in force.
#[test]
fn a_pending_invariant_owes_nothing_whatever_its_verdict() {
    let mut f = Fixture::new();
    f.judged()["fields"]["status"] = "pending".into();
    f.in_phase("c1", "v1.0/01", AFTER, &[ITEM_A]);
    assert_eq!(f.rows(), []);
    for status in INVARIANT_IN_FORCE {
        let x = f.entities.iter_mut().find(|e| e["id"] == X).expect("X");
        x["fields"]["status"] = (*status).into();
        assert_eq!(commits_due(&f), ["c1"], "{status}");
    }
    let x = f.entities.iter_mut().find(|e| e["id"] == X).expect("X");
    x["fields"]
        .as_object_mut()
        .expect("fields")
        .remove("status");
    assert_eq!(f.rows(), [], "no status answered");
}

#[test]
fn a_commit_after_the_last_judgement_that_changed_a_listed_item_is_due() {
    let mut f = Fixture::new();
    f.judged();
    f.in_phase("c1", "v1.0/01", AFTER, &[ITEM_A, ITEM_B]);
    assert_eq!(
        f.rows(),
        [Due {
            invariant: X.into(),
            phase: "v1.0/01".into(),
            commit: "c1".into(),
            item: ITEM_A.into(),
            file: None,
        }]
    );
}

#[test]
fn a_judgement_at_or_after_the_commit_covers_it_whatever_its_outcome_or_freshness() {
    for (state, outcome) in [
        ("fresh", "holds"),
        ("stale", "holds"),
        ("fresh", "violated"),
        ("stale", "uncertain"),
    ] {
        let mut f = Fixture::new();
        judge(f.judged(), state, Some(outcome), Some(JUDGED));
        f.in_phase("before", "v1.0/01", BEFORE, &[ITEM_A]);
        f.in_phase("same", "v1.0/01", JUDGED, &[ITEM_A]);
        f.in_phase("after", "v1.0/01", AFTER, &[ITEM_A]);
        assert_eq!(commits_due(&f), ["after"], "{state} {outcome}");
    }
}

/// A verdict records the second it was given: a commit authored a second later the same day is due.
#[test]
fn a_judgement_covers_no_later_commit_of_its_day() {
    let mut f = Fixture::new();
    judge(
        f.judged(),
        "fresh",
        Some("holds"),
        Some("2026-11-01T09:00:00Z"),
    );
    f.in_phase("that_second", "v1.0/01", "2026-11-01T09:00:00Z", &[ITEM_A]);
    f.in_phase(
        "a_second_later",
        "v1.0/01",
        "2026-11-01T09:00:01Z",
        &[ITEM_A],
    );
    assert_eq!(commits_due(&f), ["a_second_later"]);
}

/// A verdict whose `at` is a day (written by hand) covers every commit of that day, and none of the next.
#[test]
fn a_judgement_of_a_day_covers_that_whole_day() {
    let mut f = Fixture::new();
    judge(f.judged(), "fresh", Some("holds"), Some("2026-11-01"));
    f.in_phase("that_morning", "v1.0/01", "2026-11-01T00:00:00Z", &[ITEM_A]);
    f.in_phase("that_night", "v1.0/01", "2026-11-01T23:59:59Z", &[ITEM_A]);
    f.in_phase("next_day", "v1.0/01", "2026-11-02T00:00:00Z", &[ITEM_A]);
    assert_eq!(commits_due(&f), ["next_day"]);
}

#[test]
fn an_unrecorded_or_unreadable_verdict_is_no_judgement() {
    for (state, at) in [
        ("unrecorded", Some(JUDGED)),
        ("unreadable", Some(JUDGED)),
        ("missing", None),
        ("fresh", None),
        ("fresh", Some("yesterday")),
    ] {
        let mut f = Fixture::new();
        judge(f.judged(), state, Some("holds"), at);
        f.in_phase("c1", "v1.0/01", AFTER, &[ITEM_A]);
        assert_eq!(f.rows(), [], "{state} {at:?}");
    }
}

/// The test function items and the item that names `X` are guarded; an item that names another invariant is not, and
/// neither is a test's own function when the test is linked to another invariant.
#[test]
fn the_items_its_proof_and_naming_tests_run_as_and_the_items_naming_it_are_guarded() {
    let mut f = Fixture::new();
    f.judged();
    f.invariant("other", &[]);
    f.edge(X, "js:app/test/a.test.js#proves", "proof");
    let items = [
        (
            "js:app/test/a.test.js::proves",
            "js:app/test/a.test.js#proves",
        ),
        (
            "js:app/test/a.test.js::tagged",
            "js:app/test/a.test.js#tagged",
        ),
        (
            "js:app/test/a.test.js::elsewhere",
            "js:app/test/a.test.js#elsewhere",
        ),
    ];
    for (item, test) in items {
        f.entities
            .push(json!({ "id": item, "kind": "item", "fields": { "file": "test/a.test.js" } }));
        f.edge(item, test, RUNS_AS);
    }
    for (item, names) in [("js:app/src/c.js::C", X), ("js:app/src/d.js::D", "other")] {
        f.entities
            .push(json!({ "id": item, "kind": "item", "fields": { "file": "src/c.js" } }));
        f.edge(item, names, NAMES);
    }
    let mut input = f.input();
    // the proof test has never run: no status
    input.entities.push(
        serde_json::from_value(
            json!({ "id": "js:app/test/a.test.js#proves", "kind": "test",
            "fields": { "name": "proves", "file": "test/a.test.js" } }),
        )
        .expect("a test fact"),
    );
    add_test(
        &mut input,
        "js:app/test/a.test.js#tagged",
        "tagged",
        "test/a.test.js",
        TestStatus::Passed,
        &[X],
    );
    add_test(
        &mut input,
        "js:app/test/a.test.js#elsewhere",
        "elsewhere",
        "test/a.test.js",
        TestStatus::Passed,
        &["other"],
    );
    let x = input.entities.iter().find(|e| e.id == X).expect("X");
    let guard = guarded(&input, &Code::of(&input), x);
    assert_eq!(
        guard.items.iter().map(String::as_str).collect::<Vec<_>>(),
        [
            ITEM_A,
            "js:app/src/c.js::C",
            "js:app/test/a.test.js::proves",
            "js:app/test/a.test.js::tagged",
        ]
    );
    // the files of the guarded facts: an item's, and the tests' (the first guarded id by id order)
    assert_eq!(
        guard.files,
        BTreeMap::from([
            ("src/c.js".to_string(), "js:app/src/c.js::C".to_string()),
            (
                "test/a.test.js".to_string(),
                "js:app/test/a.test.js#proves".to_string()
            ),
        ])
    );
}

#[test]
fn a_commit_whose_items_changed_is_unwritten_is_matched_by_the_files_it_touched() {
    let mut f = Fixture::new();
    f.judged();
    f.edge(X, "js:app/test/a.test.js#proves", "proof");
    f.entities
        .push(json!({ "id": ITEM_A, "kind": "item", "fields": { "file": "src/a.js" } }));
    f.commit(
        "c1",
        "v1.0/01/p",
        AFTER,
        None,
        &["README.md", "test/a.test.js", "src/a.js"],
    );
    let mut input = f.input();
    add_test(
        &mut input,
        "js:app/test/a.test.js#proves",
        "proves",
        "test/a.test.js",
        TestStatus::Passed,
        &[],
    );
    assert_eq!(
        on_x(&due(&input)),
        [
            ("v1.0/01", "c1", ITEM_A, Some("src/a.js")),
            (
                "v1.0/01",
                "c1",
                "js:app/test/a.test.js#proves",
                Some("test/a.test.js")
            ),
        ]
    );
}

#[test]
fn a_written_empty_items_changed_changes_nothing() {
    let mut f = Fixture::new();
    f.judged();
    f.entities
        .push(json!({ "id": ITEM_A, "kind": "item", "fields": { "file": "src/a.js" } }));
    f.commit("c1", "v1.0/01/p", AFTER, Some(&[]), &["src/a.js"]);
    assert_eq!(f.rows(), []);
}

#[test]
fn holds_from_bounds_the_window_by_phase_number_in_its_milestone() {
    let mut f = Fixture::new();
    f.judged();
    f.holds_from("v1.0/02", None);
    for (commit, phase) in [
        ("c01", "v1.0/01"),
        ("c02", "v1.0/02"),
        ("c021", "v1.0/02.1"),
        ("c03", "v1.0/03"),
    ] {
        f.in_phase(commit, phase, AFTER, &[ITEM_A]);
    }
    assert_eq!(commits_due(&f), ["c02", "c021", "c03"]);
}

#[test]
fn scope_milestone_keeps_the_window_in_holds_froms_milestone() {
    let mut f = Fixture::new();
    f.judged();
    f.in_phase("first", "v1.0/02", "2026-10-01T00:00:00Z", &[]);
    f.in_phase("v1", "v1.0/03", AFTER, &[ITEM_A]);
    f.in_phase("v2", "v2.0/01", AFTER, &[ITEM_A]);
    let mut project = Fixture {
        entities: f.entities.clone(),
        edges: f.edges.clone(),
    };
    f.holds_from("v1.0/02", Some("milestone"));
    assert_eq!(commits_due(&f), ["v1"]);
    project.holds_from("v1.0/02", Some("project"));
    assert_eq!(commits_due(&project), ["v1", "v2"]);
    // with no holds_from it has no milestone to keep: every phase's commit counts
    let mut unbound = Fixture::new();
    unbound.judged()["fields"]["scope"] = "milestone".into();
    unbound.in_phase("v1", "v1.0/01", AFTER, &[ITEM_A]);
    unbound.in_phase("v2", "v2.0/01", AFTER, &[ITEM_A]);
    assert_eq!(commits_due(&unbound), ["v1", "v2"]);
}

#[test]
fn scope_project_counts_another_milestones_commit_from_holds_froms_first_commit() {
    let start = "2026-11-03T00:00:00Z";
    let mut f = Fixture::new();
    f.judged();
    f.holds_from("v1.0/02", None);
    f.in_phase("older", "v2.0/01", AFTER, &[ITEM_A]);
    f.in_phase("first", "v1.0/02", start, &[]);
    f.in_phase("equal", "v2.0/01", start, &[ITEM_A]);
    f.in_phase("newer", "v2.0/01", "2026-11-04T00:00:00Z", &[ITEM_A]);
    assert_eq!(commits_due(&f), ["equal", "newer"]);
    // the holds_from phase has no recorded commit: no other milestone's commit is placed after it
    let mut none = Fixture::new();
    none.judged();
    none.holds_from("v1.0/02", None);
    none.in_phase("newer", "v2.0/01", "2026-11-04T00:00:00Z", &[ITEM_A]);
    assert_eq!(commits_due(&none), Vec::<String>::new());
}

/// A custom-named phase and a backlog `999.x` have no number the roadmap orders: a commit of either counts from the
/// `holds_from` phase's first recorded commit, as another milestone's does.
#[test]
fn a_custom_named_phase_is_ordered_by_holds_froms_first_commit() {
    let start = "2026-11-03T00:00:00Z";
    for phase in ["v1.0/polish", "v1.0/999.1"] {
        let mut f = Fixture::new();
        f.judged();
        f.holds_from("v1.0/02", None);
        f.in_phase("older", phase, AFTER, &[ITEM_A]);
        f.in_phase("first", "v1.0/02", start, &[]);
        f.in_phase("newer", phase, "2026-11-04T00:00:00Z", &[ITEM_A]);
        assert_eq!(commits_due(&f), ["newer"], "{phase}");
    }
    // and from a custom-named holds_from, a numbered phase is ordered the same way
    let mut f = Fixture::new();
    f.judged();
    f.holds_from("v1.0/polish", None);
    f.in_phase("older", "v1.0/03", AFTER, &[ITEM_A]);
    f.in_phase("first", "v1.0/polish", start, &[]);
    f.in_phase("newer", "v1.0/01", "2026-11-04T00:00:00Z", &[ITEM_A]);
    assert_eq!(commits_due(&f), ["newer"]);
}

#[test]
fn a_quick_plans_commit_is_not_judged() {
    let mut f = Fixture::new();
    f.judged();
    f.commit("quick", "Q-1", AFTER, Some(&[ITEM_A]), &[]);
    f.in_phase("phased", "v1.0/01", AFTER, &[ITEM_A]);
    assert_eq!(commits_due(&f), ["phased"]);
}

#[test]
fn invariants_due_names_each_due_invariant_once_per_phase() {
    let mut f = Fixture::new();
    f.judged();
    let other = f.invariant("another", &[ITEM_B]);
    judge(other, "stale", Some("holds"), Some(JUDGED));
    f.in_phase("c1", "v1.0/01", AFTER, &[ITEM_A, ITEM_B]);
    f.in_phase("c2", "v1.0/01", "2026-11-05T00:00:00Z", &[ITEM_A]);
    f.in_phase("c3", "v1.0/03", AFTER, &[ITEM_B]);
    f.in_phase("c4", "v1.0/02", AFTER, &[]);
    let values: Vec<(String, String, Value)> = invariants_due(&f.rows())
        .into_iter()
        .map(|v| (v.entity, v.field, v.value))
        .collect();
    assert_eq!(
        values,
        [
            (
                "v1.0/01".to_string(),
                "invariants_due".to_string(),
                json!(["another", X])
            ),
            (
                "v1.0/03".to_string(),
                "invariants_due".to_string(),
                json!(["another"])
            ),
        ]
    );
}

/// `(code, severity, entity, message, suggestion)` of a diagnostic.
type Reported = (String, PassSeverity, Option<String>, String, Option<String>);

/// Each diagnostic the pass reports, as [`Reported`].
fn reported(f: &Fixture) -> Vec<Reported> {
    compute(&f.input())
        .diagnostics
        .into_iter()
        .map(|d| (d.code, d.severity, d.entity, d.message, d.suggestion))
        .collect()
}

const JUDGE_IT: &str =
    "judge it again (the verify_invariant prompt records its gsd/I536 verdict): ";

fn due_message(phase: &str) -> String {
    format!(
        "invariant '{X}' is due in phase '{phase}': the phase changed what it guards after it was last judged"
    )
}

#[test]
fn e531_is_one_error_per_invariant_and_phase_naming_its_commits_and_items() {
    let mut f = Fixture::new();
    f.invariant(X, &[ITEM_A, ITEM_B]);
    judge(
        f.entities.last_mut().expect("X"),
        "fresh",
        Some("holds"),
        Some(JUDGED),
    );
    f.entities
        .push(json!({ "id": ITEM_A, "kind": "item", "fields": { "file": "src/a.js" } }));
    f.in_phase("c1", "v1.0/01", AFTER, &[ITEM_B, ITEM_A]);
    f.commit(
        "c2",
        "v1.0/01/p",
        "2026-11-03T00:00:00Z",
        None,
        &["src/a.js"],
    );
    f.in_phase("c3", "v1.0/03", "2026-11-04T00:00:00Z", &[ITEM_B]);
    let error = |phase: &str, suggestion: String| {
        (
            "E531".to_string(),
            PassSeverity::Error,
            Some(X.to_string()),
            due_message(phase),
            Some(format!("{JUDGE_IT}{suggestion}")),
        )
    };
    assert_eq!(
        reported(&f),
        [
            error(
                "v1.0/01",
                format!(
                    "commit c1 changed {ITEM_A}, {ITEM_B}; commit c2 touched src/a.js ({ITEM_A})"
                )
            ),
            error("v1.0/03", format!("commit c3 changed {ITEM_B}")),
        ]
    );
}

/// The message names the invariant and the phase only: one more row in a phase already due lengthens the suggestion
/// and leaves the message, which E080 compares, as it was.
#[test]
fn the_e531_message_stays_put_when_its_rows_change() {
    let mut f = Fixture::new();
    f.judged();
    f.in_phase("c1", "v1.0/01", AFTER, &[ITEM_A]);
    let before = reported(&f);
    f.in_phase("c2", "v1.0/01", "2026-11-03T00:00:00Z", &[ITEM_A]);
    let after = reported(&f);
    assert_eq!((before.len(), after.len()), (1, 1));
    assert_eq!(before[0].3, due_message("v1.0/01"));
    assert_eq!(after[0].3, before[0].3);
    assert_eq!(
        after[0].4.as_deref(),
        Some(format!("{JUDGE_IT}commit c1 changed {ITEM_A}; commit c2 changed {ITEM_A}").as_str())
    );
    assert!(after[0].4.as_ref().map(String::len) > before[0].4.as_ref().map(String::len));
}
