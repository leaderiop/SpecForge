//! The `proof` pass: a complete owner linking a failed test (W401) and a test or item naming no entity (I402).

use super::*;

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
