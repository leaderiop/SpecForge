//! Black-box tests: the `specforge` binary on small projects that load the built `@gsd/core` (ADR gsd/0001 D5, D7).

mod harness;

use harness::project;

#[test]
fn an_empty_project_loads_gsd_core_and_checks_clean() {
    let p = project("empty");
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}{}", check.stdout, check.stderr);
    let listed = p.run(&["extensions"]);
    assert!(
        listed.stdout.contains("@gsd/core") && listed.stdout.contains("0.1.0"),
        "extensions: {}{}",
        listed.stdout,
        listed.stderr
    );
}

#[test]
fn explain_names_a_gsd_code() {
    let p = project("empty");
    let explained = p.run(&["explain", "gsd/E101"]);
    assert_eq!(
        explained.code, 0,
        "explain: {}{}",
        explained.stdout, explained.stderr
    );
    assert!(
        explained
            .stdout
            .contains("Requirement not mapped to a phase"),
        "explain: {}",
        explained.stdout
    );
}
