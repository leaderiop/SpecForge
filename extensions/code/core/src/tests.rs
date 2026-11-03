use super::*;
use specforge_extension_sdk::{
    EntityKindDescriptor, ExtensionDeclaration, FactStore, UnheldReference,
};

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
