//! The scan pass end to end (`spec/verify.md`): artifacts, key-link patterns, debt markers and anti-patterns, read
//! from the files of the `verify` project.

use crate::harness::{Project, project};

const AR1: &str = "v1.0/01/01-01/AR1";
const KL1: &str = "v1.0/01/01-01/KL1";
const TASKS: &str = "src/tasks.js";

fn verify() -> Project {
    project("verify")
}

fn state(p: &Project, id: &str) -> serde_json::Value {
    p.fields_of(id)["state"].clone()
}

fn append(p: &Project, file: &str, line: &str) {
    let text = p.read(file);
    p.write(file, &format!("{text}{line}\n"));
}

#[test]
fn a_missing_artifact_is_reported() {
    let p = verify();
    assert_eq!(state(&p, AR1), "present");
    std::fs::remove_file(p.root().join(TASKS)).expect("remove the artifact");
    assert_eq!(state(&p, AR1), "missing");
}

#[test]
fn todo_in_a_phase_file_is_e505() {
    let p = verify();
    assert_eq!(p.codes(), Vec::<String>::new());
    append(&p, TASKS, "// FIXME remove the copy");
    assert_eq!(p.codes(), ["gsd/E505"], "{:?}", p.report());
    let message = &p.report()[0].1;
    assert!(message.contains("src/tasks.js:9"), "{message}");
}

#[test]
fn a_debt_marker_with_an_issue_ref_is_silent() {
    let p = verify();
    for line in [
        "// FIXME #42 remove the copy",
        "// TBD see PR #7",
        "// XXX tracked in DEF-12",
    ] {
        append(&p, TASKS, line);
    }
    assert!(p.read(TASKS).contains("DEF-12"));
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

#[test]
fn xxxl_is_not_a_debt_marker() {
    let p = verify();
    append(&p, TASKS, "const size = \"XXXL\";");
    assert!(p.read(TASKS).contains("XXXL"));
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

#[test]
fn a_short_artifact_is_too_short() {
    let p = verify();
    p.edit(
        "spec/plans.spec",
        "  path     \"src/tasks.js\"\n  provides \"add() and list()\"\n",
        "  path      \"src/tasks.js\"\n  provides  \"add() and list()\"\n  min_lines 50\n",
    );
    assert_eq!(state(&p, AR1), "too_short");
    p.edit("spec/plans.spec", "min_lines 50", "min_lines 5");
    assert_eq!(state(&p, AR1), "present");
}

#[test]
fn a_missing_export_is_reported() {
    let p = verify();
    p.edit(
        "spec/plans.spec",
        "  provides \"add() and list()\"\n",
        "  provides \"add() and list()\"\n  exports  [\"add\", \"remove\"]\n",
    );
    assert_eq!(state(&p, AR1), "lacks_export");
    p.edit(
        "spec/plans.spec",
        "\"add\", \"remove\"",
        "\"add\", \"list\"",
    );
    assert_eq!(state(&p, AR1), "present");
    p.edit(
        "spec/plans.spec",
        "  exports  [\"add\", \"list\"]\n",
        "  contains \"Math.random\"\n",
    );
    assert_eq!(state(&p, AR1), "lacks_contains");
}

#[test]
fn a_key_link_pattern_not_found_is_not_wired() {
    let p = verify();
    assert_eq!(
        state(&p, KL1),
        serde_json::Value::Null,
        "no pattern, no state"
    );
    let anchor = "  via  \"require\"\n";
    p.edit(
        "spec/plans.spec",
        anchor,
        &format!("{anchor}  pattern \"require\"\n"),
    );
    assert_eq!(state(&p, KL1), "not_wired");
    p.edit(
        "spec/plans.spec",
        "pattern \"require\"",
        "pattern \"node:test\"",
    );
    assert_eq!(state(&p, KL1), "wired");
    p.edit(
        "spec/plans.spec",
        "pattern \"node:test\"",
        "pattern \"import.*tasks\"",
    );
    assert_eq!(state(&p, KL1), "unchecked");
}

#[test]
fn a_withheld_artifact_is_unchecked_not_missing() {
    let p = verify();
    // Past the 64 KiB cap the host sends no text, so `contains` cannot be judged.
    p.write(TASKS, &"// filler\n".repeat(8000));
    p.edit(
        "spec/plans.spec",
        "  provides \"add() and list()\"\n",
        "  provides \"add() and list()\"\n  contains \"add\"\n",
    );
    assert_eq!(state(&p, AR1), "unchecked");
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

#[test]
fn a_wave0_item_is_done_when_its_file_exists_and_its_test_ran() {
    let p = verify();
    let item = "v1.0/01/01-VALIDATION/W0-1";
    assert_eq!(p.fields_of(item)["done"], false);
    std::fs::remove_file(p.root().join("test/tasks.test.js")).expect("remove the stub");
    assert_eq!(p.fields_of(item)["done"], false);
}

#[test]
fn anti_patterns_list_todo_as_warning() {
    let p = verify();
    append(&p, "src/complete.js", "// TODO handle undo");
    let found = p.fields_of("v1.0/01/01-VERIFICATION")["anti_patterns"].clone();
    assert_eq!(
        found,
        serde_json::json!([{ "file": "src/complete.js", "line": 4, "text": "// TODO handle undo", "severity": "warning" }]),
        "{found}"
    );
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}
