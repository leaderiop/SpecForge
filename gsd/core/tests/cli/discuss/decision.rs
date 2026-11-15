//! A decision's own relations: what it supersedes, what it affects, the alternatives it weighed and its record
//! fields (ADR 0359).

use super::{checks_clean, edges, fixture, has_edge};
use crate::harness::{Project, judged, project};

/// The report's errors and warnings: kernel and `gsd/` codes alike, the judged rules' among them unless each is one
/// [`Project::expected_judged`] names. An info (an invariant's pending `gsd/I536`) is no finding.
fn findings(p: &Project) -> Vec<(String, String)> {
    let report = p.report_all();
    assert_eq!(judged(&report), p.expected_judged(&[]), "{report:#?}");
    p.report()
        .into_iter()
        .filter(|(code, _)| {
            let local = code.strip_prefix("gsd/").unwrap_or(code);
            local.starts_with('E') || local.starts_with('W')
        })
        .collect()
}

/// An invariant, with the one field it must write, and decision `D-03` of phase 02 that affects it and TASK-01.
const AFFECTS: &str = "invariant tasks_never_lost \"Tasks are never lost\" {
  statement \"A task added is still there after a restart\"
}

decision D-03 \"Save after every change\" {
  phase   v1.0/02
  area    \"Storage\"
  text    \"The file is saved after every change\"
  origin  context
  affects [TASK-01, tasks_never_lost]
}
";

/// The later decision names the one it reverses, in its phase or from the project: `supersedes [D-01]` resolves in
/// the phase's scope, `[/D-01]` from the project root.
#[test]
fn a_superseded_decision_resolves_in_its_phase_and_from_the_project() {
    let p = project("discuss");
    let own = edges(&p, "v1.0/02/D-02");
    assert!(
        has_edge(&own, "v1.0/02/D-02", "v1.0/02/D-01", "supersedes"),
        "{own:?}"
    );
    checks_clean(&p);
    let spec = fixture("discuss.spec");
    p.write(
        "spec/discuss.spec",
        &spec.replace("supersedes [D-01]", "supersedes [/D-01]"),
    );
    checks_clean(&p);
    let rooted = edges(&p, "v1.0/02/D-02");
    assert!(
        has_edge(&rooted, "v1.0/02/D-02", "D-01", "supersedes"),
        "{rooted:?}"
    );
}

/// `superseded_by` is read back, never written: on a decision it is E079 naming `decision.supersedes`, with the edit.
#[test]
fn an_authored_superseded_by_is_e079_naming_supersedes() {
    let p = project("discuss");
    p.edit(
        "spec/discuss.spec",
        "  supersedes [D-01]\n",
        "  superseded_by D-01\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "{}", check.all());
    let report = p.report();
    assert!(
        report
            .iter()
            .any(|(code, message)| code == "E079" && message.contains("decision.supersedes")),
        "{report:#?}"
    );
    assert!(report.iter().all(|(code, _)| code != "W020"), "{report:#?}");
    assert!(check.all().contains("supersedes ["), "{}", check.all());
}

/// A typed edit of `superseded_by` is refused `other_side`, naming `supersedes`; the owning side writes.
#[test]
fn a_typed_edit_of_superseded_by_names_supersedes() {
    let p = project("discuss");
    for verb in ["set", "link"] {
        let refused = p.run(&[verb, "v1.0/02/D-01", "superseded_by", "v1.0/02/D-02"]);
        assert_ne!(refused.code, 0, "{verb}: {}", refused.all());
        assert!(
            refused.all().contains("other_side") && refused.all().contains("supersedes"),
            "{verb}: {}",
            refused.all()
        );
    }
    let linked = p.run(&["link", "v1.0/02/D-02", "supersedes", "/D-01"]);
    assert_eq!(linked.code, 0, "{}", linked.all());
    let own = edges(&p, "v1.0/02/D-02");
    assert!(
        has_edge(&own, "v1.0/02/D-02", "D-01", "supersedes"),
        "{own:?}"
    );
    checks_clean(&p);
}

/// A decision affects a requirement and an invariant: both resolve, each reads it back, and nothing is found.
#[test]
fn a_decision_affects_a_requirement_and_an_invariant() {
    let p = project("discuss");
    p.write("spec/affects.spec", AFFECTS);
    assert_eq!(findings(&p), Vec::new());
    let own = edges(&p, "v1.0/02/D-03");
    for target in ["TASK-01", "tasks_never_lost"] {
        assert!(
            has_edge(&own, "v1.0/02/D-03", target, "affects"),
            "{target}: {own:?}"
        );
        let back = edges(&p, target);
        assert!(
            has_edge(&back, "v1.0/02/D-03", target, "affects"),
            "{target} reads it back: {back:?}"
        );
    }
}

/// `affected_by` is read back, never written: on a requirement it is E079 naming `decision.affects`, and a typed edit
/// of it is refused `other_side`.
#[test]
fn an_authored_affected_by_is_e079_naming_affects() {
    let p = project("discuss");
    p.write("spec/affects.spec", AFFECTS);
    p.edit(
        "spec/project.spec",
        "  text \"A user can add a task with a title\"\n",
        "  text \"A user can add a task with a title\"\n  affected_by [v1.0/02/D-03]\n",
    );
    let report = p.report();
    assert!(
        report
            .iter()
            .any(|(code, message)| code == "E079" && message.contains("decision.affects")),
        "{report:#?}"
    );
    assert!(report.iter().all(|(code, _)| code != "W020"), "{report:#?}");

    let p = project("discuss");
    p.write("spec/affects.spec", AFFECTS);
    let refused = p.run(&["link", "TASK-01", "affected_by", "v1.0/02/D-03"]);
    assert_ne!(refused.code, 0, "{}", refused.all());
    assert!(
        refused.all().contains("other_side") && refused.all().contains("affects"),
        "{}",
        refused.all()
    );
}

/// `specforge new decision --alternatives` takes the records as one JSON value; each keeps why it lost, and an
/// alternative with no name is refused, leaving the file as it was.
#[test]
fn an_alternative_keeps_why_it_lost() {
    let p = project("discuss");
    let new = |alternatives: &str| {
        p.run(&[
            "new",
            "decision",
            "D-04",
            "One JSON file",
            "--in",
            "spec/discuss.spec",
            "--phase",
            "v1.0/02",
            "--origin",
            "context",
            "--text",
            "One JSON file",
            "--alternatives",
            alternatives,
        ])
    };
    let before = p.read("spec/discuss.spec");
    let nameless = new(r#"[{"pros": ["queries"], "why_not": "No dependency in v1"}]"#);
    assert_ne!(nameless.code, 0, "{}", nameless.all());
    assert_eq!(
        p.read("spec/discuss.spec"),
        before,
        "a refused edit writes nothing"
    );

    let written = new(
        r#"[{"name": "SQLite", "pros": ["queries"], "cons": ["a dependency"], "why_not": "No dependency in v1"}]"#,
    );
    assert_eq!(written.code, 0, "{}", written.all());
    let alternatives = &p.fields_of("v1.0/02/D-04")["alternatives"];
    assert_eq!(alternatives[0]["name"], "SQLite", "{alternatives}");
    assert_eq!(
        alternatives[0]["why_not"], "No dependency in v1",
        "{alternatives}"
    );
    assert_eq!(alternatives[0]["cons"][0], "a dependency", "{alternatives}");
    checks_clean(&p);
}
