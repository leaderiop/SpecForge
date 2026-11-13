//! The execute family's shell lines, run as the assets hold them on a taskboard copy under bash with only `specforge`
//! on the `PATH`.

use super::{Shell, asset, block, typed};
use crate::harness::taskboard;

/// The `<required_reading>` block of execute-phase's executor spawn: the spawn's prompt sits in a `text` fence, so the
/// block is read from the dispatch marker on, not as an HTML block.
fn executor_reading(text: &str) -> String {
    let at = text
        .find("[gsd:dispatch phase=")
        .expect("the executor spawn's dispatch marker");
    block(&text[at..], "<required_reading>", "</required_reading>")
}

/// The executor spawn hands the executor `{phase_dir}/{plan_file}` as its plan, and no fence of the workflow assigns
/// either: the executor's reading list carries no plan.
#[test]
fn the_executor_spawn_reads_paths_no_step_sets() {
    let text = asset("gsd-core/workflows/execute-phase.md");
    let reading = executor_reading(&text);
    assert!(reading.contains("{phase_dir}/{plan_file}"), "{reading}");
    let assigns = regex::Regex::new(r"(?m)\b(phase_dir|plan_file)=").expect("a regex");
    for fence in gsd_kit::markdown::fences(&text) {
        assert!(
            !assigns.is_match(&fence.text),
            "a fence sets it:\n{}",
            fence.text
        );
    }
}

/// The summary template's fence, with a decision the executor took first, closes a plan as a model fills it: the
/// summary, its deliverable and its deviation exist, `complete-plan` wrote `completed`, and the check reports nothing
/// new.
#[test]
fn the_summary_template_closes_a_plan() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    p.write("src/store.js", "export function saveBoard() {}\n");
    p.write(
        "test/store.test.js",
        "test(\"a save then a load\", () => {});\n",
    );
    let started = p.run(&["gsd", "start-plan", "v1.0/02/02-01"]);
    assert_eq!(started.code, 0, "{}", started.all());
    let decision = typed::filled(
        &typed::typed_fence(&asset("agents/gsd-executor.md"), "**Decisions first:**"),
        &[
            ("PLAN_ID", "v1.0/02/02-01"),
            ("DECISION_ID", "D-04"),
            ("DECISION", "The store syncs the file before the rename"),
        ],
    );
    let summary = typed::filled(
        &typed::typed_fence(
            &asset("gsd-core/templates/summary.md"),
            "## The Summary's Typed Edits",
        ),
        &[
            ("PLAN_ID", "v1.0/02/02-01"),
            ("TITLE", "Save and load tasks.json: summary"),
            ("STATUS", "complete"),
            (
                "ONE_LINER",
                "Tasks survive a restart: saved after every change, loaded on start",
            ),
            ("SUBSYSTEM", "store"),
            ("TAG", "node"),
            (
                "REQUIRES",
                r#"[{"phase": "v1.0/01", "provides": "the board"}]"#,
            ),
            ("PROVIDES", "loadBoard and saveBoard"),
            ("AFFECTS", "listing"),
            ("TECH_ADDED", "node:fs"),
            ("TECH_PATTERN", "write to a temporary name, then rename"),
            ("KEY_DECISIONS", "D-04"),
            ("PATTERN", "one store module owns the file"),
            ("REQUIREMENTS_COMPLETED", "STORE-01,STORE-02"),
            (
                "THREAT_FLAG",
                "threat_flag: file | src/store.js | writes tasks.json",
            ),
            ("NEXT_READINESS", "listing can read the saved board"),
            ("DELIVERABLE", "A saved board loads back with its tasks"),
            ("REQUIREMENT", "STORE-01"),
            ("HUMAN_JUDGMENT", "false"),
            ("RATIONALE", "the store tests prove it"),
            (
                "CHECKS",
                r#"[{"kind": "unit", "ref": "test/store.test.js#a save then a load"}]"#,
            ),
            ("DEVIATION_KIND", "deviation"),
            ("RULE", "Rule 1 - Bug"),
            (
                "DEVIATION_TEXT",
                "A missing file threw; it now loads an empty board, and a test covers it",
            ),
            ("DEVIATION_FILE", "src/store.js"),
        ],
    );
    let first = typed::apply(&sh, &p, &decision);
    assert_eq!(first.code, 0, "{}{}", first.out, first.err);
    let closed = typed::apply(&sh, &p, &summary);
    assert_eq!(closed.code, 0, "{}{}", closed.out, closed.err);
    assert!(
        first.added.is_empty() && closed.added.is_empty(),
        "{:?} {:?}",
        first.added,
        closed.added
    );
    let fields = p.fields_of("v1.0/02/02-01/02-01-SUMMARY");
    assert!(fields["completed"].is_string(), "{fields}");
    assert_eq!(fields["key_decisions"], serde_json::json!(["v1.0/02/D-04"]));
    for id in ["D1", "DV1"] {
        let id = format!("v1.0/02/02-01/02-01-SUMMARY/{id}");
        assert!(p.fields_of(&id).is_object(), "{id} is missing");
    }
}
