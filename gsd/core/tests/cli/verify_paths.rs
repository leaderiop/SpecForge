//! `gsd/E320`, end to end: a verify command's directory is read through the project's files (K11), and every test
//! asserts the full `codes()` list.

use crate::harness::{Project, project};

/// A task of plan 01-02 whose one verify command is `command`, and files `files`.
fn with_task(p: &Project, command: &str, files: &str) {
    p.write(
        "spec/verify.spec",
        &format!(
            "task T2 \"Build the web part\" {{\n  plan v1.0/01/01-02\n  type auto\n  files [{files}]\n  action \"Build it\"\n  verify [\n    {{ automated \"{command}\" fails_when \"the command exits non-zero\" }}\n  ]\n  done \"It builds\"\n}}\n"
        ),
    );
}

fn message(p: &Project) -> String {
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    found[0].1.clone()
}

#[test]
fn a_computed_path_field_is_read_with_directory_facts() {
    let p = project("plan");
    with_task(&p, "cd web && npm test", "\"src/complete.js\"");
    assert_eq!(p.codes(), Vec::<String>::new());
    // A file is not a directory: the same read says so.
    with_task(&p, "cd web/package.json && npm test", "\"src/complete.js\"");
    assert_eq!(p.codes(), ["gsd/E320"]);
    assert!(message(&p).contains("'web/package.json' is not a directory"));
}

#[test]
fn a_missing_directory_is_e320() {
    let p = project("plan");
    with_task(&p, "cd nowhere && npm test", "\"src/complete.js\"");
    assert_eq!(p.codes(), ["gsd/E320"]);
    assert!(message(&p).contains("'nowhere' is not a directory"));
}

#[test]
fn a_directory_the_plan_creates_is_pending_not_missing() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "files_modified [\"src/complete.js\"]",
        "files_modified [\"src/complete.js\", \"fresh/package.json\", \"./fresh\", \"fresher/a.js\"]",
    );
    with_task(&p, "cd fresh && npm test", "\"fresh/package.json\"");
    assert_eq!(p.codes(), Vec::<String>::new());
    with_task(&p, "cd fresh && ls", "\"./fresh\"");
    assert_eq!(p.codes(), Vec::<String>::new());
    with_task(&p, "cd fresh && ls", "\"fresher/a.js\"");
    assert_eq!(p.codes(), ["gsd/E320"]);
}

#[test]
fn a_directory_without_the_manifest_its_tool_needs_is_e320() {
    let p = project("plan");
    p.write("empty/keep.txt", "x\n");
    with_task(&p, "npm --prefix empty run lint", "\"src/complete.js\"");
    assert_eq!(p.codes(), ["gsd/E320"]);
    assert!(message(&p).contains("'empty' has no package.json"));
    with_task(&p, "cd web && make test", "\"src/complete.js\"");
    assert!(message(&p).contains("'web' has no Makefile"));
}

#[test]
fn the_project_root_needs_no_directory_fact() {
    let p = project("plan");
    with_task(&p, "cd . && ls", "\"src/complete.js\"");
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_path_no_command_names_is_not_read() {
    let p = project("plan");
    with_task(&p, "node --test", "\"src/complete.js\"");
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_dynamic_path_is_w324() {
    let p = project("plan");
    with_task(&p, "cd $DIR && npm test", "\"src/complete.js\"");
    assert_eq!(p.codes(), ["gsd/W324"]);
    assert!(message(&p).contains("computed at run time"));
}

#[test]
fn an_outside_path_is_w324() {
    let p = project("plan");
    with_task(&p, "cd ../.. && make", "\"src/complete.js\"");
    assert_eq!(p.codes(), ["gsd/W324"]);
    assert!(message(&p).contains("outside the project"));
}

#[test]
fn a_missing_npm_script_is_w324() {
    let p = project("plan");
    with_task(&p, "npm --prefix web run build", "\"src/complete.js\"");
    assert_eq!(p.codes(), ["gsd/W324"]);
    assert!(message(&p).contains("'web/package.json' has no script 'build'"));
    with_task(&p, "npm --prefix web run lint", "\"src/complete.js\"");
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn an_unparsable_package_json_is_w324() {
    let p = project("plan");
    p.write("broken/package.json", "{ not json\n");
    with_task(&p, "cd broken && npm test", "\"src/complete.js\"");
    assert_eq!(p.codes(), ["gsd/W324"]);
    assert!(message(&p).contains("cannot be read as a manifest"));
}

#[test]
fn a_broken_directory_is_e320_and_not_also_w324() {
    let p = project("plan");
    with_task(&p, "cd nowhere && npm run build", "\"src/complete.js\"");
    assert_eq!(p.codes(), ["gsd/E320"]);
}
