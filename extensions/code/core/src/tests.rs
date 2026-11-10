use super::*;
use specforge_extension_sdk::{
    DanglingReference, EntityKindDescriptor, ExtensionDeclaration, FactStore, UnheldReference,
};
use specforge_test::prelude::*;

fn declaration() -> ExtensionDeclaration {
    specforge_extension_build().declaration()
}

fn kind<'d>(declaration: &'d ExtensionDeclaration, name: &str) -> &'d EntityKindDescriptor {
    declaration
        .entities
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("no kind {name}"))
}

#[test]
fn the_kinds_are_collected_with_their_schemes() {
    let declared = declaration();
    let test = kind(&declared, "test");
    assert_eq!(
        (
            test.writer.as_deref(),
            &test.schemes[..],
            test.result.as_deref()
        ),
        (
            Some("collector"),
            &strings(&["js", "rust"])[..],
            Some("status")
        )
    );
    let run = kind(&declared, "test_run");
    assert_eq!(
        (
            run.writer.as_deref(),
            &run.schemes[..],
            run.result.as_deref()
        ),
        (Some("collector"), &strings(&["run"])[..], None)
    );
    let item = kind(&declared, "item");
    let package = kind(&declared, "package");
    assert_eq!(
        (&item.schemes[..], &package.schemes[..]),
        (&strings(&["rust", "js"])[..], &strings(&["pkg"])[..])
    );
    for k in [test, run, item, package] {
        assert_eq!(k.writer.as_deref(), Some("collector"), "{}", k.name);
        assert_eq!(
            k.scoped_by, None,
            "{}: a scheme ref names the fact whole",
            k.name
        );
    }
    let edge = run.fields.iter().find(|f| f.name == "test").unwrap();
    assert_eq!(
        (edge.target_kind.as_deref(), edge.edge.as_deref()),
        (Some("test"), Some("run_of"))
    );
}

#[specforge_test(
    behavior = "cc_declare_code_facts",
    verify = "item and package reference their facts by in_package, runs_as and uses_package"
)]
fn item_and_package_reference_their_facts() {
    let declared = declaration();
    let edges = |name: &str| -> Vec<(String, Option<String>, Option<String>)> {
        kind(&declared, name)
            .fields
            .iter()
            .filter(|f| f.edge.is_some())
            .map(|f| (f.name.clone(), f.target_kind.clone(), f.edge.clone()))
            .collect()
    };
    let named = |field: &str, target: Option<&str>, edge: &str| {
        (
            field.to_string(),
            target.map(str::to_string),
            Some(edge.to_string()),
        )
    };
    assert_eq!(
        edges("item"),
        [
            named("package", Some("package"), "in_package"),
            named("names", None, "names"),
            named("test_id", Some("test"), "runs_as"),
        ]
    );
    assert_eq!(
        edges("package"),
        [named("depends_on", Some("package"), "uses_package")]
    );
    let labels: Vec<&str> = declared.edges.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(
        labels,
        ["run_of", "names", "in_package", "uses_package", "runs_as"]
    );
}

#[test]
fn test_names_is_a_reference_list_of_any_kind() {
    let declared = declaration();
    for holder in ["test", "item"] {
        let names = kind(&declared, holder)
            .fields
            .iter()
            .find(|f| f.name == "names")
            .unwrap();
        assert_eq!(
            (
                names.field_type.as_str(),
                names.target_kind.as_deref(),
                names.edge.as_deref()
            ),
            ("reference_list", None, Some("names")),
            "{holder}"
        );
    }
}

