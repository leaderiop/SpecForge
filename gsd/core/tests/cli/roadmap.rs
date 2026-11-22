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
            "gsd/E101", "gsd/E101", "gsd/E102", "gsd/E102", "gsd/E102", "gsd/E105", "gsd/W104",
            "gsd/W109"
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
        "phase 'v1.0/02' is titled 'Phase 02: Second', which begins with the phase number 'Phase 02'",
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
        "phase {number} \"P{number}\" {{\n  milestone v1.0\n  slug \"p\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}}\n\n"
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
        "phase {number} \"P{number}\" {{\n  milestone v1.0\n  slug \"p\"\n  goal \"g\"\n  success_criteria [{}]\n}}\n",
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

fn titled_phase(number: &str, title: &str) -> String {
    format!(
        "phase {number} {title:?} {{\n  milestone v1.0\n  slug \"p\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}}\n\n"
    )
}

/// A phase is labelled `Phase <its id's last segment>: <title>` (ADR 0387): a title that begins with a phase number
/// prints a second one, which a renumber leaves behind.
#[test]
fn a_numbered_phase_title_is_w109() {
    let p = project("roadmap");
    p.write(
        "spec/more.spec",
        &[
            titled_phase("02", "Phase 12: Graph Protocol"),
            titled_phase("03", "3. Listing"),
        ]
        .concat(),
    );
    let found = p.findings();
    let codes: Vec<&str> = found.iter().map(|(code, _)| code.as_str()).collect();
    assert_eq!(codes, ["gsd/W109", "gsd/W109"], "{found:#?}");
    assert!(
        found[0].1.contains(
            "phase 'v1.0/02' is titled 'Phase 12: Graph Protocol', which begins with the phase number 'Phase 12'"
        ),
        "{found:#?}"
    );
    let check = p.run(&["check", "--format", "json"]);
    assert!(
        check.stdout.contains(
            "retitle it 'Graph Protocol': its label then reads 'Phase 02: Graph Protocol'"
        ),
        "the suggestion names the title without its number: {}",
        check.all()
    );
    assert_eq!(p.run(&["check"]).code, 0, "a warning under gsd-compat");
}

#[test]
fn a_title_with_a_number_inside_is_not_w109() {
    let p = project("roadmap");
    p.write(
        "spec/more.spec",
        &[
            titled_phase("02", "HTTP 2 support"),
            titled_phase("03", "2FA login"),
            titled_phase("04", "v2.0 migration"),
        ]
        .concat(),
    );
    assert_eq!(p.findings(), []);
}

#[test]
fn w109_is_an_error_under_gsd_strict_and_gsd_plus() {
    for profile in ["gsd-strict", "gsd-plus"] {
        let p = project("roadmap");
        p.write(
            "spec/more.spec",
            &titled_phase("02", "Phase 2: Persistence"),
        );
        p.settings(serde_json::json!({ "profile": profile }));
        let check = p.run(&["check"]);
        assert_eq!(check.code, 1, "{profile}: {}", check.all());
        assert!(check.all().contains("W109"), "{profile}: {}", check.all());
    }
}

/// `phase` and `milestone` declare no `name` (ADR 0387): a project written for the old declaration is told, by the
/// kernel's W020 on each line, that the field is not its kind's.
#[test]
fn a_name_line_on_a_phase_or_a_milestone_is_w020() {
    let p = project("roadmap");
    p.write(
        "spec/more.spec",
        "milestone v2.0 \"Taskboard 2.0\" {\n  name \"Taskboard 2.0\"\n}\n\nphase 02 \"Listing\" {\n  milestone v1.0\n  \
         name \"Listing\"\n  slug \"listing\"\n  goal \"g\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    let w020: Vec<String> = p
        .checked()
        .into_iter()
        .filter(|(code, _)| code.ends_with("W020"))
        .map(|(_, message)| message)
        .collect();
    assert_eq!(w020.len(), 2, "{w020:#?}");
    for (entity, kind) in [("v2.0", "milestone"), ("v1.0/02", "phase")] {
        assert!(
            w020.iter()
                .any(|m| m.contains("name") && m.contains(entity)),
            "no W020 on {kind} {entity}'s name: {w020:#?}"
        );
    }
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
