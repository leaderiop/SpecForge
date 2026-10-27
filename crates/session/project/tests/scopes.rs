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
    let tokens: HashSet<(String, String)> = files
        .iter()
        .flat_map(|f| written_tokens(f, build.inputs(), graph.scopes()))
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

// -- K2 pins: a name is unique within its scope; each is ignored until the ticket that fixes it ------------------

use specforge_graph::{FileChange, Form};
use specforge_protocol_types::{
    EntityKindDescriptor, ExtensionDeclaration, FieldDescriptor, HandshakeResponse,
};
use specforge_registry::PlainDisk;

/// The graph build of `files` over the GSD-shaped kinds (`Assembled::scoped`).
fn scoped(files: &[(&str, &str)]) -> GraphBuild {
    let project = Assembled::scoped();
    let env = project.compiled().environment();
    env.graph_of(files.iter().map(|(path, text)| parse(text, path)))
}

fn ids(build: &GraphBuild) -> Vec<String> {
    build
        .graph()
        .nodes()
        .iter()
        .map(|n| n.id.raw.to_string())
        .collect()
}

fn codes(build: &GraphBuild, code: &str) -> Vec<String> {
    build
        .diagnostics()
        .iter()
        .filter(|d| d.code == code)
        .map(|d| d.message.clone())
        .collect()
}

/// The targets of the edges `source` writes in `field`, sorted.
fn targets(build: &GraphBuild, source: &str, field: &str) -> Vec<String> {
    let mut found: Vec<String> = build
        .graph()
        .edges_from(source)
        .iter()
        .filter(|e| e.label == field)
        .map(|e| e.target.to_string())
        .collect();
    found.sort();
    found
}

/// Two milestones, each with phase 03, each with a decision D-01.
const TWO_MILESTONES: &str = r#"
milestone v1.0 "One" { }
milestone v2.0 "Two" { }
phase 03 "Phase Three" { milestone v1.0 }
phase 03 "Phase Three" { milestone v2.0 }
decision D-01 "Choice" { phase v1.0/03 }
decision D-01 "Choice" { phase v2.0/03 }
"#;

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "a name repeated in another scope is another entity with another path"
)]
fn a_name_repeats_in_another_scope() {
    let build = scoped(&[("a.spec", TWO_MILESTONES)]);
    assert_eq!(
        ids(&build),
        [
            "v1.0",
            "v1.0/03",
            "v1.0/03/D-01",
            "v2.0",
            "v2.0/03",
            "v2.0/03/D-01"
        ]
    );
    assert!(
        codes(&build, "E002").is_empty(),
        "{:?}",
        build.diagnostics()
    );
}

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "a name repeated in one scope is E002 naming the path, or W060 for another kind, and a second spec block stays E002"
)]
fn a_name_repeated_in_one_scope_is_e002_naming_the_path() {
    let build = scoped(&[(
        "a.spec",
        r#"
milestone v1.0 "One" { }
phase 03 "Phase Three" { milestone v1.0 }
decision D-01 "Choice" { phase v1.0/03 }
decision D-01 "Again" { phase v1.0/03 }
plan D-01 "Plan" { phase v1.0/03 }
spec "one" { }
spec "other" { }
"#,
    )]);
    let e002 = codes(&build, "E002");
    assert_eq!(e002.len(), 2, "{e002:?}");
    assert!(
        e002[0].starts_with("duplicate entity ID 'v1.0/03/D-01'"),
        "{e002:?}"
    );
    assert!(
        e002[1].starts_with("a second spec block 'other'"),
        "{e002:?}"
    );
    let w060 = codes(&build, "W060");
    assert_eq!(w060.len(), 1, "{w060:?}");
    assert!(w060[0].contains("'v1.0/03/D-01'"), "{w060:?}");
}