#[test]
fn the_item_kind_vocabulary_is_the_librarys() {
    let declared = declaration();
    let item = kind(&declared, "item");
    let values = |field: &str| -> Vec<String> {
        item.fields
            .iter()
            .find(|f| f.name == field)
            .unwrap()
            .enum_values
            .clone()
    };
    let names: Vec<String> = ItemKind::ALL
        .iter()
        .map(|k| k.as_str().to_string())
        .collect();
    assert_eq!(values("kind"), names);
    assert_eq!(
        values("visibility"),
        ["public", "crate", "restricted", "private"]
    );
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_status_is_a_test_status_name() {
    let names: Vec<&str> = TestStatus::ALL.iter().map(|s| s.as_str()).collect();
    assert_eq!(STATUS.values(), &names[..]);
}

#[test]
fn the_w102_explanation_names_every_pack_of_the_table() {
    let declared = declaration();
    let w102 = declared.codes.iter().find(|c| c.code == "W102").unwrap();
    for (kind, family, pack) in refs::PACKS {
        let told = match kind {
            "package" => format!("`pkg:{family}` packages"),
            _ => format!("`{family}` {kind}s"),
        };
        let named = format!("`{pack}` for {told}");
        assert!(w102.explanation.contains(&named), "{named}");
    }
}

#[test]
fn it_catalogues_the_codes_it_reports() {
    let codes: Vec<String> = declaration().codes.into_iter().map(|c| c.code).collect();
    assert_eq!(codes, ["E101", "W102", "W401", "I402"]);
}

/// A pass input holding `entities`, with a store of `served` schemes when there is one: `js` holding tests, `run` runs.
/// `entities` beside one store serving test facts in each of `served` (test runs in `run`).
fn input(entities: Vec<PassEntity>, served: &[&str]) -> PassInput {
    let held: Vec<(&str, &str)> = served
        .iter()
        .map(|scheme| (*scheme, if *scheme == "run" { "test_run" } else { "test" }))
        .collect();
    holding(entities, &held)
}

/// `entities` beside one store holding facts of each `(scheme, kind)` of `held`; none when it is empty.
fn holding(entities: Vec<PassEntity>, held: &[(&str, &str)]) -> PassInput {
    let mut store = FactStore {
        collector: "node".to_string(),
        key: "app".to_string(),
        facts: 2,
        ..FactStore::default()
    };
    for (scheme, kind) in held {
        store
            .held
            .entry(scheme.to_string())
            .or_default()
            .push(kind.to_string());
    }
    PassInput {
        entities,
        fact_stores: (!held.is_empty()).then_some(store).into_iter().collect(),
        ..PassInput::default()
    }
}

/// The collected kind a `proof` field admits for `id`: a run's, a test's (`#`), a package's or an item's.
fn admitted(id: &str) -> &'static str {
    match id {
        _ if id.starts_with("run:") => "test_run",
        _ if id.starts_with("pkg:") => "package",
        _ if id.contains('#') => "test",
        _ => "item",
    }
}

fn entity(id: &str, kind: &str) -> PassEntity {
    PassEntity {
        id: id.to_string(),
        kind: kind.to_string(),
        ..PassEntity::default()
    }
}

/// A `truth` citing `ids` in `proof`, none held, each at its own line of `truths.spec`, its field admitting the
/// collected kind of each ([`admitted`]).
fn citing(id: &str, ids: &[&str]) -> PassEntity {
    PassEntity {
        unheld: ids
            .iter()
            .enumerate()
            .map(|(line, cited)| UnheldReference {
                field: "proof".to_string(),
                id: cited.to_string(),
                span: Some(PassSpan {
                    file: "truths.spec".to_string(),
                    start_line: line + 2,
                    start_col: 11,
                    end_line: line + 2,
                    end_col: 11 + cited.len(),
                }),
                kinds: vec![admitted(cited).to_string()],
                history: false,
            })
            .collect(),
        ..entity(id, "truth")
    }
}

/// Each finding as `(code, entity, start line, suggestion)`.
fn found(diagnostics: &[PassDiagnostic]) -> Vec<(&str, &str, usize, Option<&str>)> {
    diagnostics
        .iter()
        .map(|d| {
            (
                d.code.as_str(),
                d.entity.as_deref().unwrap_or_default(),
                d.span.as_ref().map_or(0, |s| s.start_line),
                d.suggestion.as_deref(),
            )
        })
        .collect()
}

#[test]
fn a_missing_test_of_a_served_scheme_is_e101_with_a_close_match() {
    let entities = vec![
        entity("js:app/test/a.test.js#board > adds", "test"),
        entity("js:app/test/a.test.js#board > lists", "test"),
        entity("js:app/test/b.test.js#board > add", "test"),
        entity("run:js:app/test/a.test.js#board > adds", "test_run"),
        citing(
            "t1",
            &[
                "js:app/test/a.test.js#board > add",
                "run:js:app/test/a.test.js#board > ads",
                "js:app/test/c.test.js#board > adds",
            ],
        ),
    ];
    let diagnostics = refs::refs(&input(entities, &["js", "run"]));
    assert_eq!(
        found(&diagnostics),
        [
            (
                "E101",
                "t1",
                2,
                Some("did you mean `js:app/test/a.test.js#board > adds`?")
            ),
            (
                "E101",
                "t1",
                3,
                Some("did you mean `run:js:app/test/a.test.js#board > adds`?")
            ),
            ("E101", "t1", 4, None),
        ]
    );
    assert_eq!(diagnostics[0].severity, PassSeverity::Error);
}

