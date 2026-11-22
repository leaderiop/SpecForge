//! The planner's plan edit: the `json` fence of its `<plan_format>`, filled and sent as one MCP `specforge.edit` call
//! on the taskboard twin's planless phase v1.0/03 (requirements TASK-03 and LIST-01) under gsd-compat and gsd-strict,
//! and the plan-by-plan route beside it, whose findings a later plan settles (ADR 0385).

use serde_json::{Value, json};

use super::{Shell, asset, block, line, mcp_edit, span, typed};
use crate::commands_support::{checked, refused, strict};
use crate::harness::{Project, is_judged, taskboard};

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

/// `(code, severity, entity)` of each judged rule's diagnostic `check --format json` reports on an entity whose id
/// starts with `prefix`, sorted. The entity is the quoted id a judged message names first.
fn owed(p: &Project, prefix: &str) -> Vec<(String, String, String)> {
    let run = p.run(&["check", "--format", "json"]);
    let found: Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    let mut owed: Vec<(String, String, String)> = found
        .as_array()
        .expect("a list of diagnostics")
        .iter()
        .filter(|d| d["code"].as_str().is_some_and(is_judged))
        .filter_map(|d| {
            let message = d["message"].as_str().expect("a message");
            let entity = message
                .split('\'')
                .nth(1)
                .unwrap_or_else(|| panic!("a judged message names its entity: {message}"));
            entity.starts_with(prefix).then(|| {
                let code = d["code"].as_str().expect("a code");
                let severity = d["severity"].as_str().expect("a severity");
                (code.to_string(), severity.to_string(), entity.to_string())
            })
        })
        .collect();
    owed.sort();
    owed
}

/// The two plan files the edit writes.
const FILES: [&str; 2] = ["spec/plans/03-01.spec", "spec/plans/03-02.spec"];