#[specforge_test(
    behavior = "link_derived_references",
    verify = "a type name resolves by its holder's chain and is no E004, a path written as a type is one name"
)]
fn a_type_name_resolves_by_its_chain_and_is_no_e004() {
    let build = scoped(&[(
        "a.spec",
        r#"
milestone v1.0 "One" { }
phase 03 "Phase Three" { milestone v1.0 }
shape Money "Money" { phase v1.0/03 }
shape Invoice "Invoice" {
  phase v1.0/03
  amount Money
}
shape Bill "Bill" {
  phase v1.0/03
  amount v1.0/03/Money
}
shape Lost "Lost" {
  phase v1.0/03
  amount Nowhere
}
"#,
    )]);
    assert!(
        build.graph().node("v1.0/03/Money").is_some(),
        "{:?}",
        ids(&build)
    );
    assert_eq!(
        targets(&build, "v1.0/03/Invoice", "parts"),
        ["v1.0/03/Money"]
    );
    assert_eq!(targets(&build, "v1.0/03/Bill", "parts"), ["v1.0/03/Money"]);
    let e004 = codes(&build, "E004");
    assert_eq!(e004.len(), 1, "{e004:?}");
    assert!(e004[0].contains("'Nowhere'"), "{e004:?}");
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a reference resolves in its holder, each enclosing scope, then the project"
)]
fn a_reference_resolves_from_its_holder_outward() {
    let build = scoped(&[(
        "a.spec",
        r#"
milestone v1.0 "One" { audit A1 }
audit A1 "Audit" { milestone v1.0 }
phase 03 "Phase Three" { milestone v1.0 }
decision D-01 "Choice" { phase v1.0/03 }
decision D-09 "Project choice" { }
plan 03-02 "Plan" {
  phase v1.0/03
  decisions [D-01, D-09]
}
"#,
    )]);
    assert!(
        codes(&build, "E003").is_empty(),
        "{:?}",
        build.diagnostics()
    );
    assert_eq!(
        targets(&build, "v1.0/03/03-02", "decisions"),
        ["D-09", "v1.0/03/D-01"]
    );
    assert_eq!(targets(&build, "v1.0", "audit"), ["v1.0/A1"]);
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a path resolves at the nearest level where the whole path resolves"
)]
fn a_path_resolves_where_the_whole_path_does() {
    let build = scoped(&[(
        "a.spec",
        r#"
milestone v1.0 "One" { }
milestone v2.0 "Two" { }
phase 03 "Phase Three" { milestone v1.0 }
phase 04 "Phase Four" { milestone v1.0 }
phase 03 "Phase Three" { milestone v2.0 }
decision D-01 "Choice" { phase v1.0/03 }
decision D-01 "Choice" { phase v1.0/04 }
decision D-01 "Choice" { phase v2.0/03 }
plan 03-02 "Plan" {
  phase v1.0/03
  decisions [04/D-01, v2.0/03/D-01]
}
task v1.0 "A task named like a milestone" { plan v1.0/03/03-02 }
"#,
    )]);
    assert!(
        codes(&build, "E003").is_empty(),
        "{:?}",
        build.diagnostics()
    );
    assert_eq!(
        targets(&build, "v1.0/03/03-02", "decisions"),
        ["v1.0/04/D-01", "v2.0/03/D-01"]
    );
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a rooted path names an entity from the project, and a bare name is found in the chain first"
)]
fn a_rooted_path_names_from_the_project() {
    let build = scoped(&[(
        "a.spec",
        r#"
decision D-01 "Project choice" { }
milestone v1.0 "One" { }
phase 03 "Phase Three" { milestone v1.0 }
decision D-01 "Phase choice" { phase v1.0/03 }
plan 05-01 "Plan" {
  phase v1.0/03
  decisions [D-01, /D-01]
}
"#,
    )]);
    assert!(
        codes(&build, "E003").is_empty(),
        "{:?}",
        build.diagnostics()
    );
    assert_eq!(
        targets(&build, "v1.0/03/05-01", "decisions"),
        ["D-01", "v1.0/03/D-01"]
    );
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a name outside the holder's chain is E003 naming each entity that bears it, and never links silently"
)]
fn a_name_outside_the_chain_is_e003_naming_each_candidate() {
    let build = scoped(&[(
        "a.spec",
        r#"
milestone v1.0 "One" { }
milestone v2.0 "Two" { }
phase 03 "Phase Three" { milestone v1.0 }
phase 04 "Phase Four" { milestone v1.0 }
phase 05 "Phase Five" { milestone v2.0 }
decision D-01 "Choice" { phase v1.0/03 }
decision D-01 "Choice" { phase v1.0/04 }
decision D-07 "Lone" { phase v1.0/03 }
plan 05-01 "Plan" {
  phase v2.0/05
  decisions [D-01, D-07]
}
"#,
    )]);
    let candidates = |entity: &str, reference: &str| -> Vec<String> {
        let found = build
            .diagnostics()
            .iter()
            .find(|d| {
                d.code == "E003"
                    && matches!(d.data.as_deref(), Some(specforge_common::DiagnosticData::UnresolvedReference { target, entity: e, .. }) if target == reference && e == entity)
            })
            .unwrap_or_else(|| panic!("no E003 for {reference}: {:?}", build.diagnostics()));
        assert!(
            found.message.contains("not in its scope chain"),
            "{}",
            found.message
        );
        serde_json::to_value(found.data.as_deref()).unwrap()["candidates"]
            .as_array()
            .unwrap_or_else(|| panic!("no candidates: {}", found.message))
            .iter()
            .map(|c| c.as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(
        candidates("v2.0/05/05-01", "D-01"),
        ["v1.0/03/D-01", "v1.0/04/D-01"]
    );
    assert_eq!(candidates("v2.0/05/05-01", "D-07"), ["v1.0/03/D-07"]);
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a scope field takes a unique suffix, and several matches are E003 naming each"
)]
fn a_scope_field_takes_a_unique_suffix() {
    let one = scoped(&[(
        "a.spec",
        "milestone v1.0 \"One\" { }\nphase 03 \"Three\" { milestone v1.0 }\ndecision D-01 \"D\" { phase 03 }\n",
    )]);
    assert!(
        one.graph().node("v1.0/03/D-01").is_some(),
        "{:?}",
        ids(&one)
    );
    assert!(codes(&one, "E003").is_empty(), "{:?}", one.diagnostics());

    let two = scoped(&[(
        "a.spec",
        "milestone v1.0 \"One\" { }\nmilestone v2.0 \"Two\" { }\n\
         phase 03 \"Three\" { milestone v1.0 }\nphase 03 \"Three\" { milestone v2.0 }\n\
         decision D-01 \"D\" { phase 03 }\n",
    )]);
    let e003 = codes(&two, "E003");
    assert_eq!(e003.len(), 1, "{e003:?}");
    assert!(
        e003[0].contains("ambiguous reference '03'")
            && e003[0].contains("v1.0/03")
            && e003[0].contains("v2.0/03"),
        "{e003:?}"
    );
}

#[specforge_test(
    behavior = "detect_duplicate_entity_ids",
    verify = "an entity that writes no scope field is in the project scope"
)]
fn an_absent_scope_field_is_the_project() {
    let build = scoped(&[(
        "a.spec",
        "milestone v1.0 \"One\" { }\nphase 03 \"Three\" { milestone v1.0 }\n\
         decision D-09 \"Project\" { }\ndecision D-09 \"Phase\" { phase v1.0/03 }\n",
    )]);
    assert_eq!(ids(&build), ["D-09", "v1.0", "v1.0/03", "v1.0/03/D-09"]);
    assert!(
        codes(&build, "E002").is_empty(),
        "{:?}",
        build.diagnostics()
    );
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "an entity whose scope does not resolve keeps its written scope in its path and reports E003 on the field"
)]
fn an_unresolved_scope_keeps_its_written_path() {
    let build = scoped(&[("a.spec", "decision D-01 \"D\" { phase 99 }\n")]);
    assert_eq!(ids(&build), ["99/D-01"]);
    let e003 = codes(&build, "E003");
    assert_eq!(e003.len(), 1, "{e003:?}");
    assert!(e003[0].contains("'99'"), "{e003:?}");
}

