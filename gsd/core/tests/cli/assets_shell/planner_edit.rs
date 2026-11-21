//! The planner's plan edit: the `json` fence of its `<plan_format>`, filled and sent as one MCP `specforge.edit` call
//! on the taskboard twin's planless phase v1.0/03 (requirements TASK-03 and LIST-01) under gsd-compat and gsd-strict,
//! and the refusals of the plan-by-plan route it replaces.

use serde_json::{Value, json};

use super::{Shell, asset, block, line, mcp_edit, typed};
use crate::commands_support::{refused, strict};
use crate::harness::{Project, taskboard};

/// The planner's body.
fn planner() -> String {
    asset("agents/gsd-planner.md")
}

/// The planner's `<plan_format>` block.
fn plan_format() -> String {
    block(&planner(), "<plan_format>", "</plan_format>")
}

/// The first plan's task action as the model writes it in JSON: a double quote, an em dash, a non-ASCII letter and a
/// line break, escaped as a JSON string body.
const ACTION_JSON: &str = r#"Read \"tasks.json\" — é\nthen sort by id"#;

/// [`ACTION_JSON`] as it reads back.
const ACTION: &str = "Read \"tasks.json\" — é\nthen sort by id";

/// The values the model writes into the plan edit: plans 03-01 (TASK-03) and 03-02 (LIST-01, after 03-01) of phase
/// v1.0/03. A key may hold its quotes: the estimate's projection is a quoted placeholder written as a bare number.
const FILL: &[(&str, &str)] = &[
    (
        "\"{the first plan's calibrated projection, e.g. 60000}\"",
        "40000",
    ),
    (
        "\"{the second plan's calibrated projection, e.g. 60000}\"",
        "30000",
    ),
    ("{the phase id, e.g. v1.0/03}", "v1.0/03"),
    ("{the phase file's directory, e.g. spec}", "spec"),
    ("{the first plan's id, e.g. 03-01}", "03-01"),
    ("{the second plan's id, e.g. 03-02}", "03-02"),
    ("{execute or tdd}", "execute"),
    ("{the first plan's title}", "List open tasks"),
    ("{a file the first plan touches}", "src/cli.js"),
    (
        "{a requirement the first plan addresses, e.g. TASK-03}",
        "TASK-03",
    ),
    (
        "{what the first plan accomplishes}",
        "List open tasks oldest first",
    ),
    ("{why the first plan matters}", "see what is left"),
    ("{what the first plan creates}", "a list command"),
    (
        "{a spec file or source the first plan's executor reads}",
        "spec/project.spec",
    ),
    ("{the first plan's overall check}", "npm test"),
    ("{the first plan's measurable completion}", "TASK-03 holds"),
    (
        "{the first plan's task, e.g. Task 1: an action-oriented name}",
        "Task 1: list",
    ),
    ("{the file the first plan's task touches}", "src/cli.js"),
    (
        "{the first plan's task's specific implementation}",
        ACTION_JSON,
    ),
    ("{the first plan's task's command or check}", "npm test"),
    (
        "{the first plan's task's acceptance criteria}",
        "list prints open tasks",
    ),
    (
        "{an observable behavior the first plan makes true}",
        "taskboard list shows the open tasks",
    ),
    ("{what the first plan's artifact is}", "The list command"),
    ("{a file the first plan must leave}", "src/cli.js"),
    ("{what the first plan's file provides}", "list"),
    (
        "{the first plan's connection}",
        "The command line reads the tasks",
    ),
    ("{the file that uses it in the first plan}", "src/cli.js"),
    ("{the file it uses in the first plan}", "src/tasks.js"),
    (
        "{how the first plan connects them, e.g. the import}",
        "the import",
    ),
    ("{the second plan's title}", "Filter tasks"),
    ("{a file the second plan touches}", "src/cli.js"),
    (
        "{a requirement the second plan addresses, e.g. LIST-01}",
        "LIST-01",
    ),
    (
        "{what the second plan accomplishes}",
        "Filter tasks by status",
    ),
    ("{why the second plan matters}", "find one task fast"),
    ("{what the second plan creates}", "a status filter"),
    (
        "{a spec file or source the second plan's executor reads}",
        "spec/project.spec",
    ),
    ("{the second plan's overall check}", "npm test"),
    ("{the second plan's measurable completion}", "LIST-01 holds"),
    (
        "{the second plan's task, e.g. Task 1: an action-oriented name}",
        "Task 1: filter",
    ),
    ("{the file the second plan's task touches}", "src/cli.js"),
    (
        "{the second plan's task's specific implementation}",
        "Add a --status filter to list",
    ),
    ("{the second plan's task's command or check}", "npm test"),
    (
        "{the second plan's task's acceptance criteria}",
        "list --status done prints done tasks",
    ),
    (
        "{an observable behavior the second plan makes true}",
        "taskboard list --status done shows done tasks",
    ),
    ("{what the second plan's artifact is}", "The status filter"),
    ("{a file the second plan must leave}", "src/cli.js"),
    (
        "{what the second plan's file provides}",
        "the status filter",
    ),
    (
        "{the second plan's connection}",
        "The filter reads the tasks",
    ),
    ("{the file that uses it in the second plan}", "src/cli.js"),
    ("{the file it uses in the second plan}", "src/tasks.js"),
    (
        "{how the second plan connects them, e.g. the import}",
        "the filter call",
    ),
];

