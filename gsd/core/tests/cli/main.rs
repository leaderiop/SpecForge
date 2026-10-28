//! Black-box tests: the `specforge` binary on small projects that load the built `@gsd/core` (ADR gsd/0001 D5, D7).

mod harness;

use harness::project;

#[test]
fn an_empty_project_loads_gsd_core_and_checks_clean() {
    let p = project("empty");
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    let listed = p.run(&["extensions"]);
    assert!(
        listed.all().contains("@gsd/core") && listed.all().contains("0.1.0"),
        "extensions: {}",
        listed.all()
    );
}

#[test]
fn explain_names_a_gsd_code() {
    let p = project("empty");
    let explained = p.run(&["explain", "gsd/E101"]);
    assert_eq!(explained.code, 0, "explain: {}", explained.all());
    assert!(
        explained
            .all()
            .contains("Requirement not mapped to a phase"),
        "explain: {}",
        explained.all()
    );
}

#[test]
fn a_project_with_requirements_checks_clean() {
    let p = project("roadmap");
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    assert!(
        check.all().contains("0 errors, 0 warnings"),
        "check: {}",
        check.all()
    );
}

#[test]
fn a_requirement_with_an_unknown_tier_is_refused() {
    let p = project("roadmap");
    p.write(
        "spec/bad.spec",
        "requirement TASK-09 \"Bad\" {\n  text \"x\"\n  category \"Tasks\"\n  tier v3\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(
        check.all().contains("E061") && check.all().contains("v1, v2, out_of_scope"),
        "check: {}",
        check.all()
    );
}