#[specforge_test(
    behavior = "rebuild_affected_subgraph",
    verify = "an apply that moves a scope re-keys its members held in other files, equal to a cold build"
)]
fn a_renamed_scope_rekeys_its_members_incrementally() {
    let mut build = scoped(&[
        (
            "a.spec",
            "milestone v1.0 \"One\" { }\nmilestone v2.0 \"Two\" { }\nphase 03 \"Three\" { milestone v1.0 }\n",
        ),
        ("b.spec", "decision D-01 \"D\" { phase 03 }\n"),
    ]);
    assert_eq!(ids(&build), ["v1.0", "v1.0/03", "v1.0/03/D-01", "v2.0"]);
    build.set_verify(true);
    let applied = build.apply([FileChange::Parsed(parse(
        "milestone v1.0 \"One\" { }\nmilestone v2.0 \"Two\" { }\nphase 03 \"Three\" { milestone v2.0 }\n",
        "a.spec",
    ))]);
    assert_eq!(applied.verification, Some(Ok(())));
    assert_eq!(ids(&build), ["v1.0", "v2.0", "v2.0/03", "v2.0/03/D-01"]);
}

#[specforge_test(
    behavior = "find_all_references",
    verify = "the finder reports a final token for a path's last name and a qualifier token for each name before it"
)]
fn the_finder_reads_each_segment() {
    let source = "milestone v1.0 \"One\" { }\nphase 03 \"Three\" { milestone v1.0 }\n\
                  decision D-01 \"D\" { phase v1.0/03 }\nplan 05-01 \"P\" {\n  phase v1.0/03\n  decisions [v1.0/03/D-01]\n}\n";
    let build = scoped(&[("a.spec", source)]);
    let file = parse(source, "a.spec");
    let tokens = written_tokens(&file, build.inputs(), build.graph().scopes());
    let line = 6;
    let in_list: Vec<_> = tokens
        .iter()
        .filter(|t| t.span.start_line == line)
        .collect();
    assert_eq!(in_list.len(), 3, "{in_list:?}");
    let text = source.lines().nth(line - 1).unwrap();
    let written = |t: &&specforge_graph::WrittenToken| {
        text[t.span.start_col - 1..t.span.end_col - 1].to_string()
    };
    let read: Vec<(String, String, bool)> = in_list
        .iter()
        .map(|t| (written(t), t.target.to_string(), t.form != Form::Item))
        .collect();
    assert_eq!(
        read,
        [
            ("v1.0".to_string(), "v1.0".to_string(), true),
            ("03".to_string(), "v1.0/03".to_string(), true),
            ("D-01".to_string(), "v1.0/03/D-01".to_string(), false),
        ]
    );
}