/// The plan edit of the planner's `<plan_format>` (its one `json` fence, JSON as written), filled with [`FILL`].
pub(super) fn plan_edit() -> Value {
    let fence = gsd_kit::markdown::fences(&plan_format())
        .into_iter()
        .find(|f| f.language == "json")
        .expect("the plan edit's json fence")
        .text;
    if let Err(e) = serde_json::from_str::<Value>(&fence) {
        panic!("the plan edit is JSON as written ({e}): {fence}");
    }
    let mut edit = fence;
    for (key, value) in FILL {
        assert!(edit.contains(key), "no {key} in the plan edit");
        edit = edit.replace(key, value);
    }
    assert!(!edit.contains("\"{"), "a value left unfilled: {edit}");
    serde_json::from_str(&edit)
        .unwrap_or_else(|e| panic!("the filled plan edit is JSON ({e}): {edit}"))
}

/// The create op of `kind` and `id` in `edit`.
fn op<'a>(edit: &'a mut Value, kind: &str, id: &str) -> &'a mut Value {
    edit.as_array_mut()
        .expect("a list of edits")
        .iter_mut()
        .find(|e| e["kind"] == kind && e["id"] == id)
        .unwrap_or_else(|| panic!("no {kind} {id} in the plan edit"))
}

/// [`plan_edit`] with a checkpoint, a prohibition and a user setup step of plan 03-02, in that plan's file.
fn whole_edit() -> Value {
    let mut edit = plan_edit();
    let file = op(&mut edit, "plan", "03-02")["file"].clone();
    let plan = "v1.0/03/03-02";
    let more = [
        json!({"op": "create", "kind": "checkpoint", "id": "CP1", "title": "The filter is checked by hand",
            "file": file, "fields": {"plan": plan, "kind": "human_verify", "after": "v1.0/03/03-02/T1",
            "what_built": "the status filter", "how_to_verify": "run list --status done",
            "resume_signal": "approved"}}),
        json!({"op": "create", "kind": "prohibition", "id": "PR1", "title": "A filter never hides an open task",
            "file": file, "fields": {"plan": plan, "statement": "A filter never hides an open task",
            "verification": "judgment"}}),
        json!({"op": "create", "kind": "user_setup", "id": "US1", "title": "none", "file": file,
            "fields": {"plan": plan, "service": "none", "why": "the filter needs no service"}}),
    ];
    edit.as_array_mut().expect("a list of edits").extend(more);
    edit
}

/// The ids of the graph's nodes of `kind`.
fn ids(p: &Project, kind: &str) -> Vec<String> {
    p.nodes_of(kind)
        .iter()
        .filter_map(|n| n["id"].as_str().map(str::to_string))
        .collect()
}

