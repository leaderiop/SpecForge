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