/// On the planless phase, under gsd-compat and under gsd-strict, the plan edit with 03-02 naming TASK-03 in place of
/// LIST-01 is written and reports `gsd/E301` (LIST-01 is in no plan of the phase), which `check` fails on: the finding
/// is settled later (ADR 0385), by the plan that holds LIST-01. With 03-02 naming no requirement it is refused whole
/// (`gsd/E302`, which no later step settles) and writes no file. The edit as filled is accepted in one call, writes
/// both plans with their task, truth, artifact and key link (and 03-02's checkpoint, prohibition and user setup step),
/// reads the task action back exactly and adds no code outside the judged rules. It leaves each plan owing exactly the
/// plan checker's four judged findings (`gsd/W311`, `gsd/W313`, `gsd/W315`: errors under gsd-strict, warnings under
/// gsd-compat; `gsd/W316`: a warning in both), which E080 does not count (ADR 0297). Sent again, it is refused as a
/// conflict and leaves both files byte for byte.
#[test]
fn the_planner_writes_a_phase_in_one_edit() {
    for profile in ["gsd-compat", "gsd-strict"] {
        let p = taskboard();
        if profile == "gsd-strict" {
            strict(&p);
        }
        let before = p.codes();
        assert_eq!(owed(&p, "v1.0/03/"), Vec::new(), "{profile}");
        let uncovered = taskboard();
        if profile == "gsd-strict" {
            strict(&uncovered);
        }
        let mut empty = whole_edit();
        op(&mut empty, "plan", "03-02")["fields"]["requirements"] = json!([]);
        let refusal = mcp_edit(&uncovered, &empty);
        assert_eq!(refusal["isError"], true, "{profile}: {refusal}");
        assert!(
            refusal.to_string().contains("gsd/E302"),
            "{profile}: {refusal}"
        );
        for file in FILES {
            assert!(
                !uncovered.exists(file),
                "{profile}: a refused edit wrote {file}"
            );
        }
        let mut short = whole_edit();
        op(&mut short, "plan", "03-02")["fields"]["requirements"] = json!(["TASK-03"]);
        let reported = mcp_edit(&uncovered, &short);
        assert_ne!(reported["isError"], true, "{profile}: {reported}");
        assert!(
            reported.to_string().contains("gsd/E301"),
            "{profile}: {reported}"
        );
        for file in FILES {
            assert!(
                uncovered.exists(file),
                "{profile}: the edit wrote no {file}"
            );
        }
        assert_eq!(
            checked(&uncovered, "gsd/E301"),
            (true, vec![("Error".to_string(), "v1.0/03".to_string())]),
            "{profile}"
        );

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
        let raised = if profile == "gsd-strict" {
            "Error"
        } else {
            "Warning"
        };
        let mut want = Vec::new();
        for plan in ["v1.0/03/03-01", "v1.0/03/03-02"] {
            for (code, severity) in [
                ("gsd/W311", raised),
                ("gsd/W313", raised),
                ("gsd/W315", raised),
                ("gsd/W316", "Warning"),
            ] {
                want.push((code.to_string(), severity.to_string(), plan.to_string()));
            }
        }
        want.sort();
        assert_eq!(owed(&p, "v1.0/03/"), want, "{profile}: the judged findings");

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
    let sh = Shell::only(&[]);
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

/// A plan written alone by a typed creation, before the phase's other plans, is accepted in every profile: the phase's
/// coverage (`gsd/E301`: LIST-01 is in no plan of the phase) and, under gsd-strict, the plan's must-haves (`gsd/W309`)
/// are settled later (ADR 0385), so the write reports them and `check` fails on them until the rest is written. Under
/// gsd-compat `gsd/W309` is a warning.
#[test]
fn a_plan_written_alone_is_written_and_reports_what_a_later_write_settles() {
    for (profile, must_haves) in [("gsd-compat", "Warning"), ("gsd-strict", "Error")] {
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
        assert_eq!(run.code, 0, "{profile}: {}", run.all());
        for code in ["gsd/E301", "gsd/W309"] {
            assert!(run.all().contains(code), "{profile}: {code}: {}", run.all());
        }
        assert!(
            ids(&p, "plan").iter().any(|id| id == "v1.0/03/03-01"),
            "{profile}"
        );
        assert!(p.exists("spec/plans/03-01.spec"), "{profile}");
        assert_eq!(
            checked(&p, "gsd/E301"),
            (true, vec![("Error".to_string(), "v1.0/03".to_string())]),
            "{profile}"
        );
        assert_eq!(
            checked(&p, "gsd/W309").1,
            [(must_haves.to_string(), "v1.0/03/03-01".to_string())],
            "{profile}"
        );
    }
}

/// The planner's `<plan_format>` is the plan edit: one `json` fence sent as **one** `specforge.edit` call, the atomic
/// route, no typed creation, the computed fields left out. It says a call that leaves a finding a later plan settles
/// (gsd/E301, gsd/E310, gsd/W309) is written, not refused, and no longer that a plan written alone is refused; the
/// write step says to fix a refused list and send the whole call again, and to complete a phase a written call left
/// incomplete; the critical rule names `specforge.edit`.
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
    let flat = format.split_whitespace().collect::<Vec<_>>().join(" ");
    for words in [
        "It is the atomic route",
        "`gsd/E310`",
        "a call that leaves one is written, not refused",
        "`check` fails on it until the rest of the phase is written",
    ] {
        assert!(
            flat.contains(words),
            "the plan format lacks {words}:\n{flat}"
        );
    }
    assert!(!flat.contains("a plan written alone is refused"), "{flat}");
    let body = planner();
    let write = block(&body, "<step name=\"write_phase_prompt\">", "</step>");
    assert!(
        write.contains("`specforge.edit`")
            && write.contains("send the whole call again")
            && write.contains("left the phase incomplete")
            && !write.contains("run the line again"),
        "{write}"
    );
    let rule = line(&body, "- **No hand-written spec:**");
    assert!(rule.contains("`specforge.edit`"), "{rule}");
}

/// Every other writer of a phase's plans names the plan edit and none points at the planner's typed edits: chunked
/// single-plan mode (its reference and its spawn prompt, which names no typed plan creation), gap closure and the
/// quick-batch planner wave. Chunked mode says a plan written alone reports `gsd/E301` or `gsd/E310` and is written,
/// and that `## PLAN BLOCKED` is a refused call only; its wait reads a `## PLAN BLOCKED` return as `blocked`, in both
/// of its paths, never as a stall; its recovery of a blocked entry retries the plan or plans the rest of the phase in
/// one run, naming the plans the graph holds with a query line, since the planner's plan edit creates only the others;
/// and it says a run stopped midway leaves the phase half-planned, failing `check` and refused by execute-phase.
#[test]
fn every_writer_of_a_phases_plans_names_the_plan_edit() {
    let chunked = asset("gsd-core/references/planner-chunked.md");
    let gap = asset("gsd-core/references/planner-gap-closure.md");
    let mode = asset("gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md");
    let wave = asset("gsd-core/workflows/quick-batch/steps/planner-wave.md");
    let single = block(&chunked, "### single-plan", "## Resume Behaviour");
    let spawn = block(
        &mode,
        "**Chunked mode: single-plan.**",
        "Return: ## PLAN COMPLETE",
    );
    let recovery = block(
        &mode,
        "8. **Recovery:**",
        "The plans already committed stay",
    );
    let wait = block(&mode, "5. **ORCHESTRATOR RULE", "6. **Verify disk");
    for (name, text) in [
        ("single-plan", &single),
        ("gap closure", &block(&gap, "**8. Write", "**9.")),
        ("the chunked spawn", &spawn),
        (
            "the quick-batch wave",
            &block(&wave, "<output>", "</output>"),
        ),
    ] {
        assert!(text.contains("plan edit"), "{name}: {text}");
    }
    for (name, text) in [
        ("planner-chunked.md", &chunked),
        ("planner-gap-closure.md", &gap),
        ("chunked-planning-mode.md", &mode),
        ("planner-wave.md", &wave),
        ("gsd-planner.md", &planner()),
    ] {
        assert!(!text.contains("typed edits of the planner"), "{name}");
    }
    assert!(!spawn.contains("`specforge new plan`"), "{spawn}");
    for (name, text) in [("single-plan", &single), ("the chunked spawn", &spawn)] {
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        for words in [
            "gsd/E301",
            "gsd/E310",
            "## PLAN BLOCKED",
            "written",
            "refused",
        ] {
            assert!(flat.contains(words), "{name} lacks {words}: {flat}");
        }
        assert!(!flat.contains("is refused `gsd/E301`"), "{name}: {flat}");
    }
    assert!(
        spawn.contains("holding this plan alone.\n"),
        "the spawn prompt's sentence ends: {spawn}"
    );
    for words in [
        "a line holding `## PLAN BLOCKED` -> `blocked`",
        "each `## PLAN BLOCKED` return like\n     `blocked`",
        "`marker_received`, `blocked` or `stalled`",
    ] {
        assert!(wait.contains(words), "the wait lacks {words}:\n{wait}");
    }
    for words in [
        "## PLAN BLOCKED",
        "gsd/E301",
        "never block",
        "1) Retry",
        "Plan the rest of the phase in one run",
        "specforge query \"$PHASE_ID\" --kinds plan",
        "conflict",
    ] {
        assert!(
            recovery.contains(words),
            "the recovery lacks {words}:\n{recovery}"
        );
    }
    assert!(
        single
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("to retry this plan, to plan the rest of the phase in one run, or to stop"),
        "{single}"
    );
    let stopped = block(&mode, "A run stopped before", "Move to the next Wave");
    assert!(
        stopped.contains("half-planned") && stopped.contains("`/gsd-execute-phase`"),
        "{stopped}"
    );
    assert!(
        plan_format().contains("the edit creates only the others"),
        "the plan edit says nothing of the plans that exist"
    );
}

/// The plan edit's plans written one at a time, as chunked mode writes them: plan 03-01 alone, with its task, truth,
/// artifact and key link (the plan edit's ops of plan 03-01, taken by their file), is accepted under gsd-compat and
/// under gsd-strict while LIST-01 is in no plan; the edit reports `gsd/E301` and no `gsd/W309` (the plan has its
/// must-haves), and `check` fails on the phase until plan 03-02, written next, holds LIST-01. Deleting 03-02 again is
/// refused: a delete creates nothing, so the coverage it would break is an error it adds (E080).
#[test]
fn a_phases_plans_written_one_at_a_time_are_each_accepted_and_the_last_settles_coverage() {
    let edit = plan_edit();
    let of_plan = |plan: &str| -> Vec<Value> {
        let mut edit = edit.clone();
        let file = op(&mut edit, "plan", plan)["file"].clone();
        edit.as_array()
            .expect("a list of edits")
            .iter()
            .filter(|e| e["file"] == file)
            .cloned()
            .collect()
    };
    let first = of_plan("03-01");
    let kinds: Vec<&str> = first.iter().filter_map(|e| e["kind"].as_str()).collect();
    assert_eq!(kinds, ["plan", "task", "truth", "artifact", "key_link"]);
    for profile in ["gsd-compat", "gsd-strict"] {
        let p = taskboard();
        if profile == "gsd-strict" {
            strict(&p);
        }
        let written = mcp_edit(&p, &Value::Array(first.clone()));
        assert_ne!(written["isError"], true, "{profile}: {written}");
        let text = written.to_string();
        assert!(text.contains("gsd/E301"), "{profile}: {text}");
        assert!(!text.contains("gsd/W309"), "{profile}: {text}");
        assert!(p.exists(FILES[0]), "{profile}");
        assert_eq!(
            checked(&p, "gsd/E301"),
            (true, vec![("Error".to_string(), "v1.0/03".to_string())]),
            "{profile}"
        );

        let written = mcp_edit(&p, &Value::Array(of_plan("03-02")));
        assert_ne!(written["isError"], true, "{profile}: {written}");
        assert!(
            !written.to_string().contains("gsd/E301"),
            "{profile}: {written}"
        );
        assert_eq!(checked(&p, "gsd/E301").1, Vec::new(), "{profile}");

        let file = p.read(FILES[1]);
        let deleted = p.run(&["delete", "v1.0/03/03-02"]);
        refused(&deleted, "E080", &["gsd/E301"]);
        assert_eq!(
            p.read(FILES[1]),
            file,
            "{profile}: a refused delete changed {}",
            FILES[1]
        );
    }
}

/// The rest of a chunked run, as the recovery of a blocked entry writes it. A plan of phase v1.0/03 that covers both
/// requirements exists (written alone, which `gsd/E301` accepts then); under gsd-compat and gsd-strict the
/// whole-phase plan edit is refused as a conflict and writes nothing, while the edit without the plans the recovery's
/// own query line lists (03-01) is accepted and writes 03-02.
#[test]
fn the_rest_of_a_chunked_run_writes_only_the_plans_the_graph_lacks() {
    let sh = Shell::only(&[]);
    let mode = asset("gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md");
    let recovery = block(
        &mode,
        "8. **Recovery:**",
        "The plans already committed stay",
    );
    let query = span(&recovery, "specforge query");
    for profile in ["gsd-compat", "gsd-strict"] {
        let p = taskboard();
        if profile == "gsd-strict" {
            strict(&p);
        }
        let mut edit = plan_edit();
        op(&mut edit, "plan", "03-01")["fields"]["requirements"] = json!(["TASK-03", "LIST-01"]);
        let file = op(&mut edit, "plan", "03-01")["file"].clone();
        let first: Vec<Value> = edit
            .as_array()
            .expect("a list of edits")
            .iter()
            .filter(|e| e["file"] == file)
            .cloned()
            .collect();
        let written = mcp_edit(&p, &Value::Array(first));
        assert_ne!(written["isError"], true, "{profile}: {written}");

        let whole = mcp_edit(&p, &plan_edit());
        assert_eq!(whole["isError"], true, "{profile}: {whole}");
        assert!(whole.to_string().contains("conflict"), "{profile}: {whole}");
        assert!(
            !p.exists(FILES[1]),
            "{profile}: a refused edit wrote {}",
            FILES[1]
        );

        let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID='v1.0/03'\n{query}"));
        assert_eq!(code, 0, "{profile}: {query}: {err}");
        let listed: Vec<&str> = out
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(listed, ["v1.0/03/03-01"], "{profile}: {query}");
        let rest: Vec<Value> = plan_edit()
            .as_array()
            .expect("a list of edits")
            .iter()
            .filter(|e| {
                let plan = match e["kind"].as_str() {
                    Some("plan") => format!("v1.0/03/{}", e["id"].as_str().expect("an id")),
                    _ => e["fields"]["plan"].as_str().expect("a plan").to_string(),
                };
                !listed.contains(&plan.as_str())
            })
            .cloned()
            .collect();
        let written = mcp_edit(&p, &Value::Array(rest));
        assert_ne!(written["isError"], true, "{profile}: {written}");
        assert!(
            ids(&p, "plan").iter().any(|id| id == "v1.0/03/03-02"),
            "{profile}"
        );
    }
}
