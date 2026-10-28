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

fn verify_task(p: &Project, items: &str) {
    p.write(
        "spec/verify.spec",
        &format!(
            "task T2 \"Check it\" {{\n  plan v1.0/01/01-02\n  type auto\n  files [\"src/complete.js\"]\n  action \"Check it\"\n  verify [\n{items}  ]\n  done \"It is checked\"\n}}\n"
        ),
    );
}

#[test]
fn automated_without_fails_when_is_e507() {
    let p = project("plan");
    verify_task(
        &p,
        "    { automated \"node --test\" }\n    { automated \"node a.js\" fails_when \"it exits non-zero\" }\n",
    );
    assert_eq!(p.codes(), ["gsd/E507"]);
    let found = p.findings();
    assert!(
        found[0].1.contains("`node --test` states no `fails_when`"),
        "{found:?}"
    );
}

#[test]
fn an_empty_fails_when_is_e507() {
    let p = project("plan");
    verify_task(&p, "    { automated \"node --test\" fails_when \"\" }\n");
    assert_eq!(p.codes(), ["gsd/E507"]);
}

#[test]
fn a_placeholder_fails_when_is_e507() {
    let p = project("plan");
    for text in ["TBD", "n/a", "-"] {
        verify_task(
            &p,
            &format!("    {{ automated \"node --test\" fails_when \"{text}\" }}\n"),
        );
        assert_eq!(p.codes(), ["gsd/E507"], "{text}");
    }
}

#[test]
fn a_vacuous_but_real_statement_is_clean() {
    let p = project("plan");
    verify_task(
        &p,
        "    { automated \"node --test\" fails_when \"TBD in the harness output\" }\n",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_missing_sentinel_is_not_e507() {
    let p = project("plan");
    verify_task(
        &p,
        "    { automated \"MISSING - Wave 0 creates the test\" }\n",
    );
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_bare_verify_text_is_not_e507() {
    // Plan 02-01's task has only `verify_text`.
    assert_eq!(project("plan").codes(), Vec::<String>::new());
}

/// `count` tasks of plan 01-02, task `i` running `command(i)`.
fn many_tasks(p: &Project, count: usize, command: impl Fn(usize) -> String) {
    let tasks: String = (0..count)
        .map(|i| {
            format!(
                "task T{i:04} \"Task {i}\" {{\n  plan v1.0/01/01-02\n  type auto\n  files [\"src/complete.js\"]\n  action \"Do it\"\n  verify [\n    {{ automated \"{}\" fails_when \"it exits non-zero\" }}\n  ]\n  done \"Done\"\n}}\n\n",
                command(i)
            )
        })
        .collect();
    p.write("spec/verify.spec", &tasks);
}

/// The codes but E322 (a plan of five or more tasks), which the many-task tests cannot avoid.
fn codes_but_size(p: &Project) -> Vec<String> {
    let mut codes = p.codes();
    codes.retain(|c| c != "gsd/E322");
    codes
}

/// 25 tasks run in the same 52 KB `web/package.json`: the host spends its 1 MiB text budget once, not once a task.
#[test]
fn one_manifest_is_read_once_for_all_the_tasks_that_name_it() {
    let p = project("plan");
    let padding = "x".repeat(52 * 1024);
    p.write(
        "web/package.json",
        &format!("{{ \"scripts\": {{ \"lint\": \"eslint .\" }}, \"pad\": \"{padding}\" }}\n"),
    );
    many_tasks(&p, 25, |_| "cd web && npm run lint".into());
    assert_eq!(codes_but_size(&p), Vec::<String>::new());
}

/// More distinct paths than the host serves (2000): a path it dropped is not checked, so it is no E320.
#[test]
fn a_path_the_host_did_not_serve_is_not_checked() {
    let p = project("plan");
    for i in 0..2001 {
        p.write(&format!("many/d{i}/keep.txt"), "x\n");
    }
    many_tasks(&p, 2001, |i| format!("cd many/d{i} && ls"));
    assert_eq!(codes_but_size(&p), Vec::<String>::new());
}

#[test]
fn a_manifest_over_the_host_cap_is_not_checked() {
    let p = project("plan");
    let padding = "x".repeat(70 * 1024);
    p.write(
        "big/package.json",
        &format!("{{ \"pad\": \"{padding}\" }}\n"),
    );
    with_task(&p, "cd big && npm run lint", "\"src/complete.js\"");
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_withheld_file_is_not_a_directory() {
    let p = project("plan");
    p.write(".env", "SECRET=1\n");
    with_task(&p, "cd .env && ls", "\"src/complete.js\"");
    assert_eq!(p.codes(), ["gsd/E320"]);
}

#[test]
fn task_files_are_split_on_commas_and_white_space() {
    let p = project("plan");
    with_task(&p, "cd fresh && ls", "\"a.js, fresh/b.js\"");
    // W306 compares the item as written; only the pending-creation check splits it, as GSD does.
    assert_eq!(p.codes(), ["gsd/W306"]);
}
