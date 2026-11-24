use serde_json::{Value, json};

use super::*;
use crate::passes::gates::tests::{gate_of, input};

const HOW: &str = "bump the milestone's major version (Cargo's rule: for 0.x the minor), or restore the item, or \
    commit the updated accepted report when the change is accepted; then run `specforge collect` again: this check \
    reads the API facts of the last collect";

fn shipped(id: &str, at: &str) -> Value {
    json!({ "id": id, "kind": "milestone", "fields": { "completion": format!("at {at}"), "shipped": at } })
}

fn open(id: &str, breaks: &[&str]) -> Value {
    json!({ "id": id, "kind": "milestone", "values": { "api_breaks": breaks } })
}

fn close(subject: &str, entities: Vec<Value>) -> PassInput {
    input(
        gate_of("complete_milestone", Some(subject)),
        entities,
        vec![],
    )
}

#[test]
fn e533_refuses_a_removed_item_without_a_major_bump() {
    let found = e533(&close(
        "v1.8",
        vec![
            shipped("v1.7", "2026-02-01"),
            open("v1.8", &["removed api:rust:a::f"]),
        ],
    ));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "E533");
    assert_eq!(found[0].severity, PassSeverity::Error);
    assert_eq!(found[0].entity.as_deref(), Some("v1.8"));
    assert_eq!(
        found[0].message,
        "API item 'api:rust:a::f' is removed without a major version bump: milestone 'v1.8' is compatible with the \
         last shipped 'v1.7'"
    );
    assert_eq!(found[0].suggestion.as_deref(), Some(HOW));
}

#[test]
fn e533_allows_a_break_at_a_major_bump() {
    let found = e533(&close(
        "v2.0",
        vec![
            shipped("v1.7", "2026-02-01"),
            open("v2.0", &["removed api:rust:a::f"]),
        ],
    ));
    assert!(found.is_empty());
}

#[test]
fn e533_is_silent_outside_a_gate() {
    let mut found = close(
        "v1.8",
        vec![
            shipped("v1.7", "2026-02-01"),
            open("v1.8", &["removed api:rust:a::f"]),
        ],
    );
    found.trigger = None;
    assert!(e533(&found).is_empty());
}

/// The E533 messages of closing `subject` (open, with `breaks`) after `baseline` shipped on 2026-02-01.
fn refused(baseline: &str, subject: &str, breaks: &[&str]) -> Vec<String> {
    let found = e533(&close(
        subject,
        vec![shipped(baseline, "2026-02-01"), open(subject, breaks)],
    ));
    assert!(
        found
            .iter()
            .all(|d| d.code == "E533" && d.entity.as_deref() == Some(subject))
    );
    found.into_iter().map(|d| d.message).collect()
}

#[test]
fn e533_fires_on_each_breaking_change_alone() {
    for change in ["removed", "changed", "narrowed"] {
        let line = format!("{change} api:js:p::f");
        assert_eq!(
            refused("v1.7", "v1.8", &[&line]),
            [format!(
                "API item 'api:js:p::f' is {change} without a major version bump: milestone 'v1.8' is compatible \
                 with the last shipped 'v1.7'"
            )]
        );
    }
}

#[test]
fn e533_fires_on_a_patch_bump_of_a_0x_minor() {
    assert_eq!(refused("v0.7.1", "v0.7.2", &["removed a"]).len(), 1);
}

#[test]
fn e533_fires_on_a_minor_bump_of_a_one_part_version() {
    assert_eq!(refused("v1", "v1.1", &["removed a"]).len(), 1);
}

#[test]
fn e533_allows_a_break_at_a_0x_minor_bump() {
    assert!(refused("v0.7", "v0.8", &["removed a"]).is_empty());
}

#[test]
fn e533_allows_a_break_at_a_first_release() {
    let found = e533(&close("v1.8", vec![open("v1.8", &["removed a"])]));
    assert!(found.is_empty());
}

