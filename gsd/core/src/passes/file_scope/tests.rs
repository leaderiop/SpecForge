use super::*;

fn plan(id: &str, wave: Option<&str>, modified: &[&str], deleted: &[&str]) -> PassEntity {
    let mut plan = PassEntity {
        id: id.into(),
        kind: "plan".into(),
        ..Default::default()
    };
    if let Some(wave) = wave {
        plan.fields.insert("wave".into(), wave.into());
    }
    plan.values
        .insert("files_modified".into(), serde_json::json!(modified));
    plan.values
        .insert("files_deleted".into(), serde_json::json!(deleted));
    plan
}

fn task(id: &str, files: &[&str]) -> PassEntity {
    let mut task = PassEntity {
        id: id.into(),
        kind: "task".into(),
        ..Default::default()
    };
    task.values.insert("files".into(), serde_json::json!(files));
    task
}

fn run(entities: Vec<PassEntity>) -> Vec<PassDiagnostic> {
    check(&PassInput {
        entities,
        ..Default::default()
    })
}

#[test]
fn same_wave_overlap_is_w305_once_per_path_on_the_later_plan() {
    let found = run(vec![
        plan("v/1/1-01", Some("1"), &["src/a.js", "src/b.js"], &[]),
        plan(
            "v/1/1-02",
            Some("1"),
            &["src/a.js", "src/b.js", "src/c.js"],
            &[],
        ),
    ]);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|d| d.code == "W305" && d.entity.as_deref() == Some("v/1/1-02"))
    );
    assert!(
        found[0]
            .message
            .contains("plans 'v/1/1-01' and 'v/1/1-02' of wave 1 both modify 'src/a.js'"),
        "{found:?}"
    );
}

#[test]
fn a_deleted_file_counts_as_overlap() {
    let found = run(vec![
        plan("v/1/1-01", Some("1"), &["src/a.js"], &[]),
        plan("v/1/1-02", Some("1"), &[], &["src/a.js"]),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn other_waves_other_phases_and_plans_without_a_wave_do_not_overlap() {
    let found = run(vec![
        plan("v/1/1-01", Some("1"), &["src/a.js"], &[]),
        plan("v/1/1-02", Some("2"), &["src/a.js"], &[]),
        plan("v/2/2-01", Some("1"), &["src/a.js"], &[]),
        plan("261028-a1b", None, &["src/a.js"], &[]),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_path_holding_a_comma_is_one_item() {
    let found = run(vec![
        plan("v/1/1-01", Some("1"), &["src/a.js", "src/b.js"], &[]),
        plan("v/1/1-02", Some("1"), &["src/a.js,src/b.js"], &[]),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_task_file_outside_its_plan_is_w306_per_item() {
    let found = run(vec![
        plan("v/1/1-01", Some("1"), &["src/a.js"], &["src/old.js"]),
        task(
            "v/1/1-01/T1",
            &["src/a.js", "src/old.js", "src/x.js", "src/y.js"],
        ),
    ]);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|d| d.code == "W306" && d.entity.as_deref() == Some("v/1/1-01/T1"))
    );
    assert!(found[0].message.contains("'src/x.js'") && found[1].message.contains("'src/y.js'"));
}
