//! What `@gsd/core` declares for the execution kinds (`spec/execute.md`), pinned through the SDK's runtime-free
//! `MockHost`.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of};
use gsd_core::build;
use specforge_extension_sdk::testing::MockHost;

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

#[test]
fn the_commit_kind_is_harness_written() {
    assert_eq!(
        fields_of("commit"),
        [
            "hash",
            "task",
            "plan",
            "type",
            "subject",
            "at",
            "files",
            "created",
            "insertions",
            "deletions"
        ]
    );
    for field in fields_of("commit") {
        assert_eq!(writer("commit", &field), "harness", "{field}");
        assert_ne!(field_of("commit", &field)["required"], true, "{field}");
    }
    assert_eq!(field_of("commit", "hash")["immutable"], true);
    // The commit is in the project scope, and a path it touched is text: a later commit may delete it.
    assert!(kind_of("commit")["scoped_by"].is_null());
    for field in ["files", "created"] {
        assert_ne!(field_of("commit", field)["file_reference"], true, "{field}");
    }
    // The task and the plan read their commits back by one name each.
    for field in ["task", "plan"] {
        let reference = field_of("commit", field);
        assert_eq!(reference["inverse_of"], "commits", "{field}");
        assert_eq!(reference["target_kind"], field);
    }
    assert!(!fields_of("task").contains(&"commits".to_string()));
    assert!(!fields_of("plan").contains(&"commits".to_string()));
}

#[test]
fn the_ops_kinds_declare_the_fields_of_the_model() {
    assert_eq!(
        fields_of("pause"),
        ["phase", "plan", "at", "by", "reason", "handoff"]
    );
    assert_eq!(
        fields_of("handoff"),
        [
            "phase",
            "task",
            "state",
            "current_state",
            "completed_work",
            "remaining_work",
            "decisions_made",
            "blockers",
            "context",
            "next_action"
        ]
    );
    assert_eq!(
        fields_of("blocker"),
        ["text", "phase", "plan", "raised", "resolved"]
    );
    for kind in ["pause", "handoff"] {
        assert_eq!(kind_of(kind)["scoped_by"], "phase", "{kind}");
        let phase = field_of(kind, "phase");
        assert_eq!(phase["immutable"], true, "{kind}");
        assert_ne!(
            phase["required"], true,
            "{kind}: a pause or handoff may have no phase"
        );
    }
    assert!(kind_of("blocker")["scoped_by"].is_null());
    for (kind, field) in [
        ("pause", "reason"),
        ("handoff", "state"),
        ("handoff", "current_state"),
        ("handoff", "next_action"),
        ("blocker", "text"),
    ] {
        assert_eq!(field_of(kind, field)["required"], true, "{kind}.{field}");
    }
    for (kind, field) in [("pause", "at"), ("pause", "by"), ("blocker", "raised")] {
        assert_eq!(writer(kind, field), "harness", "{kind}.{field}");
    }
    // `resolved` is authored, so its `at` gets the E107 rule; a `pause.at` is harness-written, so it gets none.
    assert_eq!(writer("blocker", "resolved"), "author");
    let rules = describe("validation_rules");
    let e107: Vec<(String, String)> = rules["items"]
        .as_array()
        .expect("rules")
        .iter()
        .filter(|r| r["code"].to_string().contains("E107"))
        .filter(|r| {
            ["pause", "handoff", "blocker"].contains(&r["target_kind"].as_str().unwrap_or_default())
        })
        .map(|r| {
            (
                r["target_kind"].as_str().unwrap_or_default().to_string(),
                r["field"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    assert_eq!(e107, [("blocker".to_string(), "resolved.at".to_string())]);
    let edges = describe("edges");
    for label in [
        "HandoffBlockers",
        "HandoffDecisionsMade",
        "HandoffCompletedWorkTask",
    ] {
        assert!(
            edges["items"]
                .as_array()
                .expect("edges")
                .iter()
                .any(|e| e["label"] == label),
            "{label} in {edges}"
        );
    }
}

#[test]
fn the_model_has_forty_kinds() {
    let kinds = describe("entities");
    assert_eq!(kinds["items"].as_array().expect("kinds").len(), 40);
}

#[test]
fn the_model_counts_its_fields() {
    let total: usize = [
        "summary",
        "deliverable",
        "deviation",
        "commit",
        "pause",
        "handoff",
        "blocker",
    ]
    .iter()
    .map(|k| fields_of(k).len())
    .sum();
    assert_eq!(total, 22 + 6 + 5 + 10 + 6 + 10 + 5);
}

#[test]
fn the_git_collector_runs_git_log_and_writes_a_plan() {
    let git = MockHost::new(build())
        .declaration()
        .collectors
        .into_iter()
        .find(|c| c.name == "gsd-git")
        .expect("the gsd-git collector");
    assert!(git.writes, "it creates the commit entities");
    assert_eq!(git.capture.as_deref(), Some("stdout"));
    assert_eq!(git.run[..2], ["git", "-c"]);
    assert!(
        git.run.contains(&"--numstat".to_string()) && git.run.contains(&"--summary".to_string())
    );
    assert!(git.run.contains(&"log".to_string()) && git.run.contains(&"--no-merges".to_string()));
    // Named explicitly (`--runner gsd-git`): a detected collector would be selected by a plain `specforge collect`.
    assert!(git.auto_detect.is_none());
    assert_eq!(git.input_formats, ["git-log"]);
}

#[test]
fn the_model_has_the_collectors_it_builds() {
    let names: Vec<String> = MockHost::new(build())
        .declaration()
        .collectors
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, ["gsd-git"]);
}