#[test]
fn a_scheme_no_store_serves_is_w102() {
    let entities = vec![
        entity("run:js:app/test/a.test.js#adds", "test_run"),
        citing("t1", &["js:app/test/a.test.js#adds", "rust:a::b"]),
    ];
    let diagnostics = refs::refs(&input(entities, &["run"]));
    assert_eq!(
        found(&diagnostics),
        [("W102", "t1", 2, None), ("W102", "t1", 3, None)]
    );
    assert!(
        diagnostics[0]
            .message
            .contains("no `js` `test` facts were collected: run `specforge collect` with `@javascript/node-test`"),
        "{}",
        diagnostics[0].message
    );
}

#[specforge_test(
    behavior = "cc_check_fact_refs",
    verify = "a reference whose kinds no served store holds in its scheme is code/W102 naming the kind and its pack"
)]
fn a_store_of_another_kind_in_the_scheme_is_w102() {
    // The store holds `js` items: no test runner ran, so a missing test cannot be told from an uncollected one.
    let entities = vec![citing("t1", &["js:app/test/a.test.js#adds"])];
    let diagnostics = refs::refs(&holding(entities, &[("js", "item")]));
    assert_eq!(found(&diagnostics), [("W102", "t1", 2, None)]);
    assert_eq!(
        diagnostics[0].message,
        "`proof` names `js:app/test/a.test.js#adds`, but no `js` `test` facts were collected: run `specforge \
         collect` with `@javascript/node-test`"
    );
}

#[test]
fn a_rust_test_ref_beside_rust_items_only_is_w102() {
    let entities = vec![
        entity("rust:toy::tests::adds", "item"),
        citing("t1", &["rust:toy#tests::adds"]),
    ];
    let diagnostics = refs::refs(&holding(entities, &[("rust", "item")]));
    assert_eq!(found(&diagnostics), [("W102", "t1", 2, None)]);
    assert!(
        diagnostics[0].message.ends_with(
            "no `rust` `test` facts were collected: run `specforge collect` with `@rust/test`"
        ),
        "{}",
        diagnostics[0].message
    );
}

#[specforge_test(
    behavior = "cc_check_fact_refs",
    verify = "a package reference's W102 names the pack of its purl type"
)]
fn a_package_references_w102_names_the_pack_of_its_purl_type() {
    let entities = vec![citing("t1", &["pkg:npm/%40acme/core", "pkg:cargo/toy"])];
    let diagnostics = refs::refs(&holding(entities, &[]));
    let remedies: Vec<&str> = diagnostics
        .iter()
        .map(|d| d.message.rsplit(": ").next().unwrap_or_default())
        .collect();
    assert_eq!(
        remedies,
        [
            "run `specforge collect` with `@javascript/source`",
            "run `specforge collect` with `@rust/cargo`"
        ]
    );
}

#[specforge_test(
    behavior = "cc_check_fact_refs",
    verify = "a test id in a field of no target kind beside only item facts is code/W102"
)]
fn a_test_id_in_a_field_of_no_target_beside_only_items_is_w102() {
    // A field with no target admits every collected kind of the scheme; the `#` makes the id a test's alone.
    let note = PassEntity {
        unheld: vec![UnheldReference {
            field: "see".to_string(),
            id: "rust:toy#tests::adds".to_string(),
            span: read_at(2),
            kinds: strings(&["item", "test"]),
            history: false,
        }],
        ..entity("t1", "truth")
    };
    let diagnostics = refs::refs(&holding(vec![note], &[("rust", "item")]));
    assert_eq!(found(&diagnostics), [("W102", "t1", 2, None)]);
    assert!(
        diagnostics[0].message.ends_with(
            "no `rust` `test` facts were collected: run `specforge collect` with `@rust/test`"
        ),
        "{}",
        diagnostics[0].message
    );
}

