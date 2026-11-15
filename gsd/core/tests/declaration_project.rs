//! What `@gsd/core` declares for the project and roadmap kinds (`spec/project.md`), pinned through the SDK's
//! runtime-free `MockHost`.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of};

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
            "text",
            "area",
            "tier",
            "reason",
            "defined",
            "category",
            "priority",
            "metric",
            "source",
            "tags",
            "status",
            "satisfied_by",
            "proof",
            "shapes",
            "text_en"
        ]
    );
}

#[test]
fn milestone_and_phase_declare_the_fields_of_the_model() {
    let milestone = "name delivered accomplishments whats_next goal target_features exit_criteria starts targets \
        depends_on owner contributors artifact_types tags abandoned workstream completion state first_commit shipped \
        stats git_range progress branch risk_level";
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
            "completed",
            "risk_level"
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
    // + 8: the milestone's fields beyond GSD (ADR 0357); + 2: the milestone's and the phase's `risk_level`.
    // + 2: the requirement's computed `proof` (its thread's state) and `satisfied_by` (the items its tasks name).
    // + 5: the requirement's quality `category`, `priority`, `metric`, `source` and `tags` (beyond GSD).
    assert_eq!(
        total,
        72 + 3 - 2 + 1 + 2 + 2 + 2 + 2 + 8 + 2 + 2 + 5,
        "the kinds' fields"
    );
}

/// The milestone's planning fields beyond GSD (ADR 0357): typed, authored, none required or write-once, and
/// `depends_on` a list of milestones that names no reverse (a milestone reads its dependents by query).
#[test]
fn the_milestone_fields_beyond_gsd_are_optional() {
    for (field, field_type) in [
        ("exit_criteria", "string_list"),
        ("starts", "date"),
        ("targets", "date"),
        ("depends_on", "reference_list"),
        ("owner", "string"),
        ("contributors", "string_list"),
        ("artifact_types", "string_list"),
        ("tags", "string_list"),
    ] {
        let declared = field_of("milestone", field);
        assert_eq!(declared["field_type"], field_type, "{declared}");
        assert_ne!(declared["required"], true, "{declared}");
        assert_ne!(declared["immutable"], true, "{declared}");
        assert!(declared["writer"].is_null(), "{declared}");
    }
    let depends_on = field_of("milestone", "depends_on");
    assert_eq!(depends_on["target_kind"], "milestone", "{depends_on}");
    assert!(depends_on["inverse_of"].is_null(), "{depends_on}");
}
