//! An old governance decision becomes a `@gsd/core` decision (ADR 0359's conversion table), and the run's frame:
//! pre-checks, fills, the ledger, the target.

use serde_json::json;
use specforge_ops::authoring::edit::{Edit, Input};

use super::super::ledger::Why;
use super::super::plan;
use super::super::table::{self, Home};
use super::super::write;
use super::{
    create_of, field_of, fixture, ledgered, link_dir, link_file, old, one, planned, stops,
};

#[test]
fn a_decision_becomes_one_create_keeping_its_title_and_file() {
    let old = fixture();
    let entities = old.entities();
    plan::prechecks(&entities).expect("the fixture converts");
    let planned = plan::plan(&entities);
    let headers: Vec<(&str, String, &str, &str)> = planned
        .creates
        .iter()
        .map(|edit| match edit {
            Edit::Create {
                kind, title, file, ..
            } => (
                kind.as_str(),
                super::path_of(edit),
                title.as_str(),
                file.as_str(),
            ),
            other => panic!("only creates: {other:?}"),
        })
        .collect();
    let expected: Vec<(&str, String, &str, &str)> = entities
        .iter()
        .filter_map(|e| {
            let new = planned.ids.get(&e.kind, &e.id)?;
            Some((
                new.kind,
                new.path(),
                e.title.as_deref().unwrap(),
                e.file.as_str(),
            ))
        })
        .collect();
    assert_eq!(
        headers, expected,
        "every converted entity, once, in the old order, by its new path"
    );
    let decisions: Vec<_> = headers.iter().filter(|h| h.0 == "decision").collect();
    assert_eq!(
        decisions.iter().map(|h| h.1.as_str()).collect::<Vec<_>>(),
        ["D-01", "D-02"],
        "every fixture decision in force, once, numbered in old-id order (one_parser, second_store): the others are \
         dropped whole (ADR 0372 D5, D3)"
    );
    assert_eq!(decisions[0].3, "spec/governance/decisions.spec");
    assert_eq!(
        (planned.converted, planned.creates.len()),
        (expected.len(), expected.len()),
        "no entity is created that the old spec did not hold"
    );
}

/// Only a decision in force converts (ADR 0372 D5): status `accepted` and no `superseded_by`. It writes `text`,
/// `rationale`, `settled true` and `origin project`; every other decision is dropped whole, its title and each value
/// one `dropped` entry whose reason names `docs/adr/` as its record.
#[test]
fn only_a_decision_in_force_converts() {
    let decision = |id: &str, status: &str, more: serde_json::Value| {
        let mut fields = json!({"status": status, "decision": "D", "reason": "R"});
        for (k, v) in more.as_object().unwrap() {
            fields[k] = v.clone();
        }
        old("decision", id, fields)
    };
    let planned = planned(&[
        decision("a", "accepted", json!({})),
        decision("p", "proposed", json!({})),
        decision("d", "deprecated", json!({})),
        decision("s", "superseded", json!({})),
        decision("later", "accepted", json!({"superseded_by": "a"})),
    ]);
    let (kind, fields) = create_of(&planned, "decision", "a");
    assert_eq!(kind, "decision");
    assert_eq!(
        fields,
        &std::collections::BTreeMap::from([
            ("origin".to_string(), one("project")),
            ("rationale".to_string(), one("R")),
            ("settled".to_string(), Input::Bool(true)),
            ("text".to_string(), one("D")),
        ])
    );
    assert_eq!(planned.creates.len(), 1);
    assert_eq!(planned.converted, 1);
    assert!(
        planned.checks.is_empty(),
        "no computed value of a decision is compared: `in_force` left (ADR 0370 D4)"
    );
    for (id, fields) in [
        ("p", ["decision", "reason", "status", "title"].as_slice()),
        ("d", &["decision", "reason", "status", "title"]),
        ("s", &["decision", "reason", "status", "title"]),
        (
            "later",
            &["decision", "reason", "status", "superseded_by", "title"],
        ),
    ] {
        let ledger = ledgered(&planned, id);
        assert_eq!(
            ledger.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
            fields,
            "{id}"
        );
        for (field, why) in ledger {
            assert!(
                matches!(&why, Why::Dropped(reason) if reason.contains("docs/adr/") && reason.contains("ADR 0372 D5")),
                "{id}.{field}: {why:?}"
            );
        }
    }
}