#[specforge_test(
    behavior = "cc_check_fact_refs",
    verify = "a reference naming no fact of a kind a served store holds in its scheme is code/E101"
)]
fn a_missing_item_of_a_held_kind_is_e101_with_no_suggestion() {
    let entities = vec![
        entity("rust:toy::Board::add", "item"),
        citing("t1", &["rust:toy::Board::ad"]),
    ];
    let diagnostics = refs::refs(&holding(entities, &[("rust", "item")]));
    // `refs` is sent test facts only, so an item reference is told no near item.
    assert_eq!(found(&diagnostics), [("E101", "t1", 2, None)]);
    assert_eq!(
        diagnostics[0].message,
        "`proof` names `rust:toy::Board::ad`, which no collected `rust` `item` fact holds"
    );
}

#[test]
fn a_facts_own_unheld_reference_is_not_judged() {
    // Another collector writes what the item names: a runner that did not run is no disagreement.
    let item = PassEntity {
        unheld: vec![
            UnheldReference {
                field: "test_id".to_string(),
                id: "rust:toy#tests::adds".to_string(),
                span: read_at(3),
                kinds: vec!["test".to_string()],
                history: false,
            },
            UnheldReference {
                field: "package".to_string(),
                id: "pkg:cargo/toy".to_string(),
                span: read_at(4),
                kinds: vec!["package".to_string()],
                history: false,
            },
        ],
        ..entity("rust:toy::tests::adds", "item")
    };
    let served = [("rust", "item"), ("rust", "test"), ("pkg", "package")];
    assert!(refs::refs(&holding(vec![item], &served)).is_empty());
}

#[test]
fn a_reference_admitting_none_of_its_kinds_is_not_judged() {
    let api = PassEntity {
        unheld: vec![UnheldReference {
            field: "surface".to_string(),
            id: "rust:toy::api".to_string(),
            span: read_at(2),
            kinds: vec!["api".to_string()],
            history: false,
        }],
        ..entity("t1", "truth")
    };
    assert!(refs::refs(&holding(vec![api], &[("rust", "item")])).is_empty());
}

/// A reference the harness wrote once (a commit's items) records what held then: no finding when no fact holds it.
#[test]
fn a_history_reference_is_no_finding() {
    let mut commit = citing("c1", &["rust:toy::gone", "rust:toy::Board::ad"]);
    commit.unheld[0].history = true;
    let held = vec![entity("rust:toy::Board::add", "item"), commit];
    let diagnostics = refs::refs(&holding(held, &[("rust", "item")]));
    assert_eq!(found(&diagnostics), [("E101", "c1", 3, None)]);
    let mut uncollected = citing("c2", &["rust:toy::gone"]);
    uncollected.unheld[0].history = true;
    assert!(refs::refs(&input(vec![uncollected], &[])).is_empty());
}

#[test]
fn a_pending_owner_is_unchecked() {
    let pending = PassEntity {
        pending: true,
        ..citing("p/t1", &["js:app/test/a.test.js#later"])
    };
    let complete = citing("q/t1", &["js:app/test/a.test.js#later"]);
    let diagnostics = refs::refs(&input(vec![pending, complete], &["js"]));
    assert_eq!(found(&diagnostics), [("E101", "q/t1", 2, None)]);
}

#[test]
fn an_uncited_fact_is_never_a_finding() {
    let entities = vec![
        entity("js:app/test/a.test.js#adds", "test"),
        entity("run:js:app/test/a.test.js#adds", "test_run"),
        entity("t1", "truth"),
    ];
    assert!(refs::refs(&input(entities.clone(), &["js", "run"])).is_empty());
    assert!(refs::refs(&input(entities, &[])).is_empty());
}

/// Where the runner pack's collector read a fact's reference: line `line` of `a.test.js`.
fn read_at(line: usize) -> Option<PassSpan> {
    Some(PassSpan {
        file: "a.test.js".to_string(),
        start_line: line,
        start_col: 5,
        end_line: line,
        end_col: 9,
    })
}

const COLLECTOR: &str =
    "run `specforge collect` again; if it stays, the runner pack mints it wrong";

