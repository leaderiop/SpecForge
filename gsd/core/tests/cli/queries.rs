//! The `specforge query` lines the ported text shows (REVIEW-r42 M3): upstream read a file glob, the port reads the
//! graph, and a query at the default depth of 1 returns nothing for a kind two or three hops away. Each line is run on
//! the taskboard twin, and it must find the kinds the twin has around its subject.

use std::collections::{BTreeMap, BTreeSet};
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

#[test]
fn a_query_reaches_the_kinds_it_asks_for() {
    let p = twin();
    let subject = Regex::new(
        r#"specforge query "?\$\{?(PHASE_ID|MILESTONE|PLAN_ID)\}?"?((?: +--[a-z-]+(?: +[^\s|)`"]+)?)*)"#,
    )
    .expect("a regex");
    let option = Regex::new(r"--([a-z-]+)(?: +([^\s|)`]+))?").expect("a regex");
    let ids = BTreeMap::from([
        ("PHASE_ID", "v1.0/01"),
        ("MILESTONE", "v1.0"),
        ("PLAN_ID", "v1.0/01/01-01"),
    ]);
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
    let mut empty = Vec::new();
    for (path, text) in assets() {
        for (n, line) in text.lines().enumerate() {
            for cap in subject.captures_iter(line) {
                let id = ids[&cap[1]];
                let (mut kinds, mut depth) = (String::new(), "1".to_string());
                for o in option.captures_iter(&cap[2]) {
                    match &o[1] {
                        "kinds" => kinds = o.get(2).map_or("", |m| m.as_str()).to_string(),
                        "depth" => depth = o.get(2).map_or("1", |m| m.as_str()).to_string(),
                        _ => {}
                    }
                }
                if kinds.is_empty() {
                    continue;
                }
                let found = kinds_of(&[id, "--kinds", &kinds, "--depth", &depth]);
                for kind in kinds.split(',') {
                    // the twin has the kind below the subject (a requirement belongs to no phase)
                    let below = nodes_of(&[id, "--kinds", kind, "--depth", "6"])
                        .into_iter()
                        .any(|(n, k)| k == kind && (n.starts_with(id) || k == "requirement"));
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
        empty.is_empty(),
        "queries that return nothing:\n{}",
        empty.join("\n")
    );
}
