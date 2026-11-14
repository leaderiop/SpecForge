//! spec-phase's typed edits, run as the spec template and the workflow hold them on a taskboard copy under bash with
//! only `specforge` on the `PATH`.

use super::{Shell, asset, fence_after, typed};
use crate::harness::taskboard;

const SPEC_ID: &str = "v1.0/03/03-SPEC";

/// The spec template's fence, filled for phase `v1.0/03` with one requirement and one interview.
fn spec_fence() -> String {
    typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/templates/spec.md"),
            "## The Spec's Typed Edits",
        ),
        &[
            ("PHASE_ID", "v1.0/03"),
            ("PADDED_PHASE", "03"),
            ("NAME", "Listing"),
            ("CREATED", "2026-11-13"),
            ("GOAL", "taskboard list shows the open tasks, oldest first"),
            (
                "BACKGROUND",
                "The board can add and complete tasks; nothing lists them",
            ),
            ("IN_SCOPE", "The list subcommand"),
            (
                "OUT_OF_SCOPE",
                "Sorting by due date: tasks have no due date",
            ),
            ("CONSTRAINT", "Node 20, no dependency"),
            (
                "ACCEPTANCE",
                "taskboard list prints each open task on its own line",
            ),
            ("REQ_TITLE", "List the open tasks"),
            ("CURRENT", "No command lists the tasks"),
            (
                "TARGET",
                "taskboard list prints the open tasks, oldest first",
            ),
            (
                "REQ_ACCEPTANCE",
                "A test adds two tasks and reads them back in order",
            ),
            ("TRACES", "TASK-03"),
            ("ROUND", "1"),
            ("PERSPECTIVE", "researcher"),
            ("QUESTION", "What lists the tasks today?"),
            ("ANSWER", "Nothing: the board only adds and completes"),
            ("LOCKED", "A new list subcommand"),
        ],
    )
}

/// The spec fence records the spec, its requirement and interview; Step 5.5's line records an edge row once however
/// often it runs and adds its acceptance line; the soft gate counts what is left; Step 5.6's line records a
/// prohibition, which `resolve-prohibition` then dismisses with a reason. The check reports nothing new (the judged codes
/// that wait for the spec's verdict left out).
#[test]
fn the_spec_fences_record_the_spec_and_its_rows() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let workflow = asset("gsd-core/workflows/spec-phase.md");
    let edge = typed::filled(
        &format!(
            "SPEC_ID={SPEC_ID}\n{}",
            fence_after(&workflow, "Record each row as it is resolved")
        ),
        &[
            ("LR_ID", "v1.0/03/03-SPEC/LR1"),
            ("CATEGORY", "empty"),
            ("STATUS", "resolved"),
            ("VERIFICATION", "explicit"),
            (
                "RESOLUTION",
                "taskboard list on an empty board prints nothing and exits 0",
            ),
            ("REASON", "Specified with the user"),
        ],
    );
    let gate = format!(
        "SPEC_ID={SPEC_ID}\n{}\necho \"$UNRESOLVED\"",
        fence_after(&workflow, "**Soft gate (after resolving):**")
    );
    let prohibition = typed::filled(
        &format!(
            "SPEC_ID={SPEC_ID}\n{}",
            fence_after(&workflow, "Record each prohibition as it is resolved")
        ),
        &[
            ("LR_ID", "v1.0/03/03-SPEC/LR1"),
            ("STATEMENT", "The list must not print a completed task"),
            ("STATUS", "resolved"),
            ("VERIFICATION", "test"),
            ("REASON", "Kept with the user"),
            ("CHECK_KIND", "node_test"),
            ("CHECK_TARGET", "test/list.test.js"),
            ("CHECK_RULE", "none"),
            ("CHECK_VIOLATION_FIXTURE", "test/fixtures/completed.json"),
            ("CHECK_CLEAN_FIXTURE", "test/fixtures/open.json"),
        ],
    );
    let mut added = Vec::new();
    for (name, script) in [
        ("spec", spec_fence()),
        ("edge", edge.clone()),
        ("edge again", edge),
        ("prohibition", prohibition),
    ] {
        let run = typed::apply(&sh, &p, &script);
        assert_eq!(run.code, 0, "{name}: {}{}", run.out, run.err);
        added.extend(run.added);
    }
    assert!(added.is_empty(), "{added:?}");
    let dismissed = p.run(&[
        "gsd",
        "resolve-prohibition",
        "v1.0/03/03-SPEC/SP1",
        "dismissed",
        "--reason",
        "The listing reads only open tasks",
    ]);
    assert_eq!(dismissed.code, 0, "{}", dismissed.all());
    let (counted, err, code) = sh.run_in(p.root(), &gate);
    assert_eq!(code, 0, "{err}");
    assert!(counted.trim().parse::<u32>().is_ok(), "a count: {counted}");
    let kinds = p.run(&["query", SPEC_ID, "--depth", "1", "--pick", ".nodes[].kind"]);
    let kinds: Vec<&str> = kinds.stdout.lines().collect();
    for kind in ["locked_req", "interview", "edge", "spec_prohibition"] {
        assert!(kinds.contains(&kind), "{kind}: {kinds:?}");
    }
    assert_eq!(kinds.iter().filter(|k| **k == "edge").count(), 1, "one row");
    let acceptance = p.fields_of(SPEC_ID)["acceptance"].to_string();
    assert!(
        acceptance.contains("on an empty board prints nothing")
            && acceptance.contains("Must not: The list must not print a completed task"),
        "{acceptance}"
    );
}