#[specforge_test(
    behavior = "cc_check_fact_refs",
    verify = "a fact's reference no fact or entity answers is code/E101 addressed to its collector, at its span"
)]
fn a_dangling_reference_of_a_test_run_is_e101_for_its_collector() {
    let dangling = |id: &str, reason, actual: Option<&str>, line| DanglingReference {
        field: "test".to_string(),
        id: id.to_string(),
        span: read_at(line),
        reason,
        actual: actual.map(str::to_string),
        did_you_mean: None,
        candidates: Vec::new(),
    };
    let run = PassEntity {
        dangling: vec![
            dangling("zz", DanglingReason::Unresolved, None, 3),
            dangling("t1", DanglingReason::Mistyped, Some("truth"), 7),
        ],
        ..entity("run:js:app/a.test.js#adds", "test_run")
    };
    let diagnostics = refs::refs(&input(vec![run, entity("t1", "truth")], &["js", "run"]));
    assert_eq!(
        found(&diagnostics),
        [
            ("E101", "run:js:app/a.test.js#adds", 3, Some(COLLECTOR)),
            ("E101", "run:js:app/a.test.js#adds", 7, Some(COLLECTOR)),
        ]
    );
    assert_eq!(
        diagnostics[0].message,
        "the fact 'run:js:app/a.test.js#adds' names 'zz', which no collected fact holds (its collector's facts \
         disagree)"
    );
    assert_eq!(
        diagnostics[1].message,
        "the fact 'run:js:app/a.test.js#adds' names 't1' in 'test', a truth, not a test"
    );
    assert_eq!(diagnostics[0].span.as_ref().unwrap().file, "a.test.js");
}

#[test]
fn a_mistyped_reference_in_a_field_code_core_does_not_declare_names_no_kind() {
    // Another extension's field on a test: what it takes is not `@code/core`'s to tell.
    let test = PassEntity {
        dangling: vec![DanglingReference {
            field: "covers".to_string(),
            id: "t1".to_string(),
            span: read_at(4),
            reason: DanglingReason::Mistyped,
            actual: Some("truth".to_string()),
            did_you_mean: None,
            candidates: Vec::new(),
        }],
        ..entity("js:app/a.test.js#adds", "test")
    };

    let diagnostics = refs::refs(&input(vec![test, entity("t1", "truth")], &["js"]));

    assert_eq!(
        diagnostics[0].message,
        "the fact 'js:app/a.test.js#adds' names 't1' in 'covers', a truth, which 'covers' does not take"
    );
}

#[test]
fn every_reference_field_with_a_target_is_one_refs_names() {
    let declared = declaration();
    let mut targeted: Vec<(&str, &str, &str)> = declared
        .entities
        .iter()
        .flat_map(|k| {
            k.fields.iter().filter_map(move |f| {
                Some((k.name.as_str(), f.name.as_str(), f.target_kind.as_deref()?))
            })
        })
        .collect();
    targeted.sort();
    let mut table: Vec<(&str, &str, &str)> = REFERENCES
        .iter()
        .map(|r| (r.kind, r.field, r.target))
        .collect();
    table.sort();
    assert_eq!(targeted, table);
    for row in &REFERENCES {
        let edge = declared.edges.iter().find(|e| e.label == row.edge).unwrap();
        assert_eq!(
            (edge.source_kind.as_deref(), edge.target_kind.as_deref()),
            (Some(row.kind), Some(row.target)),
            "{}",
            row.edge
        );
    }
}

#[test]
fn an_unheld_reference_of_a_test_run_is_e101_for_its_collector() {
    let run = PassEntity {
        unheld: vec![UnheldReference {
            field: "test".to_string(),
            id: "js:app/a.test.js#gone".to_string(),
            span: read_at(2),
            ..UnheldReference::default()
        }],
        ..entity("run:js:app/a.test.js#gone", "test_run")
    };
    // Whether or not a store serves the scheme: the collector wrote both, so it is never W102.
    for served in [&["js", "run"][..], &[]] {
        let diagnostics = refs::refs(&input(vec![run.clone()], served));
        assert_eq!(
            found(&diagnostics),
            [("E101", "run:js:app/a.test.js#gone", 2, Some(COLLECTOR))]
        );
        assert!(diagnostics[0]
            .message
            .ends_with("(its collector's facts disagree)"));
    }
}

#[test]
fn the_edit_distance_counts_insertions_deletions_and_substitutions() {
    for (a, b, distance) in [
        ("", "", 0),
        ("adds", "adds", 0),
        ("add", "adds", 1),
        ("adds", "ads", 1),
        ("adds", "odds", 1),
        ("kitten", "sitting", 3),
        ("", "abc", 3),
        ("é", "e", 1),
    ] {
        assert_eq!(refs::levenshtein(a, b), distance, "{a} -> {b}");
        assert_eq!(refs::levenshtein(b, a), distance, "{b} -> {a}");
    }
}
/// A `test` fact whose last run gave `status`, naming `names`.
fn test_fact(id: &str, status: &str, names: &[&str]) -> PassEntity {
    let mut test = entity(id, "test");
    test.fields.insert("status".to_string(), status.to_string());
    if !names.is_empty() {
        test.values.insert("names".to_string(), names.into());
    }
    test
}

