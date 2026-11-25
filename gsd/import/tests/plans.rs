//! Phase plans, their tasks and must-haves (and, from 13-06 Task 2, summaries and quick plans) through the command in
//! process, against the twins' `plans.spec` and `quick.spec`.
//!
//! Known differences from the twins, asserted as differences (13-RESEARCH "Per-Fixture Mapping", D5): `context_refs`
//! is absent (every fixture's `<context>` names only `.planning/` paths, which are dropped, while the twins name spec
//! files); a truth's title is its text and a key link's `<from> -> <to>` (the twins' titles are hand-made, and titles
//! compare nothing).

mod common;

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use common::{
    creates, declared, entries, fact, feed, find, fixture, has, import, import_fixture,
    import_with, manifest, paths,
};
use gsd_import::limits::MAX_OPS;

#[test]
fn taskboard_plans_take_the_twins_values() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(
        paths(&all, "plan"),
        ["v1.0/01/01-01", "v1.0/01/01-02", "v1.0/02/02-01"]
    );
    for (kind, n) in [("task", 6), ("truth", 6), ("artifact", 3), ("key_link", 3)] {
        assert_eq!(
            paths(&all, kind).len(),
            n,
            "{kind}: {:?}",
            paths(&all, kind)
        );
    }
    let plan = find(&all, "plan", "v1.0/01/01-01");
    assert_eq!(
        (plan.title.as_str(), plan.file.as_str()),
        ("The task model and add", "spec/gsd/phases/01.spec")
    );
    // The twin, plans.spec; `context_refs` is a known difference (D5).
    assert_eq!(
        plan.fields,
        json!({
            "phase": "v1.0/01",
            "type": "execute",
            "files_modified": ["package.json", "src/tasks.js", "test/tasks.test.js"],
            "requirements": ["TASK-01"],
            "objective": "The task model and `add`.",
            "purpose": "every later phase builds on a board of tasks with stable ids.",
            "output": "`src/tasks.js` with `createBoard` and `addTask`, and its tests.",
            "verification": ["`npm test` passes."],
            "success_criteria": ["TASK-01 holds: a user can add a task with a title"],
        })
    );
    let plan = find(&all, "plan", "v1.0/01/01-02");
    assert_eq!(plan.title, "complete and the command line");
    assert_eq!(plan.fields["depends_on"], json!(["v1.0/01/01-01"]));
    assert_eq!(
        find(&all, "plan", "v1.0/02/02-01").fields["requirements"],
        json!(["STORE-01", "STORE-02"])
    );
    let task = find(&all, "task", "v1.0/01/01-01/T1");
    assert_eq!(task.title, "The task model");
    assert_eq!(
        task.fields,
        json!({
            "plan": "v1.0/01/01-01",
            "type": "auto",
            "files": ["package.json", "src/tasks.js"],
            "action": "Create `package.json` (ES modules, `npm test` runs `node --test`) and `src/tasks.js` exporting `createBoard()` returning `{ tasks: [], nextId: 1 }` and `addTask(board, title)` that appends `{ id, title, done: false }` and bumps `nextId`.",
            "verify_text": "node -e \"import('./src/tasks.js').then(m => console.log(m.addTask(m.createBoard(), 'x')))\"",
            "done": "addTask returns a task with id 1 on an empty board",
        })
    );
    let truth = find(&all, "truth", "v1.0/02/02-01/TR3");
    assert_eq!(
        truth.fields,
        json!({"plan": "v1.0/02/02-01", "text": "A title comes back exactly as it was written (D-03)"})
    );
    // A known difference: the title is the text, the twin's is shortened.
    assert_eq!(
        truth.title,
        "A title comes back exactly as it was written (D-03)"
    );
    let artifact = find(&all, "artifact", "v1.0/01/01-01/AR1");
    assert_eq!(
        (artifact.title.as_str(), artifact.fields),
        (
            "createBoard and addTask",
            json!({"plan": "v1.0/01/01-01", "path": "src/tasks.js", "provides": "createBoard and addTask", "contains": "export function addTask"})
        )
    );
    let link = find(&all, "key_link", "v1.0/01/01-02/KL1");
    assert_eq!(
        (link.title.as_str(), link.fields),
        (
            "src/cli.js -> src/tasks.js",
            json!({"plan": "v1.0/01/01-02", "from": "src/cli.js", "to": "src/tasks.js", "via": "import { addTask, completeTask, createBoard }"})
        )
    );
    let reported = entries(&out, ".planning/phases/01-core/01-01-PLAN.md");
    for (line, outcome, says) in [
        (2, "redundant", "the file name gives them"),
        (3, "redundant", "the file name gives them"),
        (5, "computed", "plan.wave is computed"),
        (8, "computed", "plan.autonomous is computed"),
        (31, "unmapped", "GSD-internal: the spec replaces it"),
        (33, "unmapped", "GSD-internal: the spec replaces it"),
        (64, "unmapped", "an instruction to the executor"),
        (1, "mapped", "plan v1.0/01/01-01"),
        (38, "mapped", "task v1.0/01/01-01/T1"),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {outcome} {says}: {reported:#?}"
        );
    }
}