#[test]
fn e533_is_silent_when_api_breaks_is_empty_or_absent() {
    assert!(refused("v1.7", "v1.8", &[]).is_empty());
    let absent = json!({ "id": "v1.8", "kind": "milestone" });
    let found = e533(&close("v1.8", vec![shipped("v1.7", "2026-02-01"), absent]));
    assert!(found.is_empty());
}

#[test]
fn e533_is_silent_on_a_shipped_subject() {
    let mut subject = shipped("v1.8", "2026-03-01");
    subject["values"] = json!({ "api_breaks": ["removed a"] });
    let found = e533(&close("v1.8", vec![shipped("v1.7", "2026-02-01"), subject]));
    assert!(found.is_empty());
}

#[test]
fn e533_is_silent_at_a_phase_gate_and_on_a_write() {
    let entities = vec![
        shipped("v1.7", "2026-02-01"),
        open("v1.8", &["removed a"]),
        json!({ "id": "v1.8/01", "kind": "phase" }),
    ];
    let phase = input(gate_of("ship", Some("v1.8/01")), entities.clone(), vec![]);
    assert!(e533(&phase).is_empty());
    let write = input(
        json!({ "kind": "write", "subject": "v1.8" }),
        entities,
        vec![],
    );
    assert!(e533(&write).is_empty());
}

#[test]
fn e533_takes_no_baseline_from_another_workstream() {
    let edges = vec![json!({ "source": "a/v1.8", "target": "a", "label": "workstream" })];
    let found = e533(&input(
        gate_of("complete_milestone", Some("a/v1.8")),
        vec![
            shipped("v1.7", "2026-02-01"),
            open("a/v1.8", &["removed a"]),
        ],
        edges.clone(),
    ));
    assert!(found.is_empty());
    // The same milestone in workstream `a` is the baseline.
    let mut edges = edges;
    edges.push(json!({ "source": "a/v1.7", "target": "a", "label": "workstream" }));
    let found = e533(&input(
        gate_of("complete_milestone", Some("a/v1.8")),
        vec![
            shipped("a/v1.7", "2026-02-01"),
            open("a/v1.8", &["removed a"]),
        ],
        edges,
    ));
    assert_eq!(found.len(), 1);
    assert!(found[0].message.ends_with("the last shipped 'a/v1.7'"));
}

#[test]
fn e533_takes_the_latest_shipped_milestone_as_the_baseline() {
    // v2.0 shipped last: compatible with v2.1, so it fires; v1.9 would allow the break.
    let found = e533(&close(
        "v2.1",
        vec![
            shipped("v2.0", "2026-02-01"),
            shipped("v1.9", "2026-01-01"),
            open("v2.1", &["removed a"]),
        ],
    ));
    assert_eq!(found.len(), 1);
    assert!(found[0].message.ends_with("the last shipped 'v2.0'"));
    // v1.9 shipped after v2.0 (a backport): it is the baseline, and v2.1 is a major bump over it.
    let found = e533(&close(
        "v2.1",
        vec![
            shipped("v2.0", "2026-01-01"),
            shipped("v1.9", "2026-02-01"),
            open("v2.1", &["removed a"]),
        ],
    ));
    assert!(found.is_empty());
}

#[test]
fn e533_breaks_a_tie_of_dates_by_the_higher_version() {
    for order in [["v2.0", "v1.9"], ["v1.9", "v2.0"]] {
        let found = e533(&close(
            "v2.1",
            vec![
                shipped(order[0], "2026-02-01"),
                shipped(order[1], "2026-02-01"),
                open("v2.1", &["removed a"]),
            ],
        ));
        assert_eq!(found.len(), 1, "{order:?}");
        assert!(found[0].message.ends_with("the last shipped 'v2.0'"));
    }
}