/// A field whose home left `@gsd/core` (ADR 0370 D3 and D4) is one `dropped` entry with its row's reason, and the
/// decision still converts with what it keeps.
#[test]
fn a_field_whose_home_leaves_is_ledgered_dropped_and_the_entity_still_converts() {
    let planned = planned(&[old(
        "decision",
        "x",
        json!({
            "status": "accepted", "decision": "Read with one parser.", "context": "Readers drift.",
            "alternatives": ["A hand parser", "Two parsers"],
        }),
    )]);
    assert_eq!(planned.creates.len(), 1);
    let (kind, fields) = create_of(&planned, "decision", "x");
    assert_eq!(kind, "decision");
    assert_eq!(
        fields.keys().collect::<Vec<_>>(),
        ["origin", "settled", "text"],
        "of the three fields, only `decision` is written, as `text`"
    );
    assert_eq!(
        field_of(&planned, "decision", "x", "text"),
        Some(&one("Read with one parser."))
    );
    let reason = |field: &str| {
        let &[Home::Dropped(reason)] = table::row_of("decision", field).unwrap().homes else {
            panic!("decision.{field} is dropped");
        };
        assert!(reason.contains("ADR 0370"), "{reason}");
        Why::Dropped(reason.to_string())
    };
    let dropped: Vec<(String, Why)> = ledgered(&planned, "x")
        .into_iter()
        .filter(|(_, why)| matches!(why, Why::Dropped(_)))
        .collect();
    assert_eq!(
        dropped,
        ["alternatives", "context"].map(|f| (f.to_string(), reason(f))),
        "one entry per value, each with its row's reason"
    );
    assert!(
        reason("context") != reason("alternatives"),
        "each reason names the field that leaves"
    );
}

#[test]
fn a_required_value_the_old_spec_lacks_is_filled_and_ledgered() {
    let planned = planned(&[old(
        "decision",
        "x",
        json!({"status": "accepted", "decision": "X"}),
    )]);
    assert_eq!(
        field_of(&planned, "decision", "x", "origin"),
        Some(&one("project"))
    );
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

/// No row refuses (ADR 0372 D2): a field the old builtins register that no `@gsd/core` field takes is one `dropped`
/// entry with its reason, and the run goes on.
#[test]
fn a_known_field_with_no_home_is_dropped_and_never_stops_the_run() {
    let entities = [
        old(
            "decision",
            "x",
            json!({"status": "accepted", "decision": "X", "refs": ["https://example.org"], "description": "Why"}),
        ),
        old(
            "constraint",
            "c",
            json!({"description": "D", "scope": "the store"}),
        ),
        old(
            "feature",
            "f",
            json!({"problem": "P", "depends_on": ["g"], "features": ["g"]}),
        ),
    ];
    assert_eq!(plan::prechecks(&entities), Ok(()));
    let planned = planned(&entities);
    assert_eq!(planned.converted, 3);
    for (id, name) in [
        ("x", "refs"),
        ("x", "description"),
        ("c", "scope"),
        ("f", "depends_on"),
        ("f", "features"),
    ] {
        let whys: Vec<Why> = ledgered(&planned, id)
            .into_iter()
            .filter(|(f, _)| f == name)
            .map(|(_, why)| why)
            .collect();
        assert!(
            matches!(whys.as_slice(), [Why::Dropped(reason)] if !reason.trim().is_empty()),
            "{id}.{name}: {whys:?}"
        );
    }
}

#[test]
fn two_entities_given_one_id_stop_the_run_naming_them() {
    let stops = stops(&[
        old(
            "decision",
            "same",
            json!({"status": "accepted", "decision": "A"}),
        ),
        old(
            "decision",
            "same",
            json!({"status": "accepted", "decision": "B"}),
        ),
    ]);
    assert_eq!(stops.len(), 1, "{stops:?}");
    assert_eq!(
        stops[0].to_string(),
        "'D-01': two entities given one decision id, by decision 'same' and decision 'same'"
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
            ("zeta".to_string(), "decision".to_string()),
            ("zeta".to_string(), "status".to_string()),
            ("zeta".to_string(), "title".to_string()),
        ]
    );
    let text = first.ledger.to_json("p");
    assert!(text.starts_with("{\n  \"format\": 2,\n  \"from\": \"p\",\n  \"entries\": ["));
    assert!(
        text.ends_with(
            "  \"ids\": [\n    {\n      \"kind\": \"decision\",\n      \"old\": \"alpha\",\n      \"new\": \"D-01\"\n    }\n  ]\n}\n"
        ),
        "the id map follows the entries, the converted decision alone: {text}"
    );
}

/// The spec root most sources keep.
const SPEC: Option<&str> = Some("spec");

