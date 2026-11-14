//! What `@gsd/core` declares for the project and roadmap kinds (`spec/project.md`), pinned through the SDK's
//! runtime-free `MockHost`.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, fields_of};

#[test]
fn project_and_requirement_declare_the_fields_of_the_model() {
    // `spec/project.md`; a requirement reads its phase back from `phase.requirements`.
    assert_eq!(
        fields_of("project"),
        [
            "name",
            "what",
            "core_value",
            "customer",
            "revenue_model",
            "success_metric",
            "context",
            "constraints",
            "out_of_scope",
            "key_decisions",
            "project_code",
            "validated",
            "active",
            "updated",
            "windows_open",
            "windows_waived",
            "windows_fixed",
            "windows_total"
        ]
    );
    assert_eq!(
        fields_of("requirement"),
        [
            "text", "category", "tier", "reason", "defined", "status", "shapes", "text_en"
        ]
    );
}

#[test]
fn milestone_and_phase_declare_the_fields_of_the_model() {
    let milestone = "name delivered accomplishments whats_next goal target_features abandoned workstream completion \
        state first_commit shipped stats git_range progress branch";
    let milestone: Vec<&str> = milestone.split_whitespace().collect();
    assert_eq!(fields_of("milestone"), milestone);
    assert_eq!(
        fields_of("phase"),
        [
            "milestone",
            "name",
            "slug",
            "goal",
            "depends_on",
            "requirements",
            "success_criteria",
            "mode",
            "tag",
            "inserted",
            "backlog",
            "branch",
            "frontend",
            "status",
            "plans_complete",
            "completed"
        ]
    );
}

#[test]
fn a_phase_is_scoped_by_its_milestone() {
    let entities = describe("entities");
    let phase = entities["items"]
        .as_array()
        .and_then(|k| k.iter().find(|k| k["name"] == "phase"))
        .expect("phase");
    assert_eq!(phase["scoped_by"], "milestone");
}

#[test]
fn the_learning_kinds_declare_the_fields_of_the_model() {
    assert_eq!(fields_of("workstream"), ["purpose", "status"]);
    assert_eq!(
        fields_of("retrospective"),
        [
            "milestone",
            "built",
            "worked",
            "inefficient",
            "patterns",
            "cost",
            "phases",
            "plans",
            "sessions"
        ]
    );
    assert_eq!(
        fields_of("lesson"),
        [
            "retrospective",
            "text",
            "milestone",
            "top",
            "kind",
            "graduated",
            "status"
        ]
    );
    assert_eq!(
        fields_of("methodology"),
        [
            "name",
            "diagnoses",
            "recommends",
            "apply_when",
            "status",
            "superseded_by"
        ]
    );
}

/// `project.md` lists 8 kinds and 72 top-level fields. The fixes of the model add the milestone's `workstream` and
/// the lesson's `retrospective`; the milestone's computed `progress` (r38) is a third addition; two fields are read back from their inverse, not declared (the requirement's
/// `phase` from `phase.requirements`, the retrospective's `lessons` from `lesson.retrospective`): 72 + 3 - 2.
#[test]
fn the_roadmap_kinds_declare_every_field_of_project_md() {
    let kinds = [
        "project",
        "requirement",
        "milestone",
        "phase",
        "workstream",
        "retrospective",
        "lesson",
        "methodology",
    ];
    let total: usize = kinds.iter().map(|k| fields_of(k).len()).sum();
    // + 2: the requirement's `shapes` and `text_en`, which the spec-less edge probe reads (ADR 0333).
    // + 2: the lesson's `graduated` and `status`; + 2: the milestone's `goal` and `target_features`.
    assert_eq!(total, 72 + 3 - 2 + 1 + 2 + 2 + 2 + 2, "the kinds' fields");
}
