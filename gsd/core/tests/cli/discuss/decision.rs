//! A decision's own relations: what it supersedes, what it affects, the alternatives it weighed and its record
//! fields (ADR 0359).

use super::{checks_clean, edges, fixture, has_edge};
use crate::harness::project;

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