#[test]
fn a_target_holding_a_project_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    assert!(write::check_to(&dir.path().join("absent"), SPEC).is_ok());
    assert!(write::check_to(dir.path(), SPEC).is_ok());
    std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\n").unwrap();
    assert!(
        write::check_to(dir.path(), SPEC).is_ok(),
        "a tree without a project"
    );
    std::fs::create_dir(dir.path().join("spec")).unwrap();
    let held = write::check_to(dir.path(), SPEC).unwrap_err();
    assert!(
        held.ends_with("already holds spec: convert into a directory with no project"),
        "{held}"
    );
    std::fs::write(dir.path().join("specforge.json"), "{}").unwrap();
    let held = write::check_to(dir.path(), SPEC).unwrap_err();
    assert!(
        held.contains("already holds spec, specforge.json:"),
        "{held}"
    );
}

/// A symbolic link where absorb writes is refused, dangling or not, and left where it is: init would write through
/// it, and a roll back would remove it. It runs on both OSes, its links made by `link_dir` and `link_file` (ADR 0375
/// D4).
#[test]
fn a_symbolic_link_where_absorb_writes_is_refused_and_kept() {
    for name in ["spec", ".specforge", ".gitignore"] {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join(name);
        let target = dir.path().join("nowhere");
        if name == ".gitignore" {
            link_file(&target, &link);
        } else {
            link_dir(&target, &link);
        }
        let refused = write::check_to(dir.path(), SPEC).unwrap_err();
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
    link_dir(dir.path(), &to);
    assert!(
        write::check_to(&to, SPEC)
            .unwrap_err()
            .ends_with("is a symbolic link: name the directory it points to")
    );
}

/// The ledger is a path absorb writes like any other: a symbolic link there is refused, dangling or not, and kept, and
/// a write made after the check refuses it too; a ledger already there, a file, is replaced. It runs on both OSes,
/// its links made by `link_file` (ADR 0375 D4).
#[test]
fn a_symbolic_link_at_the_ledger_path_is_refused_and_never_written_through() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside.json");
    std::fs::write(&outside, "mine\n").unwrap();
    for target in [outside.clone(), dir.path().join("nowhere")] {
        let link = dir.path().join("absorb-ledger.json");
        link_file(&target, &link);
        let refused = write::check_output("ledger", &link).unwrap_err();
        assert!(
            refused.ends_with("is a symbolic link: absorb never writes through one; remove it, or name another --ledger"),
            "{refused}"
        );
        assert!(write::output("ledger", &link, "{}").is_err());
        assert!(std::fs::symlink_metadata(&link).is_ok(), "the link is kept");
        std::fs::remove_file(&link).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "mine\n");
    let held = dir.path().join("held.json");
    std::fs::write(&held, "old").unwrap();
    write::output("ledger", &held, "new").unwrap();
    assert_eq!(std::fs::read_to_string(&held).unwrap(), "new");
    assert!(
        write::check_output("ledger", dir.path())
            .unwrap_err()
            .ends_with("is no file")
    );
}

/// A source whose spec root is its project's root keeps it: a target that already holds a spec file would read it as
/// its own, so it is refused; a spec root that leaves the project is refused; a file a create would write is claimed
/// only when it is not there.
#[test]
fn the_source_spec_root_decides_what_the_target_may_hold() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("crates/a")).unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\n").unwrap();
    assert!(write::check_to(dir.path(), None).is_ok());
    assert!(write::check_to(dir.path(), Some("specs/main")).is_ok());
    std::fs::write(dir.path().join("crates/a/old.spec"), "").unwrap();
    let held = write::check_to(dir.path(), None).unwrap_err();
    // The file is named as the OS writes a path: `crates\a\old.spec` on Windows.
    let named = std::path::Path::new("crates").join("a").join("old.spec");
    assert!(
        held.ends_with(&format!(
            "holds 1 spec file(s) the target would read as its own (the source's spec root is the project's root): \
             {}",
            named.display()
        )),
        "{held}"
    );
    assert!(write::check_to(dir.path(), Some("specs")).is_ok());
    let leaves = write::check_to(dir.path(), Some("../up")).unwrap_err();
    assert!(
        leaves.contains("leaves its project's directory"),
        "{leaves}"
    );

    let mut scaffold = write::check_to(dir.path(), Some("specs")).unwrap();
    assert!(scaffold.claim(&["specs/new.spec".into()]).is_ok());
    let held = scaffold
        .claim(&["Cargo.toml".into(), "crates/a/old.spec".into()])
        .unwrap_err();
    assert!(
        held.ends_with(
            "already holds Cargo.toml, crates/a/old.spec: convert into a directory with no project"
        ),
        "{held}"
    );
}