/// `after` less `before`, each code counted.
fn added(before: &[String], after: Vec<String>) -> Vec<String> {
    let mut left = before.to_vec();
    after
        .into_iter()
        .filter(|code| match left.iter().position(|c| c == code) {
            Some(at) => {
                left.remove(at);
                false
            }
            None => true,
        })
        .collect()
}

/// The two plan files the edit writes.
const FILES: [&str; 2] = ["spec/plans/03-01.spec", "spec/plans/03-02.spec"];

/// On the planless phase, under gsd-compat and under gsd-strict, the plan edit with 03-02's requirements emptied is
/// refused whole (`gsd/E301`) and writes no file; the edit as filled is accepted in one call, writes both plans with
/// their task, truth, artifact and key link (and 03-02's checkpoint, prohibition and user setup step), reads the task
/// action back exactly and adds no code but the judged rules'; sent again, it is refused as a conflict and leaves both
/// files byte for byte.
#[test]
fn the_planner_writes_a_phase_in_one_edit() {
    for profile in ["gsd-compat", "gsd-strict"] {
        let p = taskboard();
        if profile == "gsd-strict" {
            strict(&p);
        }
        let before = p.codes();
        let mut empty = whole_edit();
        op(&mut empty, "plan", "03-02")["fields"]["requirements"] = json!([]);
        let refusal = mcp_edit(&p, &empty);
        assert_eq!(refusal["isError"], true, "{profile}: {refusal}");
        assert!(
            refusal.to_string().contains("gsd/E301"),
            "{profile}: {refusal}"
        );
        for file in FILES {
            assert!(!p.exists(file), "{profile}: a refused edit wrote {file}");
        }

        let edit = whole_edit();
        let written = mcp_edit(&p, &edit);
        assert_ne!(written["isError"], true, "{profile}: {written}");
        for (kind, children) in [
            ("plan", &["v1.0/03/03-01", "v1.0/03/03-02"][..]),
            ("task", &["v1.0/03/03-01/T1", "v1.0/03/03-02/T1"]),
            ("truth", &["v1.0/03/03-01/TR1", "v1.0/03/03-02/TR1"]),
            ("artifact", &["v1.0/03/03-01/AR1", "v1.0/03/03-02/AR1"]),
            ("key_link", &["v1.0/03/03-01/KL1", "v1.0/03/03-02/KL1"]),
            ("checkpoint", &["v1.0/03/03-02/CP1"]),
            ("prohibition", &["v1.0/03/03-02/PR1"]),
            ("user_setup", &["v1.0/03/03-02/US1"]),
        ] {
            let present = ids(&p, kind);
            for id in children {
                assert!(
                    present.iter().any(|i| i == id),
                    "{profile}: no {kind} {id} in {present:?}"
                );
            }
        }
        // Written as `link` spells it from plan 03-02: its sibling is `03-01` there.
        assert_eq!(
            p.fields_of("v1.0/03/03-02")["depends_on"],
            json!(["03-01"]),
            "{profile}"
        );
        assert_eq!(
            p.fields_of("v1.0/03/03-01/T1")["action"],
            ACTION,
            "{profile}"
        );
        let after = p.codes();
        assert!(
            after.iter().all(|c| c != "gsd/E301" && c != "gsd/W309"),
            "{profile}: {after:?}"
        );
        assert_eq!(
            added(&before, after),
            Vec::<String>::new(),
            "{profile}: added codes"
        );

        let files: Vec<String> = FILES.iter().map(|f| p.read(f)).collect();
        let again = mcp_edit(&p, &edit);
        assert_eq!(again["isError"], true, "{profile}: {again}");
        let text = again.to_string();
        assert!(
            text.contains("conflict") && text.contains("an entity with that id exists"),
            "{profile}: {text}"
        );
        for (file, was) in FILES.iter().zip(&files) {
            assert_eq!(
                &p.read(file),
                was,
                "{profile}: the refused resend changed {file}"
            );
        }
    }
}