#[test]
fn e533_fires_once_on_a_subject_that_is_not_a_version() {
    let found = e533(&close(
        "mvp",
        vec![
            shipped("v1.7", "2026-02-01"),
            open("mvp", &["removed a", "changed b"]),
        ],
    ));
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].message,
        "milestone 'mvp' has API breaks and no major version bump over the last shipped 'v1.7' can be shown: 'mvp' \
         is not a version"
    );
    assert!(
        found[0]
            .suggestion
            .as_deref()
            .is_some_and(|s| s.contains("run `specforge collect` again"))
    );
}

#[test]
fn e533_takes_no_baseline_whose_name_is_not_a_version() {
    // A release candidate or an `mvp` that shipped last is skipped: v1.7 is the baseline.
    for skipped in ["mvp", "v1.8-rc1"] {
        let found = e533(&close(
            "v1.8",
            vec![
                shipped("v1.7", "2026-01-01"),
                shipped(skipped, "2026-02-01"),
                open("v1.8", &["removed a"]),
            ],
        ));
        assert_eq!(found.len(), 1, "{skipped}");
        assert!(found[0].message.ends_with("the last shipped 'v1.7'"));
    }
    // With no shipped version at all, it is a first release.
    let found = e533(&close(
        "v2.0",
        vec![shipped("mvp", "2026-02-01"), open("v2.0", &["removed a"])],
    ));
    assert!(found.is_empty());
}

#[test]
fn e533_bounds_its_errors_at_25_and_one_summary() {
    let lines: Vec<String> = (0..60)
        .map(|i| format!("removed api:rust:a::f{i:02}"))
        .collect();
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let found = refused("v1.7", "v1.8", &lines);
    assert_eq!(found.len(), 26);
    assert_eq!(
        found[25],
        "35 more API items broken without a major version bump: milestone 'v1.8' is compatible with the last \
         shipped 'v1.7'"
    );
}

#[test]
fn e533_counts_the_more_line_of_a_capped_field() {
    let mut lines: Vec<String> = (0..50)
        .map(|i| format!("removed api:rust:a::f{i:02}"))
        .collect();
    lines.push("more 10".to_string());
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let found = refused("v1.7", "v1.8", &lines);
    assert_eq!(found.len(), 26);
    assert!(found[25].starts_with("35 more API items"));
}

#[test]
fn e533_escapes_every_id_it_echoes() {
    let found = e533(&close(
        "v1.8\n",
        vec![
            shipped("v1.7", "2026-02-01"),
            open("v1.8\n", &["removed a\nb'`c"]),
        ],
    ));
    assert_eq!(found.len(), 1);
    assert!(found[0].message.ends_with("'v1.8\\n' is not a version"));
    for d in &found {
        assert!(!d.message.contains('\n'), "{:?}", d.message);
        assert!(d.suggestion.as_deref().is_some_and(|s| !s.contains('\n')));
    }
    let found = refused("v1.7", "v1.8", &["removed a\nb'`c"]);
    assert_eq!(
        found,
        [
            "API item 'a\\nb\\'`c' is removed without a major version bump: milestone 'v1.8' is compatible with the \
          last shipped 'v1.7'"
        ]
    );
}

#[test]
fn e533_escapes_an_id_derive_kept_raw_once() {
    let id = "api:rust:a::f\u{7}\n'";
    let derived = crate::passes::derive::interface::values(&close(
        "v1.8",
        vec![
            shipped("v1.7", "2026-02-01"),
            json!({ "id": "v1.8", "kind": "milestone" }),
            json!({ "id": id, "kind": "api_item", "fields": { "change": "removed" } }),
        ],
    ));
    assert_eq!(derived.len(), 1);
    let mut subject = json!({ "id": "v1.8", "kind": "milestone" });
    subject["values"] = json!({ "api_breaks": derived[0].value });
    let found = e533(&close("v1.8", vec![shipped("v1.7", "2026-02-01"), subject]));
    assert_eq!(
        found.iter().map(|d| d.message.as_str()).collect::<Vec<_>>(),
        [
            "API item 'api:rust:a::f\\u{7}\\n\\'' is removed without a major version bump: milestone 'v1.8' is \
             compatible with the last shipped 'v1.7'"
        ]
    );
}

mod e535;
