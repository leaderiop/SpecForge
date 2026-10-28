//! What `@gsd/core` declares for the execution kinds (`spec/execute.md`), pinned through the SDK's runtime-free
//! `MockHost`.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of};

fn kind_of(kind: &str) -> serde_json::Value {
    describe("entities")["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == kind))
        .unwrap_or_else(|| panic!("no kind {kind}"))
        .clone()
}

fn writer(kind: &str, field: &str) -> String {
    field_of(kind, field)["writer"]
        .as_str()
        .unwrap_or("author")
        .to_string()
}

#[test]
fn the_execute_kinds_declare_the_fields_of_the_model() {
    assert_eq!(
        fields_of("summary"),
        [
            "plan",
            "status",
            "one_liner",
            "subsystem",
            "tags",
            "requires",
            "provides",
            "affects",
            "tech_added",
            "tech_patterns",
            "key_decisions",
            "patterns_established",
            "requirements_completed",
            "threat_flags",
            "next_readiness",
            "halted_because",
            "started",
            "completed",
            "duration",
            "actuals",
            "files_created",
            "files_modified"
        ]
    );
    assert_eq!(
        fields_of("deliverable"),
        [
            "summary",
            "description",
            "requirement",
            "human_judgment",
            "rationale",
            "checks"
        ]
    );
    assert_eq!(
        fields_of("deviation"),
        ["summary", "kind", "rule", "text", "files"]
    );
    for (kind, scope) in [
        ("summary", "plan"),
        ("deliverable", "summary"),
        ("deviation", "summary"),
    ] {
        assert_eq!(kind_of(kind)["scoped_by"], scope, "{kind}");
        assert_eq!(field_of(kind, scope)["immutable"], true, "{kind}");
        assert_eq!(field_of(kind, scope)["required"], true, "{kind}");
    }
    for (kind, field) in [
        ("summary", "status"),
        ("summary", "one_liner"),
        ("summary", "subsystem"),
        ("summary", "requirements_completed"),
        ("deliverable", "description"),
        ("deliverable", "human_judgment"),
        ("deviation", "kind"),
        ("deviation", "text"),
    ] {
        assert_eq!(field_of(kind, field)["required"], true, "{kind}.{field}");
    }
    for field in ["started", "completed"] {
        assert_eq!(writer("summary", field), "harness", "{field}");
    }
    for field in ["duration", "actuals", "files_created", "files_modified"] {
        assert_eq!(writer("summary", field), "computed", "{field}");
    }
    // A path a commit touched may be deleted or renamed since: text, never the host's `file_reference` (E016).
    for field in ["files_created", "files_modified"] {
        assert_ne!(
            field_of("summary", field)["file_reference"],
            true,
            "{field}"
        );
    }
    assert_ne!(field_of("deviation", "files")["file_reference"], true);
}

#[test]
fn a_deliverable_check_status_is_a_computed_member_in_the_declaration() {
    let checks = field_of("deliverable", "checks");
    assert_eq!(checks["field_type"], "block_list");
    let member = |name: &str| {
        checks["members"]
            .as_array()
            .and_then(|m| m.iter().find(|m| m["name"] == name))
            .unwrap_or_else(|| panic!("no member {name}: {checks}"))
            .clone()
    };
    assert_eq!(member("status")["writer"], "computed");
    assert_ne!(member("status")["required"], true);
    assert_ne!(member("kind")["writer"], "computed");
    assert_eq!(member("ref")["required"], true);
}

#[test]
fn the_deviation_kind_is_append_only_and_every_field_immutable() {
    assert_eq!(kind_of("deviation")["append_only"], true);
    for field in fields_of("deviation") {
        assert_eq!(field_of("deviation", &field)["immutable"], true, "{field}");
    }
    assert_ne!(kind_of("summary")["append_only"], true);
}

#[test]
fn a_summary_requires_phase_is_a_reference_member() {
    let requires = field_of("summary", "requires");
    assert_eq!(requires["field_type"], "block_list");
    let phase = requires["members"]
        .as_array()
        .and_then(|m| m.iter().find(|m| m["name"] == "phase"))
        .expect("phase");
    assert_eq!(phase["target_kind"], "phase");
    let edges = describe("edges");
    assert!(
        edges["items"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|e| e["label"] == "SummaryRequiresPhase" && e["source_kind"] == "summary"),
        "{edges}"
    );
}
