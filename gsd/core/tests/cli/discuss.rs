//! The discuss, specify and research kinds, end to end, over `tests/projects/discuss/`.

use crate::harness::{Project, project};

/// `(source, target, label)` of every edge `specforge query <name> --depth 1` prints.
fn edges(p: &Project, name: &str) -> Vec<(String, String, String)> {
    let query = p.run(&["query", name, "--depth", "1", "--format", "json"]);
    let graph: serde_json::Value = serde_json::from_str(&query.stdout)
        .unwrap_or_else(|e| panic!("query {name} is JSON ({e}): {}", query.all()));
    let text = |e: &serde_json::Value, key: &str| e[key].as_str().unwrap_or_default().to_string();
    graph["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|e| (text(e, "source"), text(e, "target"), text(e, "label")))
        .collect()
}

fn has_edge(edges: &[(String, String, String)], source: &str, target: &str, label: &str) -> bool {
    edges.contains(&(source.into(), target.into(), label.into()))
}

/// A spec file of the `discuss` project, to edit and write back.
fn fixture(file: &str) -> String {
    let path = format!(
        "{}/tests/projects/discuss/spec/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn checks_clean(p: &Project) {
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    assert!(
        check.all().contains("0 errors, 0 warnings"),
        "check: {}",
        check.all()
    );
}

#[test]
fn the_discuss_project_checks_clean() {
    checks_clean(&project("discuss"));
}

#[test]
fn a_decision_resolves_in_its_phase_and_from_the_project() {
    let p = project("discuss");
    let own = edges(&p, "v1.0/02/D-02");
    assert!(
        has_edge(&own, "v1.0/02/D-02", "v1.0/02/D-01", "superseded_by"),
        "{own:?}"
    );
    let spec = fixture("discuss.spec");
    p.write(
        "spec/discuss.spec",
        &spec.replace("superseded_by D-01", "superseded_by /D-01"),
    );
    checks_clean(&p);
    let rooted = edges(&p, "v1.0/02/D-02");
    assert!(
        has_edge(&rooted, "v1.0/02/D-02", "D-01", "superseded_by"),
        "{rooted:?}"
    );
}

#[test]
fn two_phases_may_each_have_a_d_01() {
    let p = project("discuss");
    p.write(
        "spec/v1.0.spec",
        &(fixture("v1.0.spec")
            + "\nphase 03 \"Sharing\" {\n  milestone v1.0\n  name \"Sharing\"\n  slug \"sharing\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}\n"),
    );
    p.write(
        "spec/more.spec",
        "decision D-01 \"Share by link\" {\n  phase v1.0/03\n  text \"A board is shared by link\"\n  origin context\n}\n",
    );
    checks_clean(&p);
    for name in ["v1.0/02/D-01", "v1.0/03/D-01", "D-01"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(query.code, 0, "query {name}: {}", query.all());
        assert!(query.all().contains(name), "query {name}: {}", query.all());
    }
}

#[test]
fn a_context_is_named_02_context_in_its_phase() {
    let p = project("discuss");
    for name in ["v1.0/02/02-CONTEXT", "02-CONTEXT"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(query.code, 0, "query {name}: {}", query.all());
        assert!(
            query.all().contains("v1.0/02/02-CONTEXT"),
            "query {name}: {}",
            query.all()
        );
    }
}

#[test]
fn a_canonical_ref_to_a_missing_file_is_e016() {
    let p = project("discuss");
    let spec = fixture("discuss.spec");
    p.write(
        "spec/discuss.spec",
        &spec.replace("docs/storage.md", "docs/missing.md"),
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(
        check.all().contains("E016") && check.all().contains("docs/missing.md"),
        "check: {}",
        check.all()
    );
}

#[test]
fn project_key_decisions_name_project_decisions() {
    let p = project("discuss");
    let own = edges(&p, "taskboard");
    assert!(
        has_edge(&own, "taskboard", "D-01", "key_decisions"),
        "{own:?}"
    );
}
