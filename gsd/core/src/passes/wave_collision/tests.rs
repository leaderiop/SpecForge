use super::*;

fn entity(id: &str, kind: &str, field: &str, paths: &[&str]) -> PassEntity {
    let mut entity = PassEntity {
        id: id.into(),
        kind: kind.into(),
        ..Default::default()
    };
    entity.values.insert(field.into(), serde_json::json!(paths));
    entity
}

fn plan(id: &str, paths: &[&str]) -> PassEntity {
    entity(id, "plan", "files_modified", paths)
}

fn task(id: &str, files: &[&str]) -> PassEntity {
    entity(id, "task", "files", files)
}

/// The entities, with `(plan, depends on)` pairs as `depends_on` edges.
fn graph(entities: Vec<PassEntity>, depends: &[(&str, &str)]) -> PassInput {
    PassInput {
        entities,
        edges: depends
            .iter()
            .map(|(source, target)| PassEdge {
                source: (*source).into(),
                target: (*target).into(),
                label: "depends_on".into(),
            })
            .collect(),
        ..Default::default()
    }
}

/// `(entity, message)` of each W526, checking its code and severity.
fn reported(input: &PassInput) -> Vec<(String, String)> {
    check(input)
        .into_iter()
        .map(|d| {
            assert_eq!(d.code, "W526");
            assert_eq!(d.severity, PassSeverity::Warning);
            (d.entity.expect("an entity"), d.message)
        })
        .collect()
}

#[test]
fn a_placement_with_no_plan_above_its_wave_reports_nothing() {
    let input = graph(
        vec![
            plan("v/1/E", &["src/a.js"]),
            plan("v/1/L", &["src/b.js"]),
            plan("v/1/M", &["src/a.js"]),
        ],
        &[("v/1/M", "v/1/E")],
    );
    assert!(check(&input).is_empty());
}

#[test]
fn a_direct_push_names_the_paths_the_earlier_plan_and_its_tasks() {
    let input = graph(
        vec![
            plan("v/1/E", &["src/a.js", "src/b.js"]),
            task("v/1/E/T1", &["src/a.js"]),
            task("v/1/E/T2", &["src/c.js"]),
            plan("v/1/L", &["src/b.js", "src/a.js"]),
        ],
        &[],
    );
    assert_eq!(
        reported(&input),
        [(
            "v/1/L".to_string(),
            "plan 'v/1/L' shares 'src/a.js', 'src/b.js' with 'v/1/E' (tasks: v/1/E/T1): it runs in strict \
             wave 2, after 'v/1/E' in strict wave 1, not in its wave 1"
                .to_string()
        )]
    );
}

#[test]
fn a_direct_push_no_task_names_says_so() {
    let input = graph(
        vec![plan("v/1/E", &["src/a.js"]), plan("v/1/L", &["src/a.js"])],
        &[],
    );
    assert_eq!(
        reported(&input)[0].1,
        "plan 'v/1/L' shares 'src/a.js' with 'v/1/E' (no task names them): it runs in strict wave 2, after \
         'v/1/E' in strict wave 1, not in its wave 1"
    );
}

#[test]
fn a_transitive_push_names_the_plan_it_depends_on() {
    let input = graph(
        vec![
            plan("v/1/E", &["src/a.js"]),
            plan("v/1/L", &["src/a.js"]),
            plan("v/1/D", &["src/d.js"]),
        ],
        &[("v/1/D", "v/1/L")],
    );
    let found = reported(&input);
    assert_eq!(
        found.iter().map(|(e, _)| e.as_str()).collect::<Vec<_>>(),
        ["v/1/D", "v/1/L"]
    );
    assert_eq!(
        found[0].1,
        "plan 'v/1/D' depends on 'v/1/L', which runs in strict wave 2: it runs in strict wave 3, not in its wave 2"
    );
}

#[test]
fn the_highest_strict_wave_binds_and_a_tie_goes_to_the_smaller_id() {
    // 1-02 and 1-04 are both pushed to 2; 1-05 shares a path with each and with 1-01 (strict 1): 1-02 binds.
    let input = graph(
        vec![
            plan("v/1/1-01", &["src/a.js"]),
            plan("v/1/1-02", &["src/a.js", "src/x.js"]),
            plan("v/1/1-03", &["src/b.js"]),
            plan("v/1/1-04", &["src/b.js", "src/y.js"]),
            plan("v/1/1-05", &["src/a.js", "src/x.js", "src/y.js"]),
        ],
        &[],
    );
    let found = reported(&input);
    let last = found
        .iter()
        .find(|(e, _)| e == "v/1/1-05")
        .expect("1-05 is pushed");
    assert_eq!(
        last.1,
        "plan 'v/1/1-05' shares 'src/a.js', 'src/x.js' with 'v/1/1-02' (no task names them): it runs in strict wave \
         3, after \
         'v/1/1-02' in strict wave 2, not in its wave 1"
    );
}

#[test]
fn a_dependency_that_also_shares_a_path_gives_the_transitive_text() {
    let input = graph(
        vec![
            plan("v/1/E", &["src/a.js"]),
            plan("v/1/L", &["src/a.js", "src/l.js"]),
            plan("v/1/D", &["src/l.js"]),
        ],
        &[("v/1/D", "v/1/L")],
    );
    assert_eq!(
        reported(&input)[0],
        (
            "v/1/D".to_string(),
            "plan 'v/1/D' depends on 'v/1/L', which runs in strict wave 2: it runs in strict wave 3, not in its \
             wave 2"
                .to_string()
        )
    );
}

#[test]
fn a_cycle_and_a_quick_plan_report_nothing() {
    let input = graph(
        vec![
            plan("v/1/E", &["src/a.js"]),
            plan("v/1/L", &["src/a.js"]),
            plan("261028-a1b", &["src/a.js"]),
            plan("261028-b2c", &["src/a.js"]),
        ],
        &[("v/1/E", "v/1/L"), ("v/1/L", "v/1/E")],
    );
    assert!(check(&input).is_empty());
}
