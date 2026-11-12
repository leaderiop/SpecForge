use super::*;

#[test]
fn nothing_is_computed_for_a_task_without_a_cd_or_prefix() {
    let task = PassEntity {
        id: "v1.0/01/01-01/T1".into(),
        kind: "task".into(),
        ..Default::default()
    };
    let out = compute(&PassInput {
        entities: vec![task],
        ..Default::default()
    });
    assert!(out.values.is_empty());
}

#[test]
fn a_planned_member_path_is_read_through_its_block() {
    let plan = PassEntity {
        id: "v1.0/01/01-01".into(),
        kind: "plan".into(),
        values: [
            ("files".to_string(), serde_json::json!(["./a.js"])),
            (
                "rows".to_string(),
                serde_json::json!([{"file": "b.js"}, {"file": ["c.js", "d.js"]}]),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    let at = |field, member| PathField {
        kind: "plan",
        field,
        member,
        mode: Mode::Planned,
        writer: Writer::Author,
    };
    let input = PassInput {
        entities: vec![plan],
        ..Default::default()
    };
    let planned = [at("files", None), at("rows", Some("file"))];
    assert_eq!(
        declared_paths(&input, "v1.0/01/01-01", &planned),
        ["a.js", "b.js", "c.js", "d.js"]
    );
}