#[test]
fn ledger_plans_take_the_twins_values() {
    let out = import_fixture("ledger");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(
        paths(&all, "plan")
            .into_iter()
            .filter(|p| p.starts_with("v1.0/"))
            .collect::<Vec<_>>(),
        ["v1.0/01/01-01", "v1.0/01/01-02", "v1.0/02/02-01"]
    );
    let truths: Vec<&str> = paths(&all, "truth")
        .into_iter()
        .filter(|p| p.starts_with("v1.0/01/"))
        .collect();
    assert_eq!(
        truths,
        [
            "v1.0/01/01-01/TR1",
            "v1.0/01/01-01/TR2",
            "v1.0/01/01-02/TR1",
            "v1.0/01/01-02/TR2"
        ]
    );
    let plan = find(&all, "plan", "v1.0/01/01-02");
    assert_eq!(plan.title, "The total");
    assert_eq!(plan.fields["depends_on"], json!(["v1.0/01/01-01"]));
    // The twin writes the backslashes the source holds.
    assert_eq!(
        find(&all, "task", "v1.0/01/01-02/T2").fields["verify_text"],
        r"printf '100\ta\n250\tb\n' | cargo run -q -- total"
    );
}

#[test]
fn a_body_with_angle_brackets_and_quotes_is_kept_verbatim() {
    let text = std::fs::read_to_string(
        fixture("taskboard").join(".planning/phases/01-core/01-02-PLAN.md"),
    )
    .expect("the fixture");
    let source = |n: usize| {
        let after = text.split("<action>").nth(n).expect("an action");
        after.split("</action>").next().expect("closed").to_string()
    };
    let all = creates(&import_fixture("taskboard"));
    for (n, task) in [(1, "T1"), (2, "T2")] {
        let action = find(&all, "task", &format!("v1.0/01/01-02/{task}")).fields["action"].clone();
        assert_eq!(action, json!(source(n)), "{task}");
    }
    assert!(source(1).contains("`no task <id>`") && source(2).contains("`add <title>`"));
}

/// The import of a roadmap of phases 1 and 2 and the served `files`.
fn run(files: Vec<(&str, &str)>) -> CommandOutput {
    let road = "# Roadmap: X\n\n### Phase 1: One\n**Goal**: g\n**Success Criteria**:\n  1. c\n\n### Phase 2: Two\n**Goal**: h\n**Success Criteria**:\n  1. d\n";
    let mut facts = vec![fact(".planning/ROADMAP.md", road)];
    facts.extend(files.into_iter().map(|(path, text)| fact(path, text)));
    import(
        &CommandGraph::default(),
        &feed(facts, false, Some(&manifest(&["@gsd/core"]))),
    )
}

