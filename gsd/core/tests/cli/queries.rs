//! The `specforge query` lines the ported text shows (REVIEW-r42 M3): upstream read a file glob, the port reads the
//! graph, and a query at the default depth of 1 returns nothing for a kind two or three hops away. Each line is run on
//! the taskboard twin, and it must find the kinds the twin has around its subject.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use regex::Regex;

use super::commands_support::{json, twin};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

fn assets() -> Vec<(String, String)> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let mut paths = Vec::new();
    walk(&base, &mut paths);
    paths.sort();
    paths
        .into_iter()
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

/// What each subject variable of a query line stands for on the twin (REVIEW-r43 M2: `$PROJECT` was not bound, so the
/// lines rooted there were never run). A subject outside the table and `UNBOUND` fails the test.
fn bound(subject: &str) -> Option<&'static str> {
    Some(match subject {
        "$PROJECT" => "taskboard",
        "$MILESTONE" | "${MILESTONE}" | "v${VERSION}" | "v{version}" | "${milestone_version}" => {
            "v1.0"
        }
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
        | "$MILESTONE/${TARGET_PHASE}"
        | "$MILESTONE/${PLAN_PHASE}"
        | "$MILESTONE/${NN}"
        | "$MILESTONE/${N}" => "v1.0/01",
        "$PLAN_ID" | "${TARGET_PLAN_ID}" => "v1.0/01/01-01",
        _ => return None,
    })
}

/// Subjects the twin cannot bind: a quick task, a placeholder the agent fills, a loop variable.
const UNBOUND: &[&str] = &[
    "${quick_id}",
    "$summary",
    "$s",
    "$_ID",
    "$r",
    "$c",
    "$TASK_ID",
    "$SUMMARY_FILE",
    "$uat_path",
    "$PLAN_REF",
    "$PHASE_NUM",
    "${PHASE_NUM}",
    "$PHASE_NUMBER",
    "${PHASE_NUMBER}-{plan_padded}",
    "${TARGET_PLAN}",
    "$((Z+1",
    "$MILESTONE/XX",
    "$PHASE_ID/${PHASE_NUM}-VERIFICATION",
    "v${VERSION}/{padded}",
    "{phase}-{plan}",
    "{phase_number}-{plan_padded}",
    "{selected-phase}",
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

/// The kinds a `project` links to (a requirement or a project-level decision): nothing links it to a milestone, a
/// phase, a pause, a handoff, a blocker or a lesson, so a `$PROJECT` query for those is empty (REVIEW-r43 M2).
const PROJECT_REACHES: &[&str] = &["decision"];

#[test]
fn a_query_reaches_the_kinds_it_asks_for() {
    let p = twin();
    let line =
        Regex::new(r#"specforge query +"?([^\s"|)`]+)"?((?: +--[a-z-]+(?: +[^\s|)`"]+)?)*)"#)
            .expect("a regex");
    let option = Regex::new(r"--([a-z-]+)(?: +([^\s|)`]+))?").expect("a regex");
    let nodes_of = |args: &[&str]| -> Vec<(String, String)> {
        let mut all = vec!["query"];
        all.extend_from_slice(args);
        all.extend_from_slice(&["--format", "brief"]);
        let run = p.run(&all);
        json(&run)["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .map(|n| {
                (
                    n["id"].as_str().expect("an id").to_string(),
                    n["kind"].as_str().expect("a kind").to_string(),
                )
            })
            .collect()
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
                    if !UNBOUND.contains(&&cap[1]) {
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