fn edge(source: &str, target: &str, label: &str) -> PassEdge {
    PassEdge {
        source: source.to_string(),
        target: target.to_string(),
        label: label.to_string(),
    }
}

/// Each finding as `(code, entity)`.
fn on(diagnostics: &[PassDiagnostic]) -> Vec<(&str, &str)> {
    diagnostics
        .iter()
        .map(|d| (d.code.as_str(), d.entity.as_deref().unwrap_or_default()))
        .collect()
}

#[test]
fn a_complete_owner_linking_a_failed_test_is_w401() {
    let input = PassInput {
        entities: vec![
            test_fact("js:app/a.test.js#adds", "failed", &[]),
            test_fact("js:app/a.test.js#lists", "passed", &[]),
            entity("run:js:app/a.test.js#adds", "test_run"),
            entity("t1", "truth"),
            entity("t2", "truth"),
        ],
        edges: vec![
            edge("t1", "js:app/a.test.js#adds", "proof"),
            edge("t1", "js:app/a.test.js#adds", "proof"),
            edge("t2", "js:app/a.test.js#lists", "proof"),
            edge(
                "run:js:app/a.test.js#adds",
                "js:app/a.test.js#adds",
                "run_of",
            ),
        ],
        ..PassInput::default()
    };
    let diagnostics = proof::proof(&input);
    assert_eq!(on(&diagnostics), [("W401", "t1")]);
    assert_eq!(diagnostics[0].severity, PassSeverity::Warning);
}

#[test]
fn a_pending_owner_linking_a_failed_test_is_silent() {
    let input = PassInput {
        entities: vec![
            test_fact("js:app/a.test.js#adds", "failed", &[]),
            PassEntity {
                pending: true,
                ..entity("p/t1", "truth")
            },
        ],
        edges: vec![edge("p/t1", "js:app/a.test.js#adds", "proof")],
        ..PassInput::default()
    };
    assert!(proof::proof(&input).is_empty());
}

/// `holder` with the dangling name `name` read at line `line` of its file.
fn naming(holder: PassEntity, name: &str, reason: DanglingReason, line: usize) -> PassEntity {
    PassEntity {
        dangling: vec![DanglingReference {
            field: "names".to_string(),
            id: name.to_string(),
            span: read_at(line),
            reason,
            actual: None,
            did_you_mean: None,
            candidates: Vec::new(),
        }],
        ..holder
    }
}

#[specforge_test(
    behavior = "cc_declare_code_facts",
    verify = "a test or item naming no entity is code/I402"
)]
fn a_test_naming_no_entity_is_i402() {
    let input = PassInput {
        entities: vec![
            naming(
                test_fact("js:app/a.test.js#adds", "passed", &["t1", "t9"]),
                "t9",
                DanglingReason::Unresolved,
                3,
            ),
            entity("t1", "truth"),
        ],
        ..PassInput::default()
    };
    let diagnostics = proof::proof(&input);
    assert_eq!(on(&diagnostics), [("I402", "js:app/a.test.js#adds")]);
    assert!(
        diagnostics[0].message.contains("`t9`, which is no entity"),
        "{}",
        diagnostics[0].message
    );
    assert_eq!(diagnostics[0].severity, PassSeverity::Info);
    assert_eq!(diagnostics[0].span.as_ref().map(|s| s.start_line), Some(3));
    assert!(
        refs::refs(&input).is_empty(),
        "a dangling name is the proof pass's"
    );
}

#[test]
fn an_item_naming_several_entities_is_i402_listing_them() {
    let mut item = naming(
        entity("rust:app::board::add", "item"),
        "t1",
        DanglingReason::Ambiguous,
        7,
    );
    item.dangling[0].candidates = strings(&["a/t1", "b/t1"]);
    let input = PassInput {
        entities: vec![item],
        ..PassInput::default()
    };
    let diagnostics = proof::proof(&input);
    assert_eq!(on(&diagnostics), [("I402", "rust:app::board::add")]);
    let message = &diagnostics[0].message;
    assert!(
        message.contains("the item `rust:app::board::add` names `t1`, the short name of several"),
        "{message}"
    );
    assert!(message.ends_with("entities (a/t1, b/t1)"), "{message}");
}
