//! The roadmap rules, end to end: which codes `specforge check` reports on the `roadmap_errors` project, and that
//! the clean `roadmap` project reports none.

use crate::harness::{Project, project, taskboard};

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

/// A phase's second verification and second validation are reported, as its second context is: each is one file of the
/// phase directory, and nothing reported them before.
#[test]
fn a_phase_with_two_verifications_or_validations_is_e206() {
    let p = project("verify");
    let e206 = |p: &Project| -> Vec<String> {
        p.findings()
            .into_iter()
            .filter(|(code, _)| code == "gsd/E206")
            .map(|(_, message)| message)
            .collect()
    };
    assert_eq!(e206(&p), Vec::<String>::new(), "one of each");
    p.write(
        "spec/second.spec",
        "verification 01-VERIFICATION-2 \"Again\" {\n  phase v1.0/01\n}\n\n\
         validation 01-VALIDATION-2 \"Again\" {\n  phase v1.0/01\n}\n",
    );
    let added = e206(&p);
    assert_eq!(added.len(), 2, "{added:#?}");
    for (message, second) in added.iter().zip([
        "validation 'v1.0/01/01-VALIDATION-2' is a second validation of phase 'v1.0/01'",
        "verification 'v1.0/01/01-VERIFICATION-2' is a second verification of phase 'v1.0/01'",
    ]) {
        assert!(message.contains(second), "{added:#?}");
    }
}

/// A milestone keeps the Current Milestone's goal and target features: both write on the taskboard, read back, and
/// the check reports nothing new.
#[test]
fn a_milestone_keeps_its_goal_and_target_features() {
    let p = taskboard();
    let before = p.codes();
    for args in [
        &["set", "v1.0", "goal", "A board that survives a restart"][..],
        &["link", "v1.0", "target_features", "Save after every change"],
        &["link", "v1.0", "target_features", "List the open tasks"],
    ] {
        let run = p.run(args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.all());
    }
    assert_eq!(p.codes(), before);
    let fields = p.fields_of("v1.0");
    assert_eq!(fields["goal"], "A board that survives a restart");
    assert_eq!(
        fields["target_features"],
        serde_json::json!(["Save after every change", "List the open tasks"])
    );
}

/// The milestone `v1.1` with every field beyond GSD (ADR 0357), depending on `v1.0`, its dates `starts` and `targets`
/// given by the caller.
fn milestone_v1_1(starts: &str, targets: &str) -> String {
    format!(
        "milestone v1.1 \"Taskboard 1.1\" {{
  name           \"Taskboard 1.1\"
  exit_criteria  [\"A board is shared by link\", \"No task is lost on a restart\"]
  starts         {starts}
  targets        {targets}
  depends_on     [v1.0]
  owner          \"Board team\"
  contributors   [\"Ana\", \"Ben\"]
  artifact_types [\"cli\"]
  tags           [\"sharing\"]
}}
"
    )
}

/// A milestone may carry the old product milestone's planning fields, beyond GSD: all eight write, read back, and the
/// GSD project checks as clean as before.
#[test]
fn a_milestone_carries_its_planning_fields_beyond_gsd() {
    let p = project("roadmap");
    p.write(
        "spec/v1.1.spec",
        &milestone_v1_1("2026-01-05", "2026-03-01"),
    );
    assert_eq!(p.findings_all(), []);
    assert_eq!(p.codes(), Vec::<String>::new());
    let fields = p.fields_of("v1.1");
    let expected = serde_json::json!({
        "exit_criteria": ["A board is shared by link", "No task is lost on a restart"],
        "starts": "2026-01-05",
        "targets": "2026-03-01",
        "depends_on": ["v1.0"],
        "owner": "Board team",
        "contributors": ["Ana", "Ben"],
        "artifact_types": ["cli"],
        "tags": ["sharing"],
    });
    for (field, value) in expected.as_object().expect("an object") {
        assert_eq!(&fields[field], value, "{field}: {fields}");
    }
    assert!(
        edges_from(&p, "v1.1").contains(&("depends_on".into(), "v1.1".into(), "v1.0".into())),
        "{:#?}",
        edges_from(&p, "v1.1")
    );
}

/// Two milestones that depend on each other are each on the cycle: one `gsd/E102` per milestone, as for phases.
#[test]
fn a_milestone_dependency_cycle_is_e102() {
    let p = project("roadmap");
    p.write(
        "spec/v1.1.spec",
        &milestone_v1_1("2026-01-05", "2026-03-01"),
    );
    p.edit(
        "spec/v1.0.spec",
        "  name \"Taskboard 1.0\"\n",
        "  name \"Taskboard 1.0\"\n  depends_on [v1.1]\n",
    );
    let found = p.findings();
    let codes: Vec<&str> = found.iter().map(|(code, _)| code.as_str()).collect();
    assert_eq!(codes, ["gsd/E102", "gsd/E102"], "{found:#?}");
    for (at, milestone) in ["v1.0", "v1.1"].into_iter().enumerate() {
        let expected = format!("milestone '{milestone}' is on a dependency cycle");
        assert!(found[at].1.contains(&expected), "{expected} in {found:#?}");
    }
}

/// A milestone that targets a day before it starts is a contradiction, `gsd/E537`, an error under every profile;
/// the same day is clean.
#[test]
fn a_milestone_targeting_before_it_starts_is_e537() {
    for profile in ["gsd-compat", "gsd-strict", "gsd-plus"] {
        let p = project("roadmap");
        p.settings(serde_json::json!({ "profile": profile }));
        p.write(
            "spec/v1.1.spec",
            &milestone_v1_1("2026-03-01", "2026-02-01"),
        );
        let found = p.findings();
        let codes: Vec<&str> = found.iter().map(|(code, _)| code.as_str()).collect();
        assert_eq!(codes, ["gsd/E537"], "{profile}: {found:#?}");
        assert!(
            found[0]
                .1
                .contains("milestone 'v1.1' targets 2026-02-01 before it starts 2026-03-01"),
            "{profile}: {found:#?}"
        );
        let check = p.run(&["check"]);
        assert_eq!(
            check.code,
            1,
            "{profile}: an error fails the check: {}",
            check.all()
        );
        p.write(
            "spec/v1.1.spec",
            &milestone_v1_1("2026-03-01", "2026-03-01"),
        );
        assert_eq!(p.findings(), [], "{profile}: the same day is clean");
    }
}

/// A milestone date that is no calendar day is the kernel's E061 alone: the text still reaches the pass, and
/// `gsd/E537` never judges it, so no second error, false or misleading, rides on the first.
#[test]
fn a_milestone_date_that_is_no_calendar_day_is_e061_and_never_e537() {
    let p = project("roadmap");
    for (starts, targets) in [
        ("\"2026-3-1\"", "2026-10-01"),
        ("\"2026-3-1\"", "2026-02-01"),
        ("\"2026-03-01T09:00:00Z\"", "2026-03-01"),
        ("\"\"", "2026-01-01"),
    ] {
        p.write("spec/v1.1.spec", &milestone_v1_1(starts, targets));
        // The I204 note ("not judged") is every project's, not this one's.
        let codes: Vec<String> = p
            .checked()
            .into_iter()
            .map(|(code, _)| code)
            .filter(|code| code != "I204")
            .collect();
        assert_eq!(codes, ["E061"], "starts {starts}, targets {targets}");
    }
}

/// `(label, source, target)` of every edge out of `id`, as `specforge query --format json` reports them.
fn edges_from(p: &Project, id: &str) -> Vec<(String, String, String)> {
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
