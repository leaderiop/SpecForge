//! A rename of an entity collected facts name, against what the stores hold beside it (ADR 0346 D7): a name the
//! store holds that names no entity, a name one fact holds for the entity and another for something else, two
//! renames of one name the ledger was merged with, and a second store placed in a file a rename rewrote.

use serde_json::json;
use specforge_project::testing::{TestProject, stored_with};
use specforge_test_macros::test as specforge_test;

use crate::collected::{FACT, KEY, covered_and_placed, project, rename_now, stale_lines};

/// [`project`] whose fact covers `T1` and `T9`, which names no entity, both written on line 1 of `a.test.js`.
fn beside_a_dangling_name() -> TestProject {
    let p = project();
    let source = "// covers T1 T9\n";
    std::fs::write(p.root().join("a.test.js"), source).unwrap();
    let at = |from: usize| json!([{"file": "a.test.js", "start_line": 1, "start_col": from, "end_line": 1, "end_col": from + 2}]);
    stored_with(
        p.root(),
        "node",
        KEY,
        json!([{"kind": "test", "id": FACT, "fields": {"file": "a.test.js", "covers": ["T1", "T9"]},
                "named_at": {"covers": {"T1": at(11), "T9": at(14)}}}]),
        [(
            "a.test.js".to_string(),
            specforge_ops::revision::of_bytes(source.as_bytes()),
        )]
        .into(),
    );
    p
}

/// A rename to the name a fact holds and nothing carries is a capture: the store would hold it twice, once for the
/// renamed entity and once for nothing, and the next rename could not tell them apart.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename to a name a fact holds for no entity is refused as name_captured"
)]
fn a_rename_to_a_name_a_fact_holds_for_no_entity_is_captured() {
    let p = beside_a_dangling_name();

    let refused = rename_now(&p, "T1", "T9").unwrap_err();

    assert!(refused.starts_with("name_captured: "), "{refused}");
    assert!(refused.contains("'T9'"), "{refused}");
    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        "// covers T1 T9\n"
    );
    assert!(stale_lines(&p).is_empty());
}

/// A ledger whose renames already made two held names read as one (written before a capture was refused, or merged):
/// a rename of that name writes a stale line for each, so the serve shifts and checks every place it wrote.
#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "a rename of a name two held names are read as writes a stale line for each"
)]
fn a_rename_of_a_name_two_held_names_read_as_writes_a_line_for_each() {
    let p = beside_a_dangling_name();
    p.write(
        "truths.spec",
        format!("truth T9 \"One\" {{\n  proof [\"{FACT}\"]\n}}\n"),
    );
    std::fs::write(p.root().join("a.test.js"), "// covers T9 T9\n").unwrap();
    let store = std::fs::read(p.root().join(crate::collected::STORE)).unwrap();
    let ledger = p.root().join("specforge-harness");
    let mut lines =
        specforge_project::harness::Ledger::parse(&std::fs::read_to_string(&ledger).unwrap())
            .ledger;
    lines.insert(specforge_project::harness::Line::stale(
        "node", KEY, &store, "covers", "T1", "T9",
    ));
    lines.insert(specforge_project::harness::Line::moved("T9", "T1"));
    std::fs::write(&ledger, lines.canonical()).unwrap();

    rename_now(&p, "T9", "T7").unwrap();

    assert_eq!(
        std::fs::read_to_string(p.root().join("a.test.js")).unwrap(),
        "// covers T7 T7\n"
    );
    assert_eq!(
        stale_lines(&p),
        [
            ("covers".into(), "T1".into(), "T7".into()),
            ("covers".into(), "T9".into(), "T7".into())
        ]
    );
    assert_eq!(covered_and_placed(&p), (vec!["T7".to_string()], true));
}
