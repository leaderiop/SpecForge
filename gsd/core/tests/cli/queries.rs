//! The `specforge query` lines the ported text shows: upstream read a file glob, the port reads the
//! graph, and a query at the default depth of 1 returns nothing for a kind two or three hops away. Each line is run on
//! the taskboard twin, and it must find the kinds the twin has around its subject.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use regex::Regex;

use super::commands_support::{json, twin};
use super::files;

fn assets() -> Vec<(String, String)> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    files::under(&base)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .map(|p| {
            let name = p
                .strip_prefix(&base)
                .expect("under assets")
                .display()
                .to_string();
            (name, std::fs::read_to_string(&p).expect("an asset"))
        })
        .collect()
}

#[test]
fn no_query_lacks_its_entity() {
    let rx = Regex::new(r"specforge query +--").expect("a regex");
    for (path, text) in assets() {
        for (n, line) in text.lines().enumerate() {
            assert!(
                !rx.is_match(line),
                "{path}:{n}: `specforge query` needs an entity: {line}"
            );
        }
    }
}

/// What each subject variable of a query line stands for on the twin (`$PROJECT` was not bound, so the
/// lines rooted there were never run). A subject outside the table and `UNBOUND` fails the test.
fn bound(subject: &str) -> Option<&'static str> {
    Some(match subject {
        "$PROJECT" => "taskboard",
        "$MILESTONE"
        | "${MILESTONE}"
        | "$UNDO_MILESTONE"
        | "${UNDO_MILESTONE:-${PHASE_DIR%%/*}}"
        | "v${VERSION}"
        | "v{version}"
        | "${milestone_version}" => "v1.0",
        "$PHASE_ID"
        | "${PHASE_ID}"
        | "${PHASE}"
        | "${CURRENT_PHASE}"
        | "$PHASE1"
        | "$PHASE_DIR"
        | "${PHASE_DIR}"
        | "${PARENT_PHASE}"
        | "$MILESTONE/${PHASE_NUM}"
        | "$MILESTONE/01"
        | "$MILESTONE/$NEXT_PHASE"
        | "$MILESTONE/${TARGET_PHASE}"
        | "$UNDO_MILESTONE/${TARGET_PHASE}"
        | "$MILESTONE/${PLAN_PHASE}"
        | "$UNDO_MILESTONE/${PLAN_PHASE}"
        | "$MILESTONE/${NN}"
        | "$UNDO_MILESTONE/${NN}"
        | "{selected-phase}"
        | "{PHASE_ID}"
        | "$MILESTONE/${N}"
        | "$UNDO_MILESTONE/${N}" => "v1.0/01",
        "$PLAN_ID" | "$PLAN_ENTITY" | "$PLAN_FULL_ID" | "${TARGET_PLAN_ID}" | "$plan_id"
        | "$plan" => "v1.0/01/01-01",
        // facts: `the_verifiers_fact_spot_checks_print_what_they_promise` collects them first
        "$TEST_ID" => TWIN_TEST,
        "js:$PACKAGE/$MODULE_FILE::$FUNCTION_NAME" => TWIN_FUNCTION,
        _ => return None,
    })
}

/// Subjects the twin cannot bind: a quick task, a placeholder the agent fills, a loop variable.
/// The reading-list queries a quick prompt builds from `quick_id` in its `${... ? '...' : ''}` form (their subject
/// reads `'`): `quick_context::a_quick_reading_list_query_reaches_the_context_decisions_and_findings` runs them.
const QUICK_READING: &[&str] = &[
    "specforge query ' + quick_id + '-CONTEXT --depth 2 --kinds context,decision --format context`",
    "specforge query ' + quick_id + '-RESEARCH --depth 1 --format context`",
];

const UNBOUND: &[&str] = &[
    "${quick_id}",
    "${quick_id}-RESEARCH",
    "${quick_id}/${quick_id}-SUMMARY",
    // a quick batch: the twin holds none; `assets_shell::batch` records one and runs these
    "$BATCH_ID",
    "$RESUME_BATCH_ID",
    "$QB_ITEM",
    "$summary",
    "$s",
    "$_ID",
    "$r",
    "$c",
    "$ph",
    "$t",
    "$p",
    "$m",
    "$TASK_ID",
    "$SPEC_ID",
    "$SPEC_FILE",
    "$SUMMARY_FILE",
    "$SUMMARY_ID",
    "$RESEARCH_ID",
    // the context the PRD path just recorded and the research the researcher returned (`assets_shell::plan_edits`)
    "$CONTEXT_ID",
    "<research>",
    "{REVIEW_ID}",
    // the latest review round, read by the line above it: `assets_shell::plan_edits` records one and runs them
    "$REVIEW_ID",
    "<review>",
    "$uat_path",
    // entity ids a prompt hands its agent, each read with `--format context`
    "{roadmap_path}",
    "{context_path}",
    "{research_path}",
    "{verification_path}",
    "{uat_path}",
    "{PATTERNS_PATH}",
    "{AI_SPEC_PATH}",
    "{UI_SPEC_PATH}",
    "{SPEC_PATH}",
    "$PLAN_REF",
    "$PHASE_NUM",
    "${PHASE_NUM}",
    "$PHASE_NUMBER",
    "${PHASE_NUMBER}-{plan_padded}",
    "${TARGET_PLAN}",
    "$((Z+1",
    "$MILESTONE/XX",
    "$MILESTONE/N",
    "$PHASE_ID/${PHASE_NUM}-VERIFICATION",
    "v${VERSION}/{padded}",
    "{phase}-{plan}",
    "{phase_number}-{plan_padded}",
    "{dir}",
    "<N>",
    "<phase>",
    "<entity>",
    "<id>",
    "<truth>",
    "<verification>",
    "<package_audit>",
    "<capability-keyword>",
    "<quick",
    "RESEARCH",
    "RESEARCH`)",
];

