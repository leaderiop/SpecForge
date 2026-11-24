//! Phase plans, their tasks and must-haves (and, from 13-06 Task 2, summaries and quick plans) through the command in
//! process, against the twins' `plans.spec` and `quick.spec`.
//!
//! Known differences from the twins, asserted as differences (13-RESEARCH "Per-Fixture Mapping", D5): `context_refs`
//! is absent (every fixture's `<context>` names only `.planning/` paths, which are dropped, while the twins name spec
//! files); a truth's title is its text and a key link's `<from> -> <to>` (the twins' titles are hand-made, and titles
//! compare nothing).

mod common;

use serde_json::{Value, json};
use specforge_extension_sdk::PlanOp;
use specforge_extension_sdk::prelude::*;

use common::{declared, fact, feed, fixture, import, import_fixture, manifest, payload};
use gsd_import::limits::MAX_OPS;

/// One create of the plan, with its graph path.
#[derive(Debug, Clone)]
struct Create {
    kind: String,
    path: String,
    title: String,
    file: String,
    fields: Value,
}

/// Each create of `out`'s plan; its path is its scope reference (`summary`, `plan` or `phase`) and its id.
fn creates(out: &CommandOutput) -> Vec<Create> {
    let plan = out.plan.as_ref().expect("import answers a plan");
    plan.ops
        .iter()
        .map(|op| match op {
            PlanOp::Create {
                kind,
                id,
                title,
                file,
                fields,
                ..
            } => {
                let fields = json!(fields);
                let scope = ["summary", "plan", "phase"]
                    .iter()
                    .find_map(|k| fields[*k].as_str());
                Create {
                    kind: kind.clone(),
                    path: scope.map_or_else(|| id.clone(), |s| format!("{s}/{id}")),
                    title: title.clone(),
                    file: file.clone(),
                    fields,
                }
            }
            other => panic!("import answers creates only, not {other:?}"),
        })
        .collect()
}

/// The create `kind path`.
fn find(all: &[Create], kind: &str, path: &str) -> Create {
    all.iter()
        .find(|c| c.kind == kind && c.path == path)
        .cloned()
        .unwrap_or_else(|| panic!("{kind} {path} is created"))
}

/// The paths of the creates of `kind`, in plan order.
fn paths<'c>(all: &'c [Create], kind: &str) -> Vec<&'c str> {
    all.iter()
        .filter(|c| c.kind == kind)
        .map(|c| c.path.as_str())
        .collect()
}

/// `(line, section, outcome, reason)` of each entry about `file`.
fn entries(out: &CommandOutput, file: &str) -> Vec<(u64, String, String, String)> {
    payload(out)["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .filter(|e| e["file"] == file)
        .map(|e| {
            let s = |k: &str| e[k].as_str().unwrap_or_default().to_string();
            (
                e["line"].as_u64().unwrap_or(0),
                s("section"),
                s("outcome"),
                s("reason"),
            )
        })
        .collect()
}

/// Whether `entries` holds one at `line` with `outcome` whose reason holds `says`.
fn has(entries: &[(u64, String, String, String)], line: u64, outcome: &str, says: &str) -> bool {
    entries
        .iter()
        .any(|(l, _, o, r)| *l == line && o == outcome && r.contains(says))
}

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

/// The manifest of the fixture `name`, listing `@gsd/security` when `security`, as the harness writes it (a fixture's
/// committed manifest lists no extension).
fn fixture_manifest(name: &str, security: bool) -> String {
    let text = std::fs::read_to_string(fixture(name).join("specforge.json")).expect("a manifest");
    let mut json: Value = serde_json::from_str(&text).expect("JSON");
    if security {
        json["extensions"] = json!(["@gsd/core", "@gsd/security=../../security/target/x.wasm"]);
    }
    json.to_string()
}

/// The fixture `name` imported, with or without `@gsd/security` in its manifest.
fn import_with(name: &str, security: bool) -> CommandOutput {
    import(
        &CommandGraph::default(),
        &feed(
            common::planning_facts(&fixture(name)),
            false,
            Some(&fixture_manifest(name, security)),
        ),
    )
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
