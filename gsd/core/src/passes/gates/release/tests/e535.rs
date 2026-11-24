//! E535's package and backwards parts (ADR 0390 D6).

use serde_json::{Value, json};

use super::{close, shipped};
use crate::passes::gates::release::e535;
use crate::passes::gates::tests::{gate_of, input};
use specforge_extension_sdk::prelude::*;

const PACKAGES: &str = "set the package's version to the milestone's, or list the package in \
    `specforge.release_exempt` when the milestone does not release it; then run `specforge collect` again: this check \
    reads the package versions of the last collect";

/// An open milestone whose computed `release.packages` is `packages`.
fn releasing(id: &str, packages: &[&str]) -> Value {
    json!({ "id": id, "kind": "milestone", "values": { "release": { "packages": packages } } })
}

fn messages(found: &[PassDiagnostic]) -> Vec<&str> {
    found.iter().map(|d| d.message.as_str()).collect()
}

#[test]
fn e535_refuses_a_package_not_at_the_milestones_version() {
    let found = e535(&close(
        "v1.8",
        vec![
            shipped("v1.7", "2026-02-01"),
            releasing("v1.8", &["b 1.7.0"]),
        ],
    ));
    assert_eq!(
        messages(&found),
        ["package 'b' is at version '1.7.0', not at milestone 'v1.8''s version"]
    );
    assert_eq!(found[0].code, "E535");
    assert_eq!(found[0].severity, PassSeverity::Error);
    assert_eq!(found[0].entity.as_deref(), Some("v1.8"));
    assert_eq!(found[0].suggestion.as_deref(), Some(PACKAGES));
}

#[test]
fn e535_allows_a_release_whose_packages_match() {
    let found = e535(&close(
        "v1.8",
        vec![shipped("v1.7", "2026-02-01"), releasing("v1.8", &[])],
    ));
    assert!(found.is_empty());
    // Nothing observed: no `release` at all.
    let found = e535(&close(
        "v1.8",
        vec![
            shipped("v1.7", "2026-02-01"),
            json!({ "id": "v1.8", "kind": "milestone" }),
        ],
    ));
    assert!(found.is_empty());
}

#[test]
fn e535_refuses_a_version_that_goes_backwards() {
    let found = e535(&close(
        "v1.6",
        vec![shipped("v1.7", "2026-02-01"), releasing("v1.6", &[])],
    ));
    assert_eq!(
        messages(&found),
        ["milestone 'v1.6' goes backwards from the last shipped 'v1.7': its version is not later"]
    );
    assert_eq!(
        found[0].suggestion.as_deref(),
        Some("name the milestone by a version later than the last shipped 'v1.7'")
    );
    // The same version is not later either.
    let found = e535(&close(
        "v1.7.0",
        vec![shipped("v1.7", "2026-02-01"), releasing("v1.7.0", &[])],
    ));
    assert_eq!(found.len(), 1);
}

#[test]
fn e535_allows_a_later_version_and_a_first_release() {
    let found = e535(&close(
        "v1.8",
        vec![shipped("v1.7", "2026-02-01"), releasing("v1.8", &[])],
    ));
    assert!(found.is_empty());
    let found = e535(&close("v0.1", vec![releasing("v0.1", &[])]));
    assert!(found.is_empty());
}

#[test]
fn e535_takes_no_baseline_from_another_workstream() {
    let edges = vec![json!({ "source": "a/v1.6", "target": "a", "label": "workstream" })];
    let found = e535(&input(
        gate_of("complete_milestone", Some("a/v1.6")),
        vec![shipped("v1.7", "2026-02-01"), releasing("a/v1.6", &[])],
        edges,
    ));
    assert!(found.is_empty());
}

#[test]
fn e535_is_silent_outside_a_gate_and_for_a_name_that_is_no_version() {
    let mut write = close(
        "v1.6",
        vec![
            shipped("v1.7", "2026-02-01"),
            releasing("v1.6", &["b 1.7.0"]),
        ],
    );
    write.trigger = None;
    assert!(e535(&write).is_empty());
    let found = e535(&close(
        "mvp",
        vec![
            shipped("v1.7", "2026-02-01"),
            releasing("mvp", &["b 1.7.0"]),
        ],
    ));
    assert!(found.is_empty());
}

#[test]
fn e535_names_at_most_25_packages_then_counts_the_rest() {
    let lines: Vec<String> = (0..30)
        .map(|n| format!("p{n:02} 0.1.0"))
        .chain(["more 5".to_string()])
        .collect();
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let found = e535(&close("v1.8", vec![releasing("v1.8", &lines)]));
    assert_eq!(found.len(), 26);
    assert_eq!(
        found[25].message,
        "10 more packages are not at milestone 'v1.8''s version"
    );
    assert!(
        found
            .iter()
            .all(|d| d.suggestion.as_deref() == Some(PACKAGES))
    );
}

#[test]
fn e535_escapes_what_it_echoes() {
    let found = e535(&close("v1.8", vec![releasing("v1.8", &["a\nb'c 1\t0"])]));
    assert_eq!(
        messages(&found),
        ["package 'a\\nb\\'c' is at version '1\\t0', not at milestone 'v1.8''s version"]
    );
}