/// A plan whose frontmatter is `front` and whose body is `body`.
fn plan_text(front: &str, body: &str) -> String {
    format!(
        "---\ntype: execute\nfiles_modified: [a.js]\n{front}---\n\n<objective>\nDo it.\n</objective>\n\n{body}"
    )
}

#[test]
fn a_plan_reads_every_task_field_and_reports_the_rest() {
    let body = "<execution_context>\n@~/gsd/workflows/execute-plan.md\n</execution_context>\n\n<context>\n@.planning/PROJECT.md\n@src/a.js\n</context>\n\n<interfaces>\nx\n</interfaces>\n\n<tasks>\n<task type=\"tdd\">\n  <name>Task 1: Write it</name>\n  <files>a.js , b.js</files>\n  <read_first>\n    - src/a.js (the module)\n    - `docs/b.md`\n  </read_first>\n  <behavior>\n    - Test 1: it works\n  </behavior>\n  <action>Write `a <b>`.</action>\n  <verify>\n    <automated>npm test</automated>\n    <fails_when>a FAIL line</fails_when>\n    <automated>npm run lint</automated>\n  </verify>\n  <acceptance_criteria>\n    - `grep -c x a.js` prints 1\n    - it builds\n  </acceptance_criteria>\n  <done>It is written</done>\n</task>\n<task type=\"checkpoint:human-verify\" gate=\"blocking\">\n  <what-built>it</what-built>\n</task>\n</tasks>\n";
    let text = plan_text(
        "wave: 0\nautonomous: false\nestimate:\n  tokens: 10\nuser_setup: []\nodd_key: 1\nmust_haves:\n  truths:\n    - \"It works\"\n  artifacts:\n    - path: a.js\n      provides: \"it\"\n      min_lines: 12\n      exports: [run]\n      size: big\n  prohibitions:\n    - \"No b\"\n",
        body,
    );
    let path = ".planning/phases/01-one/01-01-PLAN.md";
    let out = run(vec![(path, &text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    let plan = find(&all, "plan", "v1.0/01/01-01");
    assert_eq!(plan.fields["wave0"], true);
    assert_eq!(plan.fields["context_refs"], json!(["src/a.js"]));
    // The roadmap names no title: the objective's first line, less its period, reported filled.
    assert_eq!(plan.title, "Do it");
    let task = find(&all, "task", "v1.0/01/01-01/T1");
    assert_eq!(
        task.fields,
        json!({
            "plan": "v1.0/01/01-01",
            "type": "tdd",
            "files": ["a.js", "b.js"],
            "read_first": ["src/a.js", "docs/b.md"],
            "action": "Write `a <b>`.",
            "verify": [
                {"automated": "npm test", "fails_when": "a FAIL line"},
                {"automated": "npm run lint"}
            ],
            "acceptance": ["`grep -c x a.js` prints 1", "it builds"],
            "done": "It is written",
        })
    );
    assert_eq!(task.title, "Write it");
    // The checkpoint is no task: only one task is created.
    assert_eq!(paths(&all, "task"), ["v1.0/01/01-01/T1"]);
    let artifact = find(&all, "artifact", "v1.0/01/01-01/AR1");
    assert_eq!(artifact.fields["min_lines"], 12);
    assert_eq!(artifact.fields["exports"], json!(["run"]));
    let reported = entries(&out, path);
    for (line, outcome, says) in [
        (4, "computed", "plan.wave is computed"),
        (5, "computed", "plan.autonomous is computed"),
        // A block value is located at its first line.
        (7, "unmapped", "a Tier 2 section"),
        (8, "unmapped", "a Tier 2 section"),
        (
            9,
            "unmapped",
            "no row of the NN-MM-PLAN.md table reports odd_key",
        ),
        (18, "unmapped", "a member `size` no row of the table names"),
        (20, "unmapped", "a Tier 2 section"),
        (24, "filled", "names no title for 01-01"),
        (27, "unmapped", "`specforge gsd reference` serves them"),
        (32, "unmapped", "GSD-internal"),
        (
            36,
            "unmapped",
            "no row of the NN-MM-PLAN.md table reports <interfaces>",
        ),
        (48, "unmapped", "a TDD task's expected behaviours"),
        (63, "unmapped", "a Tier 2 section"),
        (
            64,
            "unmapped",
            "no row of the NN-MM-PLAN.md table reports <what-built>",
        ),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {outcome} {says}: {reported:#?}"
        );
    }
}

#[test]
fn the_file_name_wins_over_the_frontmatter_and_the_directory() {
    let text = plan_text("phase: 02-two\nplan: 03\n", "");
    let path = ".planning/phases/02-two/01-01-PLAN.md";
    let out = run(vec![(path, &text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(paths(&all, "plan"), ["v1.0/01/01-01"]);
    assert_eq!(
        find(&all, "plan", "v1.0/01/01-01").file,
        "spec/gsd/phases/01.spec"
    );
    let reported = entries(&out, path);
    assert!(
        has(
            &reported,
            1,
            "unmapped",
            "the directory `02-two` names another phase"
        ),
        "{reported:#?}"
    );
    assert!(
        has(
            &reported,
            4,
            "unmapped",
            "`phase: 02-two` disagrees with the file name 01-01"
        ),
        "{reported:#?}"
    );
    assert!(
        has(
            &reported,
            5,
            "unmapped",
            "`plan: 03` disagrees with the file name 01-01"
        ),
        "{reported:#?}"
    );
    // A file name that gives no plan id, and a phase the roadmap lacks, import nothing.
    let out = run(vec![
        (".planning/phases/01-one/1-1-PLAN.md", &text),
        (".planning/phases/07-x/07-01-PLAN.md", &text),
    ]);
    assert!(paths(&creates(&out), "plan").is_empty());
    assert!(has(
        &entries(&out, ".planning/phases/01-one/1-1-PLAN.md"),
        1,
        "unmapped",
        "no plan id"
    ));
    assert!(has(
        &entries(&out, ".planning/phases/07-x/07-01-PLAN.md"),
        1,
        "unmapped",
        "phase 07 is no phase of ROADMAP.md"
    ));
}

#[test]
fn an_unclosed_task_element_keeps_the_tasks_before_it() {
    let body = "<tasks>\n<task type=\"auto\">\n  <name>Task 1: One</name>\n  <action>a</action>\n  <done>d</done>\n</task>\n<task type=\"auto\">\n  <name>Task 2: Two</name>\n  <action>never closed\n";
    let path = ".planning/phases/01-one/01-01-PLAN.md";
    let out = run(vec![(path, &plan_text("", body))]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(paths(&creates(&out), "task"), ["v1.0/01/01-01/T1"]);
    assert!(
        has(
            &entries(&out, path),
            16,
            "unmapped",
            "is not closed by </task>"
        ),
        "{:#?}",
        entries(&out, path)
    );
}

#[test]
fn a_task_past_max_ops_refuses_the_import() {
    let task = "<task type=\"auto\">\n<action>a</action>\n<done>d</done>\n</task>\n";
    let body = task.repeat(MAX_OPS + 1);
    let out = run(vec![(
        ".planning/phases/01-one/01-01-PLAN.md",
        &plan_text("", &body),
    )]);
    assert_ne!(out.exit_code, 0);
    assert!(out.plan.is_none_or(|p| p.ops.is_empty()));
    assert!(
        out.stderr.contains(&format!("truncated at {MAX_OPS}")),
        "{}",
        out.stderr
    );
}

#[test]
fn no_record_holds_a_computed_field() {
    let declared = declared();
    let mut checked = 0;
    for (name, security) in [
        ("taskboard", false),
        ("ledger", false),
        ("vault", false),
        ("vault", true),
    ] {
        let out = import_with(name, security);
        assert_eq!(out.exit_code, 0, "{name}: {}", out.stderr);
        for c in creates(&out) {
            for (field, value) in c.fields.as_object().expect("fields") {
                let mut keys = vec![format!("{}.{field}", c.kind)];
                for block in value.as_array().into_iter().flatten() {
                    for member in block.as_object().into_iter().flat_map(|m| m.keys()) {
                        keys.push(format!("{}.{field}.{member}", c.kind));
                    }
                }
                for key in keys {
                    checked += 1;
                    let writer = declared.get(&key).unwrap_or_else(|| {
                        panic!(
                            "{name}: {} {} writes {key}, no declared field",
                            c.kind, c.path
                        )
                    });
                    assert!(
                        matches!(writer.as_str(), "" | "harness"),
                        "{name}: {} {} writes {key}, whose writer is `{writer}`",
                        c.kind,
                        c.path
                    );
                }
            }
        }
    }
    assert!(checked > 200, "{checked}");
}

#[test]
fn taskboard_summary_and_deliverable_values_match_the_twin() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    assert_eq!(
        paths(&all, "summary"),
        ["v1.0/01/01-01/01-01-SUMMARY", "v1.0/01/01-02/01-02-SUMMARY"]
    );
    assert_eq!(
        paths(&all, "deliverable"),
        [
            "v1.0/01/01-01/01-01-SUMMARY/D1",
            "v1.0/01/01-02/01-02-SUMMARY/D1",
            "v1.0/01/01-02/01-02-SUMMARY/D2"
        ]
    );
    let summary = find(&all, "summary", "v1.0/01/01-01/01-01-SUMMARY");
    assert_eq!(
        (summary.title.as_str(), summary.file.as_str()),
        ("The task model and add: summary", "spec/gsd/phases/01.spec")
    );
    // The twin, plans.spec, exactly.
    assert_eq!(
        summary.fields,
        json!({
            "plan": "v1.0/01/01-01",
            "status": "complete",
            "one_liner": "A board of plain data with `addTask`, ids counting up from 1 and never reused.",
            "subsystem": "tasks",
            "tags": ["node", "esm"],
            "provides": ["createBoard and addTask"],
            "affects": ["01-02", "persistence"],
            "tech_patterns": ["a board is plain data: { tasks, nextId }"],
            "requirements_completed": ["TASK-01"],
        })
    );
    // A known difference (D6): the twin carries no `requires`; the block form is imported.
    let summary = find(&all, "summary", "v1.0/01/01-02/01-02-SUMMARY");
    assert_eq!(
        summary.fields["requires"],
        json!([{"phase": "v1.0/01", "provides": "createBoard and addTask"}])
    );
    assert_eq!(
        find(&all, "deliverable", "v1.0/01/01-02/01-02-SUMMARY/D1").fields,
        json!({
            "summary": "v1.0/01/01-02/01-02-SUMMARY",
            "description": "A task is completed by its id, and an unknown id is refused by name",
            "requirement": "TASK-02",
            "human_judgment": false,
            "checks": [
                {"kind": "unit", "ref": "test/tasks.test.js#a task is completed by its id"},
                {"kind": "unit", "ref": "test/tasks.test.js#completing an unknown id throws"}
            ],
        })
    );
    assert_eq!(
        find(&all, "deliverable", "v1.0/01/01-02/01-02-SUMMARY/D2").fields,
        json!({
            "summary": "v1.0/01/01-02/01-02-SUMMARY",
            "description": "The command line adds and completes tasks",
            "human_judgment": true,
            "rationale": "The output wording is a judgement a person makes at the terminal",
        })
    );
    let reported = entries(&out, ".planning/phases/01-core/01-02-SUMMARY.md");
    for (line, outcome, says) in [
        (2, "redundant", "the file name gives them"),
        (20, "computed", "summary.files_created is computed"),
        (24, "unmapped", "a Tier 2 section"),
        (35, "computed", "deliverable.checks.status is computed"),
        (38, "computed", "deliverable.checks.status is computed"),
        (46, "computed", "summary.duration is computed"),
        (
            47,
            "unmapped",
            "a harness timestamp the import does not write",
        ),
        (51, "redundant", "the plan's title names the summary"),
        (55, "unmapped", "prose: no field holds it"),
        (65, "unmapped", "git history is the collector's"),
        (70, "unmapped", "a Tier 2 section"),
        (78, "unmapped", "a Tier 2 section"),
        (1, "mapped", "summary v1.0/01/01-02/01-02-SUMMARY"),
        (29, "mapped", "deliverable v1.0/01/01-02/01-02-SUMMARY/D1"),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {outcome} {says}: {reported:#?}"
        );
    }
}

#[test]
fn vault_summary_reports_a_requires_of_plan_ids_and_its_threat_flags() {
    let path = ".planning/phases/01-core/01-02-SUMMARY.md";
    let out = import_with("vault", false);
    let all = creates(&out);
    let summary = find(&all, "summary", "v1.0/01/01-02/01-02-SUMMARY");
    assert_eq!(summary.fields.get("requires"), None);
    assert_eq!(summary.fields.get("threat_flags"), None);
    let reported = entries(&out, path);
    assert!(
        has(&reported, 7, "unmapped", "names no `provides`"),
        "{reported:#?}"
    );
    assert!(
        has(&reported, 45, "needs-extension", "@gsd/security"),
        "{reported:#?}"
    );
    // A known difference (D15): with @gsd/security, the twin carries no threat flags; the import does.
    let all = creates(&import_with("vault", true));
    assert_eq!(
        find(&all, "summary", "v1.0/01/01-02/01-02-SUMMARY").fields["threat_flags"],
        json!([
            "threat_flag: information_disclosure | src/cli.js | the secret is an argv entry, visible in `ps` and in shell history"
        ])
    );
}

#[test]
fn ledger_quick_plan_and_summary_match_the_twin() {
    let out = import_fixture("ledger");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    let plan = find(&all, "plan", "261105-tcl");
    assert_eq!(
        (plan.title.as_str(), plan.file.as_str()),
        ("Show totals in a fixed-width column", "spec/gsd/quick.spec")
    );
    // The twin, quick.spec: no phase.
    assert_eq!(
        plan.fields,
        json!({
            "type": "execute",
            "slug": "totals-column",
            "files_modified": ["src/lib.rs", "src/main.rs", "tests/totals.rs"],
            "objective": "Show totals in a fixed-width column.",
            "purpose": "totals line up when a user prints several stores one under another.",
            "output": "`column(cents)` in `src/lib.rs`, used by `ledger total`.",
            "success_criteria": ["A total prints right-aligned in a column of fourteen"],
        })
    );
    assert_eq!(
        find(&all, "task", "261105-tcl/T1").fields,
        json!({
            "plan": "261105-tcl",
            "type": "auto",
            "files": ["src/lib.rs", "src/main.rs", "tests/totals.rs"],
            "action": "Add `column(cents)`: euros and cents, right-aligned in 14 characters; `ledger total` prints through it. Fold the total in an `i32` so the column's width is the value's width.",
            "verify_text": "cargo test --test totals a_total_prints_in_a_column_of_fourteen",
            "done": "A total prints in a column of fourteen",
        })
    );
    assert_eq!(
        find(&all, "truth", "261105-tcl/TR1").fields["text"],
        "A total prints right-aligned in a column of fourteen"
    );
    assert_eq!(
        find(&all, "artifact", "261105-tcl/AR1").fields,
        json!({"plan": "261105-tcl", "path": "src/lib.rs", "provides": "column", "contains": "pub fn column"})
    );
    let summary = find(&all, "summary", "261105-tcl/261105-tcl-SUMMARY");
    assert_eq!(
        summary.title,
        "Show totals in a fixed-width column: summary"
    );
    // A known difference (D14): the twin's subsystem is `entries`; a quick summary has none, so it is filled.
    assert_eq!(
        summary.fields,
        json!({
            "plan": "261105-tcl",
            "status": "complete",
            "one_liner": "`ledger total` prints its total right-aligned in a column of fourteen.",
            "subsystem": "quick",
            "requirements_completed": [],
        })
    );
    let reported = entries(
        &out,
        ".planning/quick/261105-tcl-totals-column/261105-tcl-SUMMARY.md",
    );
    for (line, outcome, says) in [
        (1, "filled", "a quick summary has no subsystem"),
        (1, "filled", "a quick summary lists no requirements"),
        (2, "redundant", "the quick plan gives them"),
        (3, "redundant", "the quick plan gives them"),
        (
            28,
            "unmapped",
            "a section no row of the quick/* table names",
        ),
    ] {
        assert!(
            has(&reported, line, outcome, says),
            "{line} {outcome} {says}: {reported:#?}"
        );
    }
    let reported = entries(
        &out,
        ".planning/quick/261105-tcl-totals-column/261105-tcl-PLAN.md",
    );
    assert!(
        has(&reported, 6, "computed", "plan.autonomous is computed"),
        "{reported:#?}"
    );
}

#[test]
fn a_quick_context_and_research_are_reported_tier_2() {
    let dir = ".planning/quick/261201-abc-x";
    let out = run(vec![
        (&format!("{dir}/261201-abc-CONTEXT.md"), "# Context\n"),
        (&format!("{dir}/261201-abc-RESEARCH.md"), "# Research\n"),
        (&format!("{dir}/notes.md"), "x\n"),
    ]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    for name in ["CONTEXT", "RESEARCH"] {
        let reported = entries(&out, &format!("{dir}/261201-abc-{name}.md"));
        assert_eq!(reported.len(), 1, "{reported:#?}");
        assert!(
            has(&reported, 0, "unmapped", "a Tier 2 section"),
            "{reported:#?}"
        );
    }
    assert!(has(
        &entries(&out, &format!("{dir}/notes.md")),
        0,
        "unmapped",
        "no row names it"
    ));
}

/// A summary of the plan 01-01 with `front` added to its frontmatter.
fn summary_text(front: &str) -> String {
    format!(
        "---\nphase: 01-one\nplan: 01\nsubsystem: core\nstatus: complete\nrequirements-completed: []\n{front}---\n\n# Plan 01 Summary\n\n**It works.**\n"
    )
}

#[test]
fn a_summary_without_its_plan_is_not_imported() {
    let path = ".planning/phases/01-one/01-01-SUMMARY.md";
    let out = run(vec![(path, &summary_text(""))]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert!(paths(&creates(&out), "summary").is_empty());
    assert!(has(
        &entries(&out, path),
        1,
        "unmapped",
        "no plan for this summary"
    ));
}

#[test]
fn a_second_plan_of_one_id_keeps_its_tasks_to_itself() {
    let task = |n: u32| {
        format!(
            "<task type=\"auto\">\n  <name>Task {n}: T</name>\n  <action>a</action>\n  <done>d</done>\n</task>\n"
        )
    };
    let (old, one) = (
        ".planning/phases/01-old/01-01-PLAN.md",
        ".planning/phases/01-one/01-01-PLAN.md",
    );
    let first = plan_text("", &format!("<tasks>\n{}</tasks>\n", task(1)));
    let second = plan_text("", &format!("<tasks>\n{}{}</tasks>\n", task(1), task(2)));
    // Read in path order: `01-old` claims the plan id first.
    let out = run(vec![(old, &first), (one, &second)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(paths(&creates(&out), "task"), ["v1.0/01/01-01/T1"]);
    let reported = entries(&out, one);
    assert!(
        has(&reported, 1, "unmapped", "is given twice"),
        "{reported:#?}"
    );
    let orphans = reported
        .iter()
        .filter(|(_, _, _, r)| r.contains("parent not imported (v1.0/01/01-01, given twice)"))
        .count();
    assert_eq!(orphans, 2, "{reported:#?}");
    assert!(
        !reported.iter().any(|(_, _, o, _)| o == "mapped"),
        "{reported:#?}"
    );
}

#[test]
fn a_heading_outside_every_element_or_under_a_summary_title_is_reported() {
    let plan = ".planning/phases/01-one/01-01-PLAN.md";
    let summary = ".planning/phases/01-one/01-01-SUMMARY.md";
    let body = "# Plan 01-01\n\nIntro prose.\n\n<tasks>\n<task type=\"auto\">\n  <name>Task 1: T</name>\n  \
        <action>a</action>\n  <done>d</done>\n</task>\n</tasks>\n\n## Notes\n\nSome prose\nover two lines.\n";
    let summary_body = format!(
        "{}\n### Stray\n\nx\n\n## Accomplishments\n\n### Sub\n",
        summary_text("")
    );
    let out = run(vec![(plan, &plan_text("", body)), (summary, &summary_body)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(paths(&creates(&out), "task"), ["v1.0/01/01-01/T1"]);
    let reported = entries(&out, plan);
    let outside: Vec<u64> = reported
        .iter()
        .filter(|(_, _, o, r)| o == "unmapped" && r.contains("outside every element"))
        .map(|(l, ..)| *l)
        .collect();
    assert_eq!(outside, [10, 12, 22, 24], "{reported:#?}");
    let reported = entries(&out, summary);
    assert!(
        has(
            &reported,
            13,
            "unmapped",
            "a section no row of the NN-MM-SUMMARY.md table"
        ),
        "{reported:#?}"
    );
    assert!(!reported.iter().any(|(l, ..)| *l == 19), "{reported:#?}");
}

#[test]
fn a_deliverable_and_a_requires_report_each_member_they_do_not_read() {
    let path = ".planning/phases/01-one/01-01-SUMMARY.md";
    let front = "requires:\n  - phase: 01-one\n    provides: x\n    via: y\ncoverage:\n  - id: d1\n    \
        description: D\n    human_judgment: false\n    requirement: [A-01, B-02]\n    verification:\n      \
        - kind: unit\n        ref: t1\n        command: npm test\n      - plain\n  - id: d2\n    description: E\n    \
        human_judgment: false\n    verification: npm test\n";
    let out = run(vec![
        (".planning/phases/01-one/01-01-PLAN.md", &plan_text("", "")),
        (path, &summary_text(front)),
    ]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    let summary = find(&all, "summary", "v1.0/01/01-01/01-01-SUMMARY");
    assert_eq!(
        summary.fields["requires"],
        json!([{"phase": "v1.0/01", "provides": "x"}])
    );
    let d1 = find(&all, "deliverable", "v1.0/01/01-01/01-01-SUMMARY/d1");
    assert_eq!(d1.fields["checks"], json!([{"kind": "unit", "ref": "t1"}]));
    assert_eq!(d1.fields.get("requirement"), None);
    let reported = entries(&out, path);
    for (line, says) in [
        (10, "a requires member `via` no row of the table names"),
        (15, "`requirement` is not one requirement id"),
        (19, "a check member `command` no row of the table names"),
        (20, "a check that is not a map"),
        (24, "`verification` is not a list"),
    ] {
        assert!(
            has(&reported, line, "unmapped", says),
            "{line} {says}: {reported:#?}"
        );
    }
}

#[test]
fn a_summary_writes_no_harness_timestamp() {
    let path = ".planning/phases/01-one/01-01-SUMMARY.md";
    let out = run(vec![
        (".planning/phases/01-one/01-01-PLAN.md", &plan_text("", "")),
        (
            path,
            &summary_text("started: 2026-03-03\ncompleted: 2026-03-04\n"),
        ),
    ]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let all = creates(&out);
    let summary = find(&all, "summary", "v1.0/01/01-01/01-01-SUMMARY");
    assert_eq!(
        summary.fields,
        json!({"plan": "v1.0/01/01-01", "status": "complete", "one_liner": "It works.", "subsystem": "core", "requirements_completed": []})
    );
    assert_eq!(summary.title, "Do it: summary");
    let reported = entries(&out, path);
    for line in [7, 8] {
        assert!(
            has(
                &reported,
                line,
                "unmapped",
                "a harness timestamp the import does not write"
            ),
            "{line}: {reported:#?}"
        );
    }
}
