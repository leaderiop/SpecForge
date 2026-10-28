//! The roadmap rules, end to end: which codes `specforge check` reports on the `roadmap_errors` project, and that
//! the clean `roadmap` project reports none.

use crate::harness::project;

/// `(code, entity)` of every diagnostic of `specforge check --format json`, sorted.
fn findings(p: &crate::harness::Project) -> Vec<(String, String)> {
    let run = p.run(&["check", "--format", "json"]);
    let diagnostics: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    let mut found: Vec<(String, String)> = diagnostics
        .as_array()
        .expect("a list")
        .iter()
        .filter(|d| d["code"].as_str().is_some_and(|c| c.starts_with("gsd/")))
        .map(|d| {
            let message = d["message"].as_str().unwrap_or_default();
            (
                d["code"].as_str().unwrap_or_default().to_string(),
                message.to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn roadmap_errors_report_each_code_once() {
    let p = project("roadmap_errors");
    let found = findings(&p);
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
    assert_eq!(findings(&project("roadmap")), []);
}

#[test]
fn a_second_project_is_e108() {
    let p = project("roadmap");
    p.write(
        "spec/second.spec",
        "project second \"Second\" {\n  name \"Second\"\n  what \"A second project\"\n  core_value \"none\"\n  context \"a test\"\n  constraints [{ kind other what \"none\" why \"a test\" }]\n}\n",
    );
    let found = findings(&p);
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
    assert_eq!(findings(&p), [], "a lettered and a nested phase are clean");
    p.write("spec/more.spec", &["02", "02.1.1"].map(phase_spec).concat());
    let found = findings(&p);
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
    let found = findings(&p);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "gsd/W104");
    assert!(found[0].1.contains("skip 03-19999"), "{found:#?}");
}
