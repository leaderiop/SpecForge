//! The roadmap rules, end to end: which codes `specforge check` reports on the `roadmap_errors` project, and that
//! the clean `roadmap` project reports none.

use crate::harness::project;

#[test]
fn roadmap_errors_report_each_code_once() {
    let p = project("roadmap_errors");
    let found = p.findings();
    let codes: Vec<&str> = found.iter().map(|(code, _)| code.as_str()).collect();
    assert_eq!(
        codes,
        [
            "gsd/E101", "gsd/E101", "gsd/E102", "gsd/E102", "gsd/E102", "gsd/E105", "gsd/W104"
        ],
        "{found:#?}"
    );
    let text = found
        .iter()
        .map(|(_, m)| m.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for expected in [
        "requirement 'LOOSE-01' is in no phase",
        "requirement 'TWICE-01' is in 2 phases",
        "phase 'v1.0/04' depends on 'v1.0/05', which is not an earlier phase",
        "phase 'v1.0/04' is on a dependency cycle",
        "phase 'v1.0/05' is on a dependency cycle",
        "requirement 'GONE-01' is out of scope but gives no reason",
        "phase numbers skip 03, before 'v1.0/04'",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
}

#[test]
fn a_clean_roadmap_reports_no_gsd_code() {
    assert_eq!(project("roadmap").findings_all(), []);
}

#[test]
fn a_second_project_is_e108() {
    let p = project("roadmap");
    p.write(
        "spec/second.spec",
        "project second \"Second\" {\n  name \"Second\"\n  what \"A second project\"\n  core_value \"none\"\n  context \"a test\"\n  constraints [{ kind other what \"none\" why \"a test\" }]\n}\n",
    );
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/E108");
    assert!(found[0].1.contains("a second project"), "{found:#?}");
}

fn phase_spec(number: &str) -> String {
    format!(
        "phase {number} \"P{number}\" {{\n  milestone v1.0\n  name \"P{number}\"\n  slug \"p\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}}\n\n"
    )
}

#[test]
fn letter_and_nested_decimal_phases_are_numbered_not_skipped() {
    let p = project("roadmap");
    p.write(
        "spec/more.spec",
        &["02", "02A", "02.1", "02.1.1"].map(phase_spec).concat(),
    );
    assert_eq!(p.findings(), [], "a lettered and a nested phase are clean");
    p.write("spec/more.spec", &["02", "02.1.1"].map(phase_spec).concat());
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0]
            .1
            .contains("'v1.0/02.1.1' follows phase 02.1, which does not exist"),
        "{found:#?}"
    );
}

#[test]
fn a_far_phase_number_is_one_warning() {
    let p = project("roadmap");
    p.write("spec/more.spec", &["02", "20000"].map(phase_spec).concat());
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/W104");
    assert!(found[0].1.contains("skip 03-19999"), "{found:#?}");
}

fn phase_with_criteria(number: &str, criteria: &[&str]) -> String {
    let items: Vec<String> = criteria.iter().map(|c| format!("{c:?}")).collect();
    format!(
        "phase {number} \"P{number}\" {{\n  milestone v1.0\n  name \"P{number}\"\n  slug \"p\"\n  goal \"g\"\n  success_criteria [{}]\n}}\n",
        items.join(", ")
    )
}

#[test]
fn six_criteria_is_w103() {
    let p = project("roadmap");
    p.write(
        "spec/more.spec",
        &phase_with_criteria("02", &["a", "b", "c", "d", "e", "f"]),
    );
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/W103");
    assert!(
        found[0].1.contains("'v1.0/02' has 6 success criteria"),
        "{found:#?}"
    );
    assert_eq!(p.run(&["check"]).code, 0, "a warning under gsd-compat");
}

#[test]
fn one_criterion_is_w103() {
    let p = project("roadmap");
    p.write("spec/more.spec", &phase_with_criteria("02", &["a"]));
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/W103");
}

#[test]
fn a_criterion_with_a_comma_counts_once() {
    let p = project("roadmap");
    // Joined by `, ` the field text would read as three.
    p.write(
        "spec/more.spec",
        &phase_with_criteria(
            "02",
            &["A task is added, listed, and saved", "A task is completed"],
        ),
    );
    assert_eq!(p.findings(), []);
    p.write(
        "spec/more.spec",
        &phase_with_criteria("02", &["A task is added, listed, and saved"]),
    );
    let found = p.findings();
    assert_eq!(found.len(), 1, "one item, however many commas: {found:#?}");
    assert!(found[0].1.contains("has 1 success criteria"), "{found:#?}");
}

#[test]
fn w103_is_an_error_under_gsd_strict() {
    let p = project("roadmap");
    p.write("spec/more.spec", &phase_with_criteria("02", &["a"]));
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(check.all().contains("W103"), "check: {}", check.all());
}

#[test]
fn a_phase_with_two_contexts_is_e206() {
    let p = project("discuss");
    p.write(
        "spec/second.spec",
        "context 02-CONTEXT-B \"Another context\" {\n  phase v1.0/02\n  boundary \"Again\"\n  canonical_refs [\n    { topic \"Storage\" paths [\"docs/storage.md\"] }\n  ]\n}\n",
    );
    assert_eq!(p.codes(), ["gsd/E206"], "no kernel code joins it");
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/E206");
    assert!(
        found[0]
            .1
            .contains("'v1.0/02/02-CONTEXT-B' is a second context"),
        "{found:#?}"
    );
}