/// An extension declaring `kinds`, each as the JSON of its descriptor.
fn declaring(kinds: &[serde_json::Value]) -> ExtensionDeclaration {
    ExtensionDeclaration {
        handshake: HandshakeResponse {
            name: "@test/scopes".into(),
            version: "1.0.0".into(),
            ..HandshakeResponse::default()
        },
        entities: kinds
            .iter()
            .map(|k| serde_json::from_value::<EntityKindDescriptor>(k.clone()).unwrap())
            .collect(),
        ..ExtensionDeclaration::default()
    }
}

fn field(name: &str, field_type: &str, target: Option<&str>) -> serde_json::Value {
    serde_json::to_value(FieldDescriptor {
        name: name.into(),
        field_type: field_type.into(),
        target_kind: target.map(Into::into),
        ..FieldDescriptor::default()
    })
    .unwrap()
}

/// The W021 messages about a `scoped_by` registering `kinds` reports.
fn w021(kinds: &[serde_json::Value]) -> Vec<String> {
    let project = Assembled::over(specforge_registry::build_registries(vec![declaring(kinds)]));
    project
        .compiled()
        .environment()
        .registries()
        .diagnostics()
        .filter(|d| d.code == "W021" && d.message.contains("scoped_by"))
        .map(|d| d.message.clone())
        .collect()
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a kind declares the reference field its entities are scoped by, and a field that cannot scope is W021"
)]
fn a_kind_declares_its_scope_field() {
    use serde_json::json;
    let kind = |name: &str, scoped_by: &str, fields: Vec<serde_json::Value>| json!({"name": name, "keyword": name, "scoped_by": scoped_by, "fields": fields});
    let root = json!({"name": "root", "keyword": "root"});
    // A well-formed declaration registers without complaint.
    let good = kind("leaf", "up", vec![field("up", "reference", Some("root"))]);
    assert!(w021(&[root.clone(), good]).is_empty());
    // Each of these is W021, and the kind is then unscoped.
    let bad = [
        ("a field the kind has not", kind("leaf", "nope", vec![])),
        (
            "a list",
            kind(
                "leaf",
                "up",
                vec![field("up", "reference_list", Some("root"))],
            ),
        ),
        (
            "no target kind",
            kind("leaf", "up", vec![field("up", "reference", None)]),
        ),
        (
            "a structural target",
            kind("leaf", "up", vec![field("up", "reference", Some("spec"))]),
        ),
        (
            "itself",
            kind("leaf", "up", vec![field("up", "reference", Some("leaf"))]),
        ),
    ];
    for (what, leaf) in bad {
        let found = w021(&[root.clone(), leaf]);
        assert_eq!(found.len(), 1, "{what}: {found:?}");
        assert!(found[0].contains("scoped_by"), "{what}: {found:?}");
    }
    let cycle = [
        kind("a", "up", vec![field("up", "reference", Some("b"))]),
        kind("b", "up", vec![field("up", "reference", Some("a"))]),
    ];
    assert!(!w021(&cycle).is_empty(), "a scoping cycle is W021");
}

