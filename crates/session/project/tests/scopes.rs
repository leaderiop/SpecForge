//! A name is unique within its scope (ADR 0284, plan K2): what a project with no scoped kind keeps.

use std::collections::{HashMap, HashSet};

use specforge_component::committed::pinned_builtin_declarations;
use specforge_graph::{GraphBuild, written_tokens};
use specforge_parser::{SpecFile, parse};
use specforge_project::testing::Assembled;
use specforge_test_macros::test as specforge_test;

/// The builtins' registries as every example project compiles with them.
fn builtins() -> Assembled {
    Assembled::over(specforge_registry::build_registries(
        pinned_builtin_declarations().unwrap(),
    ))
}

/// The E002 and W060 (code, id) a flat `(kind, id)` / `id` rule gives `files` in path order: the rule as it was
/// before scopes, with r29's one spec block.
fn flat_rule(files: &[SpecFile]) -> Vec<(&'static str, String)> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut winners: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for file in files {
        for entity in &file.entities {
            let kind = entity.kind.raw.as_str();
            if matches!(kind, "defaults" | "define") {
                continue;
            }
            let id = entity.id.raw.to_string();
            let key = (
                kind.to_string(),
                if kind == "spec" {
                    kind.to_string()
                } else {
                    id.clone()
                },
            );
            if !seen.insert(key) {
                out.push(("E002", id));
            } else if !winners.insert(id.clone()) {
                out.push(("W060", id));
            }
        }
    }
    out
}

/// Check one project's files: ids, edges, rooted values and the duplicate diagnostics.
fn check(project: &Assembled, files: Vec<SpecFile>) {
    let mut files = files;
    files.sort_by_key(|f| f.path.to_string());
    let expected = flat_rule(&files);
    let env = project.compiled().environment();
    let build: GraphBuild = env.graph_of(files.clone());
    let graph = build.graph();
    let names: HashSet<String> = files
        .iter()
        .flat_map(|f| f.entities.iter().map(|e| e.id.raw.to_string()))
        .collect();
    for node in graph.nodes() {
        assert!(
            names.contains(node.id.raw.as_str()),
            "{} is no parsed name",
            node.id.raw
        );
    }
    let kind_of = |name: &str| graph.node(name).map(|n| n.kind.raw);
    let tokens: HashSet<(String, String)> = files
        .iter()
        .flat_map(|f| written_tokens(f, build.inputs(), kind_of))
        .flat_map(|t| {
            t.held_by
                .iter()
                .map(|h| (h.to_string(), t.target.to_string()))
                .collect::<Vec<_>>()
        })
        .collect();
    for edge in graph.edges() {
        let key = (edge.source.to_string(), edge.target.to_string());
        assert!(
            tokens.contains(&key),
            "edge {key:?} is no written reference"
        );
    }
    let got: Vec<(String, String)> = build
        .diagnostics()
        .iter()
        .filter(|d| matches!(d.code.as_str(), "E002" | "W060"))
        .map(|d| {
            let entity = match d.data.as_deref() {
                Some(specforge_common::DiagnosticData::Subject { entity }) => entity.clone(),
                other => panic!("no subject: {other:?}"),
            };
            (d.code.clone(), entity)
        })
        .collect();
    let expected: Vec<(String, String)> = expected
        .into_iter()
        .map(|(c, id)| (c.to_string(), id))
        .collect();
    assert_eq!(got, expected);
}

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "a project with no scoped kind keeps each id its name, each edge its written text, and E002 and W060 on the same declarations"
)]
fn an_unscoped_project_keeps_each_id_its_name_and_each_edge_its_written_text() {
    let project = builtins();
    let root = specforge_repo_root::repo_root();
    let corpus = specforge_parser::testing::corpus();
    // No value of the corpus starts with `/`: the rooted form reads nothing that parsed before.
    let mut specs: HashMap<bool, Vec<SpecFile>> = HashMap::new();
    for (path, text) in corpus {
        let file = parse(text, &path.to_string_lossy());
        for entity in &file.entities {
            for entry in entity.fields.entries() {
                if let specforge_parser::FieldValue::Identifier(word) = &entry.value {
                    assert!(!word.starts_with('/'), "{}: {word}", path.display());
                }
            }
        }
        specs
            .entry(path.starts_with(root.join("spec")))
            .or_default()
            .push(file.clone());
        check(&project, vec![file]);
    }
    check(&project, specs.remove(&true).unwrap());
}