/// A dependency found after the plans exist is linked with the plan format's `specforge link` line, filled: on the
/// planless phase, the plan edit without 03-02's `depends_on` is accepted; the line naming a plan of another phase is
/// refused `gsd/E102`, and naming its sibling 03-01 makes 03-02 depend on it, as the edit would have written it,
/// adding no code.
#[test]
fn a_dependency_found_later_is_linked() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let mut edit = whole_edit();
    let fields = op(&mut edit, "plan", "03-02")["fields"]
        .as_object_mut()
        .expect("the plan's fields");
    assert!(
        fields.remove("depends_on").is_some(),
        "the edit's second plan depends on the first"
    );
    let written = mcp_edit(&p, &edit);
    assert_ne!(written["isError"], true, "{written}");
    let link = |needed: &str| {
        typed::filled(
            &typed::typed_fence(&plan_format(), "A dependency found after the plans exist"),
            &[("PLAN_ID", "v1.0/03/03-02"), ("NEEDED_PLAN", needed)],
        )
    };
    let across = typed::apply(&sh, &p, &link("v1.0/02/02-01"));
    assert_ne!(across.code, 0, "{}{}", across.out, across.err);
    assert!(
        across.err.contains("gsd/E102") || across.out.contains("gsd/E102"),
        "{}{}",
        across.out,
        across.err
    );
    assert!(p.fields_of("v1.0/03/03-02")["depends_on"].is_null());
    let linked = typed::apply(&sh, &p, &link("v1.0/03/03-01"));
    assert_eq!(linked.code, 0, "{}{}", linked.out, linked.err);
    assert!(linked.added.is_empty(), "{:?}", linked.added);
    assert_eq!(p.fields_of("v1.0/03/03-02")["depends_on"], json!(["03-01"]));
}

/// The first line of the plan-by-plan route the planner used to be told is refused in every profile: under gsd-compat
/// E080 carries `gsd/E301` (LIST-01 is in no plan of the phase) and no `gsd/W309`, under gsd-strict both; nothing is
/// written, so the phase has no plan and the plan's file does not exist.
#[test]
fn a_plan_written_alone_is_refused_and_nothing_is_written() {
    for profile in ["gsd-compat", "gsd-strict"] {
        let p = taskboard();
        if profile == "gsd-strict" {
            strict(&p);
        }
        let run = p.run(&[
            "new",
            "plan",
            "03-01",
            "List open tasks",
            "--in",
            "spec/plans/03-01.spec",
            "--phase",
            "v1.0/03",
            "--type",
            "execute",
            "--files_modified",
            "src/cli.js",
            "--requirements",
            "TASK-03",
            "--objective",
            "List open tasks oldest first",
        ]);
        refused(&run, "E080", &["gsd/E301", "nothing was written"]);
        assert_eq!(
            run.all().contains("gsd/W309"),
            profile == "gsd-strict",
            "{profile}: {}",
            run.all()
        );
        let plans = ids(&p, "plan");
        assert!(
            plans.iter().all(|id| !id.starts_with("v1.0/03/")),
            "{profile}: {plans:?}"
        );
        assert!(!p.exists("spec/plans/03-01.spec"), "{profile}");
    }
}

/// The planner's `<plan_format>` is the plan edit: one `json` fence sent as **one** `specforge.edit` call, no typed
/// creation, the computed fields left out; the write step says to fix the list and send the whole call again, and the
/// critical rule names `specforge.edit`.
#[test]
fn the_plan_format_is_one_edit_of_the_whole_phase() {
    let format = plan_format();
    assert!(
        format.contains("## The Plan Edit") && format.contains("**one** MCP `specforge.edit` call"),
        "{format}"
    );
    let json = gsd_kit::markdown::fences(&format)
        .into_iter()
        .filter(|f| f.language == "json")
        .count();
    assert_eq!(json, 1, "the plan edit's json fences");
    let typed: Vec<&str> = format
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("specforge new "))
        .collect();
    assert!(typed.is_empty(), "{typed:?}");
    assert!(format.contains("computed"), "{format}");
    let body = planner();
    let write = block(&body, "<step name=\"write_phase_prompt\">", "</step>");
    assert!(
        write.contains("`specforge.edit`")
            && write.contains("send the whole call again")
            && !write.contains("run the line again"),
        "{write}"
    );
    let rule = line(&body, "- **No hand-written spec:**");
    assert!(rule.contains("`specforge.edit`"), "{rule}");
}