#[specforge_test(
    behavior = "check_entities_in_one_order",
    verify = "E013 and E014 judge an entity's name, not its path"
)]
#[ignore = "K2 T6"]
fn a_name_is_judged_not_its_path() {
    let long = "n".repeat(58);
    let source = format!(
        "milestone v1.0 \"One\" {{ }}\nphase 0123456789abc \"Three\" {{ milestone v1.0 }}\n\
         decision {long} \"Long\" {{ phase v1.0/0123456789abc }}\n\
         decision phase \"Reserved\" {{ phase v1.0/0123456789abc }}\n"
    );
    let project = Assembled::scoped().sources(&[("a.spec", &source)]);
    let env = project.compiled().environment();
    let build = env.graph_of([parse(&source, "a.spec")]);
    let entities = env.entity_snapshot(&build);
    let found = env.run_checks(&entities, &PlainDisk);
    let of = |code: &str| found.iter().filter(|d| d.code == code).count();
    assert_eq!(of("E014"), 0, "{found:?}");
    assert_eq!(of("E013"), 1, "{found:?}");
}

#[specforge_test(
    behavior = "compute_the_coverage_view",
    verify = "a test report names an entity by its path or a unique short form"
)]
#[ignore = "K2 T6"]
fn a_test_report_names_an_entity_by_its_short_form() {
    let source = "milestone v1.0 \"One\" { }\nphase 03 \"Three\" { milestone v1.0 }\n\
                  plan 03-02 \"Plan\" {\n  phase v1.0/03\n  verify unit \"works\"\n}\n";
    let project = Assembled::scoped().sources(&[("a.spec", source)]);
    project.record_passing(&[("03-02", "works")]);
    let recorded = project.compiled().recorded(&PlainDisk).unwrap();
    let verdict = recorded
        .coverage
        .verdict("v1.0/03/03-02")
        .expect("a verdict");
    assert!(verdict.is_proven(), "{verdict:?}");
}
