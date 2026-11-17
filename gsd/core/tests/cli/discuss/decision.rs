//! A decision's own relation: what it supersedes (ADR 0359).

use super::{checks_clean, edges, fixture, has_edge};
use crate::harness::project;

/// The later decision names the one it reverses, in its phase or from the project: D-01 reverses D-02 (written on
/// D-01 as ADR 0359's conversion row writes a `superseded_by D-01` held on D-02), and `supersedes [D-02]` resolves in
/// the phase's scope, `[/D-01]` from the project root.
#[test]
fn a_superseded_decision_resolves_in_its_phase_and_from_the_project() {
    let p = project("discuss");
    let own = edges(&p, "v1.0/02/D-01");
    assert!(
        has_edge(&own, "v1.0/02/D-01", "v1.0/02/D-02", "supersedes"),
        "{own:?}"
    );
    checks_clean(&p);
    let spec = fixture("discuss.spec");
    p.write(
        "spec/discuss.spec",
        &spec.replace("supersedes [D-02]", "supersedes [/D-01]"),
    );
    checks_clean(&p);
    let rooted = edges(&p, "v1.0/02/D-01");
    assert!(
        has_edge(&rooted, "v1.0/02/D-01", "D-01", "supersedes"),
        "{rooted:?}"
    );
}

/// A decision superseded with force no longer holds: its computed `in_force` is false and its successor's true, and
/// the planner's pack serves both with it, as gsd/E310 counts them (no selector filters on a computed field).
#[test]
fn a_superseded_decision_is_not_in_force_in_the_planners_pack() {
    let p = project("discuss");
    assert_eq!(p.fields_of("v1.0/02/D-02")["in_force"], false);
    assert_eq!(p.fields_of("v1.0/02/D-01")["in_force"], true);
    let run = p.run(&["context", "gsd/planner", "v1.0/02"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let pack: serde_json::Value = serde_json::from_str(&run.stdout).expect("a pack");
    let in_force = |id: &str| {
        pack["graph"]["nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|n| n["id"] == id))
            .map(|n| n["fields"]["in_force"].clone())
    };
    assert_eq!(in_force("v1.0/02/D-02"), Some(false.into()), "{pack}");
    assert_eq!(in_force("v1.0/02/D-01"), Some(true.into()), "{pack}");
}

/// `superseded_by` is read back, never written: the fixture's relation written the old way, on D-02, is E079 naming
/// `decision.supersedes`, with the edit.
#[test]
fn an_authored_superseded_by_is_e079_naming_supersedes() {
    let p = project("discuss");
    let written = "  text   \"The file is written to a temp name, then renamed\"\n";
    let spec = fixture("discuss.spec");
    assert!(spec.contains(written), "{spec}");
    p.write(
        "spec/discuss.spec",
        &spec
            .replace("  supersedes [D-02]\n", "")
            .replace(written, &format!("{written}  superseded_by D-01\n")),
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

/// `new decision` writes a rooted id of a reference list as `link` writes it, spelled from the new decision's own
/// scope chain: `/D-01` from a phase decision keeps its slash (the phase's own D-01 would answer `D-01`).
#[test]
fn new_writes_a_rooted_reference_as_link_does() {
    let p = project("discuss");
    let new = p.run(&[
        "new",
        "decision",
        "D-09",
        "Nine",
        "--in",
        "spec/discuss.spec",
        "--phase",
        "v1.0/02",
        "--origin",
        "context",
        "--text",
        "Nine",
        "--supersedes",
        "/D-01",
    ]);
    assert_eq!(new.code, 0, "{}", new.all());
    let linked = p.run(&["link", "v1.0/02/D-02", "supersedes", "/D-01"]);
    assert_eq!(linked.code, 0, "{}", linked.all());
    let spec = p.read("spec/discuss.spec");
    let block = |id: &str| -> Vec<String> {
        let start = spec.find(&format!("decision {id} ")).expect("the block");
        spec[start..]
            .lines()
            .take_while(|line| *line != "}")
            .filter(|line| line.contains("supersedes"))
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    };
    assert_eq!(block("D-09"), ["supersedes [/D-01]"]);
    assert_eq!(block("D-09"), block("D-02"));
    let own = edges(&p, "v1.0/02/D-09");
    assert!(
        has_edge(&own, "v1.0/02/D-09", "D-01", "supersedes"),
        "{own:?}"
    );
}

/// A typed edit of `superseded_by` is refused `other_side`, naming `supersedes`; the owning side writes.
#[test]
fn a_typed_edit_of_superseded_by_names_supersedes() {
    let p = project("discuss");
    for verb in ["set", "link"] {
        let refused = p.run(&[verb, "v1.0/02/D-02", "superseded_by", "v1.0/02/D-01"]);
        assert_ne!(refused.code, 0, "{verb}: {}", refused.all());
        assert!(
            refused.all().contains("other_side") && refused.all().contains("supersedes"),
            "{verb}: {}",
            refused.all()
        );
    }
    let linked = p.run(&["link", "v1.0/02/D-01", "supersedes", "/D-01"]);
    assert_eq!(linked.code, 0, "{}", linked.all());
    let own = edges(&p, "v1.0/02/D-01");
    for target in ["v1.0/02/D-02", "D-01"] {
        assert!(
            has_edge(&own, "v1.0/02/D-01", target, "supersedes"),
            "{target}: {own:?}"
        );
    }
    checks_clean(&p);
}
