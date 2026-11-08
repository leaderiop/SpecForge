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
        (Some("collector"), &["js".to_string()][..], Some("status"))
    );
    let run = kind(&declared, "test_run");
    assert_eq!(
        (
            run.writer.as_deref(),
            &run.schemes[..],
            run.result.as_deref()
        ),
        (Some("collector"), &["run".to_string()][..], None)
    );
    for k in [test, run] {
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

#[test]
fn a_status_is_a_test_status_name() {
    let names: Vec<&str> = TestStatus::ALL.iter().map(|s| s.as_str()).collect();
    assert_eq!(STATUS.values(), &names[..]);
}

#[test]
fn it_catalogues_the_codes_it_reports() {
    let codes: Vec<String> = declaration().codes.into_iter().map(|c| c.code).collect();
    assert_eq!(codes, ["E101", "W102", "W401", "I402"]);
}

/// A pass input holding `entities`, with a store of `served` schemes when there is one.
fn input(entities: Vec<PassEntity>, served: &[&str]) -> PassInput {
    PassInput {
        entities,
        fact_stores: (!served.is_empty())
            .then(|| FactStore {
                collector: "node".to_string(),
                key: "app".to_string(),
                schemes: served.iter().map(|s| s.to_string()).collect(),
                facts: 2,
            })
            .into_iter()
            .collect(),
        ..PassInput::default()
    }
}

fn entity(id: &str, kind: &str) -> PassEntity {
    PassEntity {
        id: id.to_string(),
        kind: kind.to_string(),
        ..PassEntity::default()
    }
}

/// A `truth` citing `ids` in `proof`, none held, each at its own line of `truths.spec`.
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
    assert_eq!(found(&diagnostics), [("W102", "t1", 2, None)]);
    assert!(
        diagnostics[0]
            .message
            .contains("no `js` facts were collected at this commit: run `specforge collect`"),
        "{}",
        diagnostics[0].message
    );
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
    assert_eq!(targeted, refs::TAKES);
}

#[test]
fn an_unheld_reference_of_a_test_run_is_e101_for_its_collector() {
    let run = PassEntity {
        unheld: vec![UnheldReference {
            field: "test".to_string(),
            id: "js:app/a.test.js#gone".to_string(),
            span: read_at(2),
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

#[test]
fn a_test_naming_no_entity_is_i402() {
    let input = PassInput {
        entities: vec![
            test_fact("js:app/a.test.js#adds", "passed", &["t1", "t9"]),
            entity("t1", "truth"),
        ],
        ..PassInput::default()
    };
    let diagnostics = proof::proof(&input);
    assert_eq!(on(&diagnostics), [("I402", "js:app/a.test.js#adds")]);
    assert!(
        diagnostics[0].message.contains("`t9`"),
        "{}",
        diagnostics[0].message
    );
    assert_eq!(diagnostics[0].severity, PassSeverity::Info);
}
