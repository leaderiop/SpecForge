//! An old governance decision becomes a `@gsd/core` decision (ADR 0359's conversion table), and the run's frame:
//! pre-checks, fills, the ledger, the target.

use serde_json::json;
use specforge_ops::authoring::edit::{Edit, Input};
use std::collections::BTreeMap;

use super::super::ledger::Why;
use super::super::plan;
use super::super::table;
use super::super::write;
use super::{create, field, fixture, ledgered, many, old, one, planned, stops};

#[test]
fn a_decision_becomes_one_create_keeping_its_id_title_and_file() {
    let old = fixture();
    let entities = old.entities();
    plan::prechecks(&entities).expect("the fixture converts");
    let planned = plan::plan(&entities, &old.index(), &Default::default());
    let headers: Vec<(&str, &str, &str, &str)> = planned
        .creates
        .iter()
        .map(|edit| match edit {
            Edit::Create {
                kind,
                id,
                title,
                file,
                ..
            } => (kind.as_str(), id.as_str(), title.as_str(), file.as_str()),
            other => panic!("only creates: {other:?}"),
        })
        .filter(|header| header.1 != "store_entries_solution")
        .collect();
    let expected: Vec<(&str, &str, &str, &str)> = entities
        .iter()
        .filter_map(|e| {
            Some((
                table::new_kind(&e.kind)?,
                e.id.as_str(),
                e.title.as_deref().unwrap(),
                e.file.as_str(),
            ))
        })
        .collect();
    assert_eq!(
        headers, expected,
        "every converted entity, once, in the old order"
    );
    let decisions: Vec<_> = headers.iter().filter(|h| h.0 == "decision").collect();
    assert_eq!(decisions.len(), 6, "every fixture decision, once");
    assert_eq!(decisions[0].3, "spec/governance/decisions.spec");
    assert_eq!(
        (planned.converted, planned.created),
        (expected.len(), 1),
        "the module and the release create nothing; one feature's solution is created"
    );
}

#[test]
fn decision_status_writes_outcome_or_settled_and_never_status() {
    let planned = planned(&[
        old(
            "decision",
            "a",
            json!({"status": "accepted", "decision": "A"}),
        ),
        old(
            "decision",
            "p",
            json!({"status": "proposed", "decision": "P"}),
        ),
        old(
            "decision",
            "d",
            json!({"status": "deprecated", "decision": "D"}),
        ),
        old(
            "decision",
            "s",
            json!({"status": "superseded", "decision": "S"}),
        ),
    ]);
    assert_eq!(field(&planned, "a", "settled"), Some(&Input::Bool(true)));
    assert_eq!(field(&planned, "a", "outcome"), None);
    assert_eq!(field(&planned, "p", "outcome"), Some(&one("pending")));
    assert_eq!(field(&planned, "d", "outcome"), Some(&one("revisit")));
    for id in ["a", "p", "d", "s"] {
        assert_eq!(field(&planned, id, "status"), None, "{id}");
    }
    assert_eq!(
        create(&planned, "s").1.keys().collect::<Vec<_>>(),
        ["origin", "text"]
    );
    let in_force: Vec<(&str, Option<bool>)> = planned
        .checks
        .iter()
        .map(|c| {
            assert_eq!((c.new_kind, c.new_field), ("decision", "in_force"));
            (c.id.as_str(), c.expected.as_ref().and_then(|v| v.as_bool()))
        })
        .collect();
    assert_eq!(
        in_force,
        [
            ("a", Some(true)),
            ("p", Some(true)),
            ("d", Some(true)),
            ("s", Some(false))
        ]
    );
}

#[test]
fn superseded_by_is_written_as_supersedes_on_the_successor() {
    let planned = planned(&[
        old(
            "decision",
            "first",
            json!({"status": "superseded", "decision": "1", "superseded_by": "third"}),
        ),
        old(
            "decision",
            "second",
            json!({"status": "superseded", "decision": "2", "superseded_by": "third"}),
        ),
        old(
            "decision",
            "third",
            json!({"status": "accepted", "decision": "3"}),
        ),
    ]);
    assert_eq!(field(&planned, "first", "supersedes"), None);
    assert_eq!(field(&planned, "first", "superseded_by"), None);
    assert_eq!(
        field(&planned, "third", "supersedes"),
        Some(&many(&["first", "second"])),
        "the direction flips, every superseded decision listed, by id"
    );
}

#[test]
fn alternatives_become_options_by_name() {
    let planned = planned(&[old(
        "decision",
        "x",
        json!({"status": "accepted", "decision": "X", "alternatives": ["A hand parser", "Two parsers"]}),
    )]);
    let option = |name: &str| BTreeMap::from([("name".to_string(), one(name))]);
    assert_eq!(
        field(&planned, "x", "alternatives"),
        Some(&Input::Blocks(vec![
            option("A hand parser"),
            option("Two parsers")
        ]))
    );
}

