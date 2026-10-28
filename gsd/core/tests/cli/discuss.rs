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

fn codes(p: &Project) -> Vec<String> {
    let run = p.run(&["check", "--format", "json"]);
    let found: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    found
        .as_array()
        .expect("a list")
        .iter()
        .filter_map(|d| d["code"].as_str().map(String::from))
        .filter(|c| c != "I204")
        .collect()
}

/// The project with its baseline committed and `edit` applied to `spec/discuss.spec`.
fn edited(from: &str, to: &str) -> Project {
    let p = project("discuss");
    p.commit();
    let spec = fixture("discuss.spec");
    assert!(spec.contains(from), "the fixture holds `{from}`");
    p.write("spec/discuss.spec", &spec.replace(from, to));
    p
}

#[test]
fn a_rewritten_interview_answer_is_e084() {
    let p = edited("In one JSON file", "In SQLite");
    assert_eq!(codes(&p), ["E084"]);
}

#[test]
fn a_removed_interview_is_e085() {
    let p = edited(
        "interview IV1 \"Where are tasks kept?\" {\n  spec        v1.0/02/02-SPEC\n  round       1\n  perspective researcher\n  question    \"Where are tasks kept?\"\n  answer      \"In one JSON file\"\n}\n",
        "",
    );
    assert_eq!(codes(&p), ["E085"]);
}

#[test]
fn an_interview_answer_may_be_set_once() {
    let p = edited(
        "question    \"What if the file is corrupt?\"\n",
        "question    \"What if the file is corrupt?\"\n  answer      \"Start empty and say so\"\n",
    );
    assert_eq!(codes(&p), Vec::<String>::new());
    p.commit();
    let spec = fixture("discuss.spec");
    p.write(
        "spec/discuss.spec",
        &spec.replace(
            "question    \"What if the file is corrupt?\"\n",
            "question    \"What if the file is corrupt?\"\n  answer      \"Refuse to start\"\n",
        ),
    );
    assert_eq!(codes(&p), ["E084"]);
}

#[test]
fn a_score_above_100_is_refused() {
    let p = project("discuss");
    let spec = fixture("discuss.spec");
    let scored = |ambiguity: &str| {
        spec.replace(
            "goal    \"Tasks go from memory-only to saved on disk\"\n",
            &format!("goal    \"Tasks go from memory-only to saved on disk\"\n  scores  {{ goal 90 boundary 80 constraint 70 acceptance 80 ambiguity {ambiguity} }}\n"),
        )
    };
    p.write("spec/discuss.spec", &scored("10"));
    assert_eq!(codes(&p), Vec::<String>::new());
    p.write("spec/discuss.spec", &scored("101"));
    assert_eq!(codes(&p).len(), 1, "{:?}", codes(&p));
}

const FIRST_ENTRY: &str = "    {\n      area     \"Storage\"\n      question \"Where are tasks kept?\"\n      options  [\"One JSON file\", \"SQLite\"]\n      chosen   \"One JSON file\"\n      mode     interactive\n    }\n";
const SECOND_ENTRY: &str = "    {\n      area     \"Safety\"\n      question \"How is the file written?\"\n      options  [\"In place\", \"Atomically\"]\n      chosen   \"Atomically\"\n      mode     interactive\n    }\n";

#[test]
fn a_removed_log_entry_is_refused() {
    let p = edited(FIRST_ENTRY, "");
    assert_eq!(codes(&p), ["E085"]);
    let run = p.run(&["check"]);
    assert!(
        run.all().contains("entries"),
        "names the field: {}",
        run.all()
    );
}

#[test]
fn a_rewritten_log_entry_is_refused() {
    let p = edited("chosen   \"Atomically\"", "chosen   \"In place\"");
    assert_eq!(codes(&p), ["E085"]);
}

#[test]
fn an_appended_log_entry_is_clean() {
    let p = edited(
        SECOND_ENTRY,
        &format!(
            "{SECOND_ENTRY}    {{ area \"Safety\" question \"Keep a backup?\" options [\"Yes\", \"No\"] chosen \"No\" mode advisor }}\n"
        ),
    );
    assert_eq!(codes(&p), Vec::<String>::new());
}

fn queried(p: &Project, name: &str) -> String {
    let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
    assert_eq!(query.code, 0, "query {name}: {}", query.all());
    query.all()
}

#[test]
fn research_items_are_named_within_their_research() {
    let p = project("discuss");
    checks_clean(&p);
    // The phase research's `F1` and the project research's `F1` coexist.
    assert!(queried(&p, "v1.0/02/02-RESEARCH/F1").contains("v1.0/02/02-RESEARCH/F1"));
    assert!(queried(&p, "RESEARCH/F1").contains("RESEARCH/F1"));
    for item in ["SRC1", "SC1", "P1", "PF1", "DH1", "OQ1", "PA1"] {
        let name = format!("v1.0/02/02-RESEARCH/{item}");
        assert!(queried(&p, &name).contains(&name), "{name}");
    }
}

#[test]
fn a_project_research_is_named_research_in_the_project_scope() {
    let p = project("discuss");
    assert!(queried(&p, "RESEARCH").contains("RESEARCH"));
    let own = edges(&p, "RESEARCH/F1");
    assert!(
        has_edge(&own, "RESEARCH/F1", "RESEARCH", "research"),
        "{own:?}"
    );
}

#[test]
fn an_open_question_names_the_decision_that_resolves_it() {
    let own = edges(&project("discuss"), "v1.0/02/02-RESEARCH/OQ1");
    assert!(
        has_edge(
            &own,
            "v1.0/02/02-RESEARCH/OQ1",
            "v1.0/02/D-02",
            "resolved_by"
        ),
        "{own:?}"
    );
}

#[test]
fn an_unknown_legitimacy_is_refused() {
    let p = project("discuss");
    let spec = std::fs::read_to_string(format!(
        "{}/tests/projects/discuss/spec/research.spec",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the fixture");
    p.write(
        "spec/research.spec",
        &spec.replace("legitimate\n", "trusted\n"),
    );
    assert_eq!(codes(&p), ["E061"]);
}

fn with_canonical_refs(refs: Option<&str>) -> Project {
    let p = project("discuss");
    let spec = fixture("discuss.spec");
    let field = "  canonical_refs [\n    { topic \"Storage\" paths [\"docs/storage.md\"] }\n  ]\n";
    assert!(spec.contains(field), "the fixture holds the refs");
    let replacement = refs.map_or_else(String::new, |r| format!("  canonical_refs {r}\n"));
    p.write("spec/discuss.spec", &spec.replace(field, &replacement));
    p
}

#[test]
fn context_without_refs_is_e201() {
    let found = with_canonical_refs(None).findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/E201");
    assert!(found[0].1.contains("no canonical_refs"), "{found:#?}");
}

#[test]
fn context_with_empty_refs_is_e201() {
    let found = with_canonical_refs(Some("[]")).findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/E201");
    assert!(found[0].1.contains("empty canonical_refs"), "{found:#?}");
}

#[test]
fn a_context_that_says_no_external_specs_is_clean() {
    let p = with_canonical_refs(Some(
        "[\n    { topic \"No external specs — requirements are fully captured in decisions above\" paths [] }\n  ]",
    ));
    checks_clean(&p);
}