/// A test fact of the twin, once `collect_node_test_golden` has run.
const TWIN_TEST: &str = "js:taskboard/test/tasks.test.js#a task is added with the next id";
/// An item fact of the twin, once `@javascript/source` has collected.
const TWIN_FUNCTION: &str = "js:taskboard/src/tasks.js::addTask";

/// The kinds a `project` links to (a requirement or a project-level decision): nothing links it to a milestone, a
/// phase, a pause, a handoff, a blocker or a lesson, so a `$PROJECT` query for those is empty.
const PROJECT_REACHES: &[&str] = &["decision"];

#[test]
fn a_query_reaches_the_kinds_it_asks_for() {
    let p = twin();
    let line =
        Regex::new(r#"specforge query +"?([^\s"|)`]+)"?((?: +--[a-z-]+(?: +[^\s|)`"]+)?)*)"#)
            .expect("a regex");
    let option = Regex::new(r"--([a-z-]+)(?: +([^\s|)`]+))?").expect("a regex");
    // The assets repeat their queries: each distinct one runs once.
    let seen = RefCell::new(HashMap::<Vec<String>, Vec<(String, String)>>::new());
    let nodes_of = |args: &[&str]| -> Vec<(String, String)> {
        let key: Vec<String> = args.iter().map(ToString::to_string).collect();
        if let Some(nodes) = seen.borrow().get(&key) {
            return nodes.clone();
        }
        let mut all = vec!["query"];
        all.extend_from_slice(args);
        all.extend_from_slice(&["--format", "brief"]);
        let run = p.run(&all);
        let nodes: Vec<(String, String)> = json(&run)["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .map(|n| {
                (
                    n["id"].as_str().expect("an id").to_string(),
                    n["kind"].as_str().expect("a kind").to_string(),
                )
            })
            .collect();
        seen.borrow_mut().insert(key, nodes.clone());
        nodes
    };
    let kinds_of = |args: &[&str]| -> BTreeSet<String> {
        nodes_of(args).into_iter().map(|(_, kind)| kind).collect()
    };
    let mut checked = 0;
    let (mut empty, mut unknown) = (Vec::new(), BTreeSet::new());
    for (path, text) in assets() {
        for (n, text_line) in text.lines().enumerate() {
            for cap in line.captures_iter(text_line) {
                let Some(id) = bound(&cap[1]) else {
                    let quick =
                        &cap[1] == "'" && QUICK_READING.iter().any(|q| text_line.contains(q));
                    if !quick && !UNBOUND.contains(&&cap[1]) {
                        unknown.insert(format!("{path}:{n}: {}", &cap[1]));
                    }
                    continue;
                };
                let (mut kinds, mut depth) = (String::new(), "1".to_string());
                for o in option.captures_iter(&cap[2]) {
                    match &o[1] {
                        "kinds" => kinds = o.get(2).map_or("", |m| m.as_str()).to_string(),
                        "depth" => depth = o.get(2).map_or("1", |m| m.as_str()).to_string(),
                        _ => {}
                    }
                }
                if id == "taskboard" {
                    // static: the project reaches only what it links to, whatever the twin holds
                    let wrong: Vec<&str> = kinds
                        .split(',')
                        .filter(|k| !k.is_empty() && !PROJECT_REACHES.contains(k))
                        .collect();
                    if !wrong.is_empty() || (kinds.is_empty() && depth != "0") {
                        empty.push(format!(
                            "{path}:{n}: `{}` reads {} from the project, which links only {PROJECT_REACHES:?} (use `specforge export --kinds ...`)",
                            &cap[0],
                            if kinds.is_empty() { "its neighbours".into() } else { kinds.clone() }
                        ));
                    }
                }
                if kinds.is_empty() {
                    continue;
                }
                let found = kinds_of(&[id, "--kinds", &kinds, "--depth", &depth]);
                // the twin's kinds below the subject (a requirement belongs to no phase; the project is the twin)
                let scope = if id == "taskboard" { "v1.0" } else { id };
                for kind in kinds.split(',') {
                    let below = nodes_of(&[scope, "--kinds", kind, "--depth", "6"])
                        .into_iter()
                        .any(|(n, k)| k == kind && (n.starts_with(scope) || k == "requirement"));
                    if below {
                        checked += 1;
                        if !found.contains(kind) {
                            empty.push(format!(
                                "{path}:{n}: `{}` finds no {kind} at depth {depth}",
                                &cap[0]
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 10, "{checked} kinds checked");
    assert!(
        unknown.is_empty(),
        "a query subject the test neither binds nor lists as unbound:\n{}",
        unknown.into_iter().collect::<Vec<_>>().join("\n")
    );
    assert!(
        empty.is_empty(),
        "queries that return nothing:\n{}",
        empty.join("\n")
    );
}

/// The verifier's spot-checks of collected facts (`agents/gsd-verifier.md`: an exported function, a test that exists,
/// a test's status), each `specforge query` as written, run on the twin once its tests and items are collected, its
/// subject bound to a fact of the twin.
#[test]
fn the_verifiers_fact_spot_checks_print_what_they_promise() {
    let p = twin();
    let tests = p.collect_node_test_golden();
    assert_eq!(tests.code, 0, "{}", tests.all());
    let items = p.collect(&["--runner", "javascript-source"]);
    assert_eq!(items.code, 0, "{}", items.all());
    let verifier = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/agents/gsd-verifier.md");
    let text = std::fs::read_to_string(verifier).expect("the verifier's prompt");
    let line =
        Regex::new(r#"specforge query "([^"]+)" --depth 0 --pick '([^']+)'"#).expect("a regex");
    let mut printed = Vec::new();
    for cap in text.lines().flat_map(|l| line.captures_iter(l)) {
        if ![TWIN_TEST, TWIN_FUNCTION].contains(&bound(&cap[1]).unwrap_or_default()) {
            continue;
        }
        let run = p.run(&[
            "query",
            bound(&cap[1]).expect("bound"),
            "--depth",
            "0",
            "--pick",
            &cap[2],
        ]);
        assert_eq!(run.code, 0, "{}: {}", &cap[0], run.all());
        printed.push((cap[2].to_string(), run.stdout.trim().to_string()));
    }
    assert_eq!(
        printed,
        [
            (
                ".nodes[0].fields.kind[]".to_string(),
                "function".to_string()
            ),
            (".nodes[0].id".to_string(), TWIN_TEST.to_string()),
            (".nodes[0].fields.status".to_string(), "passed".to_string()),
        ]
    );
}

/// A name git refuses is no branch: the field is absent, and the check says why.
#[test]
fn a_branch_name_git_refuses_is_w409_and_no_field() {
    let p = twin();
    let branch = |id: &str| {
        let answer = json(&p.run(&["query", id, "--depth", "0", "--format", "json"]));
        answer["nodes"][0]["fields"]["branch"].clone()
    };
    for template in [
        "--help",
        "-evil{phase}",
        "a..{slug}",
        "{slug}.lock",
        "x;touch pwned",
    ] {
        p.settings(serde_json::json!({
            "git.branching_strategy": "phase",
            "git.phase_branch_template": template,
        }));
        assert!(branch("v1.0/01").is_null(), "{template}");
        assert!(
            p.codes().contains(&"gsd/W409".to_string()),
            "{template}: {:?}",
            p.report()
        );
    }
    p.settings(serde_json::json!({
        "git.branching_strategy": "milestone",
        "git.milestone_branch_template": "--help",
    }));
    assert!(branch("v1.0").is_null());
    assert!(branch("v1.0/01").is_null());
    let w409 = p.codes().iter().filter(|c| *c == "gsd/W409").count();
    assert_eq!(
        w409,
        1,
        "the milestone's, not one per phase: {:?}",
        p.report()
    );
}

/// The branch a phase and its milestone execute on, under each `git.branching_strategy`.
#[test]
fn a_phase_and_its_milestone_answer_their_branch() {
    let p = twin();
    let branch = |id: &str| {
        let answer = json(&p.run(&["query", id, "--depth", "0", "--format", "json"]));
        answer["nodes"][0]["fields"]["branch"].clone()
    };
    assert!(branch("v1.0/01").is_null(), "the default strategy is none");
    p.settings(serde_json::json!({"git.branching_strategy": "phase"}));
    assert_eq!(branch("v1.0/01"), "gsd/phase-01-core");
    assert!(branch("v1.0").is_null(), "a milestone has none under phase");
    p.settings(serde_json::json!({
        "git.branching_strategy": "phase",
        "git.phase_branch_template": "work/{phase}/{slug}",
    }));
    assert_eq!(branch("v1.0/01"), "work/01/core");
    p.settings(serde_json::json!({"git.branching_strategy": "milestone"}));
    assert_eq!(branch("v1.0"), "gsd/v1.0-taskboard-1-0");
    assert_eq!(branch("v1.0/01"), "gsd/v1.0-taskboard-1-0");
    p.settings(serde_json::json!({"git.branching_strategy": "none"}));
    assert!(branch("v1.0/01").is_null());
    assert!(branch("v1.0").is_null());
}