#[test]
fn a_required_value_the_old_spec_lacks_is_filled_and_ledgered() {
    let planned = planned(&[old(
        "decision",
        "x",
        json!({"status": "accepted", "decision": "X"}),
    )]);
    assert_eq!(field(&planned, "x", "origin"), Some(&one("project")));
    assert_eq!(
        ledgered(&planned, "x"),
        [(
            "origin".to_string(),
            Why::Filled {
                field: "decision.origin".to_string(),
                value: json!("project")
            }
        )]
    );
}

#[test]
fn a_value_in_a_refused_field_stops_the_run_naming_it() {
    let stops = stops(&[
        old(
            "decision",
            "x",
            json!({"status": "accepted", "decision": "X", "refs": ["https://example.org"]}),
        ),
        old(
            "decision",
            "y",
            json!({"status": "accepted", "decision": "Y", "description": "Why"}),
        ),
    ]);
    let named: Vec<String> = stops.iter().map(ToString::to_string).collect();
    assert_eq!(
        named,
        [
            "decision 'x' refs: refused: no kind holds it (value [\"https://example.org\"])",
            "decision 'y' description: refused: no kind holds it (value \"Why\")",
        ]
    );
}

#[test]
fn two_creates_with_one_id_stop_the_run_naming_them() {
    let stops = stops(&[
        old(
            "decision",
            "same",
            json!({"status": "accepted", "decision": "A"}),
        ),
        old(
            "decision",
            "same",
            json!({"status": "proposed", "decision": "B"}),
        ),
    ]);
    assert_eq!(stops.len(), 1, "{stops:?}");
    assert_eq!(
        stops[0].to_string(),
        "'same': two creates with one id, by decision 'same' and decision 'same'"
    );
}

#[test]
fn the_ledger_is_sorted_and_identical_across_runs() {
    let entities = [
        old(
            "decision",
            "zeta",
            json!({"status": "superseded", "decision": "Z"}),
        ),
        old(
            "decision",
            "alpha",
            json!({"status": "accepted", "decision": "A", "invariants": ["gone"]}),
        ),
    ];
    let first = planned(&entities);
    let second = planned(&entities);
    assert_eq!(first.ledger.to_json("p"), second.ledger.to_json("p"));
    assert_eq!(first.creates, second.creates);
    let keys: Vec<(String, String)> = first
        .ledger
        .entries()
        .map(|e| (e.id.clone(), e.field.clone()))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
    assert_eq!(
        keys,
        [
            ("alpha".to_string(), "invariants".to_string()),
            ("alpha".to_string(), "origin".to_string()),
            ("zeta".to_string(), "origin".to_string()),
        ]
    );
    let text = first.ledger.to_json("p");
    assert!(text.starts_with("{\n  \"format\": 1,\n  \"from\": \"p\",\n  \"entries\": ["));
    assert!(text.ends_with("}\n"));
}

#[test]
fn a_target_holding_a_project_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    assert!(write::check_to(&dir.path().join("absent")).is_ok());
    assert!(write::check_to(dir.path()).is_ok());
    std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\n").unwrap();
    assert!(
        write::check_to(dir.path()).is_ok(),
        "a tree without a project"
    );
    std::fs::create_dir(dir.path().join("spec")).unwrap();
    let held = write::check_to(dir.path()).unwrap_err();
    assert!(
        held.ends_with("already holds spec: convert into a directory with no project"),
        "{held}"
    );
    std::fs::write(dir.path().join("specforge.json"), "{}").unwrap();
    let held = write::check_to(dir.path()).unwrap_err();
    assert!(
        held.contains("already holds specforge.json, spec:"),
        "{held}"
    );
}

/// A symbolic link where absorb writes is refused, dangling or not, and left where it is: init would write through
/// it, and a roll back would remove it.
#[cfg(unix)]
#[test]
fn a_symbolic_link_where_absorb_writes_is_refused_and_kept() {
    for name in ["spec", ".specforge", ".gitignore"] {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join(name);
        std::os::unix::fs::symlink(dir.path().join("nowhere"), &link).unwrap();
        let refused = write::check_to(dir.path()).unwrap_err();
        assert!(
            refused.ends_with(&format!(
                "holds a symbolic link where absorb writes: {name}; remove it, or convert elsewhere"
            )),
            "{name}: {refused}"
        );
        assert!(std::fs::symlink_metadata(&link).is_ok(), "{name} is kept");
    }
    let dir = tempfile::tempdir().unwrap();
    let to = dir.path().join("to");
    std::os::unix::fs::symlink(dir.path(), &to).unwrap();
    assert!(
        write::check_to(&to)
            .unwrap_err()
            .ends_with("is a symbolic link: name the directory it points to")
    );
}
