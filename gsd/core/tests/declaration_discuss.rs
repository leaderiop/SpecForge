//! What `@gsd/core` declares for the discuss kinds (`spec/discuss.md`), pinned through the SDK's runtime-free
//! `MockHost`.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of};

#[test]
fn context_decision_and_deferred_idea_declare_the_fields_of_the_model() {
    assert_eq!(
        fields_of("context"),
        [
            "phase",
            "plan",
            "gathered",
            "boundary",
            "discretion",
            "specifics",
            "canonical_refs",
            "reusable_assets",
            "patterns",
            "integration_points",
            "status"
        ]
    );
    assert_eq!(
        fields_of("decision"),
        [
            "phase",
            "plan",
            "area",
            "text",
            "rationale",
            "outcome",
            "reversibility",
            "settled",
            "origin",
            "superseded_by"
        ]
    );
    assert_eq!(
        fields_of("deferred_idea"),
        ["text", "phase", "why", "origin", "status"]
    );
}

/// `name` of the field `field` of `kind`, with its described properties.
#[test]
fn the_discuss_kinds_are_scoped_by_their_phase() {
    let entities = describe("entities");
    for kind in ["context", "decision", "deferred_idea"] {
        let entity = entities["items"]
            .as_array()
            .and_then(|k| k.iter().find(|k| k["name"] == kind))
            .expect("the kind");
        assert_eq!(entity["scoped_by"], "phase", "{kind}");
    }
}

#[test]
fn canonical_ref_paths_are_files_from_the_project_root() {
    let refs = field_of("context", "canonical_refs");
    let paths = refs["members"]
        .as_array()
        .and_then(|m| m.iter().find(|m| m["name"] == "paths"))
        .expect("paths");
    assert_eq!(paths["field_type"], "string_list");
    assert_eq!(
        paths["checks"],
        serde_json::json!([{"checker": "@specforge/checks:file", "args": {"root": "project"}}])
    );
}

#[test]
fn a_decision_origin_is_required_and_immutable_and_its_phase_is_not_required() {
    let origin = field_of("decision", "origin");
    assert_eq!(
        (&origin["required"], &origin["immutable"]),
        (&true.into(), &true.into())
    );
    assert_ne!(field_of("decision", "phase")["required"], true);
    assert_eq!(field_of("context", "status")["writer"], "computed");
}

#[test]
fn the_spec_kinds_declare_the_fields_of_the_model() {
    assert_eq!(
        fields_of("phase_spec"),
        [
            "phase",
            "context_pack",
            "created",
            "goal",
            "background",
            "in_scope",
            "out_of_scope",
            "constraints",
            "acceptance",
            "scores"
        ]
    );
    assert_eq!(
        fields_of("locked_req"),
        [
            "spec",
            "title",
            "current",
            "target",
            "acceptance",
            "traces",
            "shapes",
            "text_en"
        ]
    );
    let scores = field_of("phase_spec", "scores");
    let members: Vec<(&str, i64, i64)> = scores["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| {
            (
                m["name"].as_str().expect("a name"),
                m["minimum"].as_i64().expect("a minimum"),
                m["maximum"].as_i64().expect("a maximum"),
            )
        })
        .collect();
    assert_eq!(
        members,
        [
            ("goal", 0, 100),
            ("boundary", 0, 100),
            ("constraint", 0, 100),
            ("acceptance", 0, 100),
            ("ambiguity", 0, 100)
        ]
    );
}

/// The edge probe's rows of a phase spec (ADR 0333): each scoped by its spec, which it never changes.
#[test]
fn the_probe_kinds_declare_the_fields_of_the_model() {
    assert_eq!(
        fields_of("edge"),
        [
            "spec",
            "requirement",
            "category",
            "status",
            "verification",
            "resolution",
            "reason"
        ]
    );
    assert_eq!(
        fields_of("spec_prohibition"),
        [
            "spec",
            "requirement",
            "statement",
            "status",
            "verification",
            "reason",
            "check_kind",
            "check_target",
            "check_rule",
            "check_violation_fixture",
            "check_clean_fixture"
        ]
    );
    let entities = describe("entities");
    for kind in ["edge", "spec_prohibition"] {
        let entity = entities["items"]
            .as_array()
            .and_then(|k| k.iter().find(|k| k["name"] == kind))
            .expect(kind);
        assert_eq!(entity["scoped_by"], "spec", "{kind}");
        assert_eq!(field_of(kind, "spec")["immutable"], true, "{kind}");
        assert_eq!(
            field_of(kind, "spec")["target_kind"],
            "phase_spec",
            "{kind}"
        );
        assert_eq!(
            field_of(kind, "requirement")["target_kind"],
            "locked_req",
            "{kind}"
        );
        assert_eq!(
            field_of(kind, "status")["enum_values"],
            serde_json::json!(["resolved", "dismissed", "unresolved"]),
            "{kind}"
        );
    }
    assert_eq!(
        field_of("edge", "category")["enum_values"],
        serde_json::json!([
            "boundary",
            "adjacency",
            "empty",
            "encoding",
            "ordering",
            "precision",
            "idempotency",
            "concurrency",
            "unclassified"
        ])
    );
    assert_eq!(
        field_of("edge", "verification")["enum_values"],
        serde_json::json!(["explicit", "backstop"])
    );
    assert_eq!(
        field_of("spec_prohibition", "verification")["enum_values"],
        serde_json::json!(["test", "judgment"])
    );
    assert_eq!(
        field_of("spec_prohibition", "check_kind")["enum_values"],
        serde_json::json!(["node_test", "lint_rule"])
    );
    for kind in ["locked_req", "requirement"] {
        assert_eq!(
            field_of(kind, "shapes")["enum_values"],
            serde_json::json!(["numeric_range", "collection", "text", "stateful", "io"]),
            "{kind}"
        );
    }
}

/// The description lists the kind as append-only (a removed interview is E085, proven black-box too).
#[test]
fn every_field_of_an_interview_is_immutable() {
    let entities = describe("entities");
    let interview = entities["items"]
        .as_array()
        .and_then(|k| k.iter().find(|k| k["name"] == "interview"))
        .expect("interview");
    assert_eq!(interview["scoped_by"], "spec");
    assert_eq!(interview["append_only"], true);
    let fields = interview["fields"].as_array().expect("fields");
    assert_eq!(fields.len(), 6);
    for field in fields {
        assert_eq!(field["immutable"], true, "{}", field["name"]);
    }
}

#[test]
fn the_discussion_entries_are_append_only() {
    assert_eq!(fields_of("discussion"), ["phase", "entries", "checkpoint"]);
    // the checkpoint is set and cleared as the discussion goes: not append-only
    assert_ne!(field_of("discussion", "checkpoint")["append_only"], true);
    let entries = field_of("discussion", "entries");
    assert_eq!(entries["field_type"], "block_list");
    assert_eq!(entries["append_only"], true);
    let members: Vec<&str> = entries["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| m["name"].as_str().expect("a name"))
        .collect();
    let entry = "area question options chosen mode notes confidence evidence";
    assert_eq!(members.join(" "), entry);
}

/// `discuss.md` listed 95 fields over its 16 kinds before `source.research` was added. The model fixes: the decision's
/// `from_summary` waits for `summary` and `source` gains `research` (the model says every item has one): 95 - 1 + 1 =
/// 95, the research's and the spec's `context_pack` included.
#[test]
fn the_discuss_kinds_declare_every_field_of_discuss_md() {
    let kinds = [
        "context",
        "decision",
        "deferred_idea",
        "phase_spec",
        "locked_req",
        "interview",
        "edge",
        "spec_prohibition",
        "discussion",
        "research",
        "finding",
        "stack_choice",
        "pattern",
        "pitfall",
        "dont_hand_roll",
        "open_question",
        "source",
        "package_audit",
    ];
    let total: usize = kinds.iter().map(|k| fields_of(k).len()).sum();
    // + 3: the quick plan's `plan` on context, decision and research. + 2 + 7 + 11: the edge probe's `shapes` and
    // `text_en` on locked_req, and its two row kinds, edge (7 fields) and spec_prohibition (11) (ADR 0333).
    // + 1: the discussion's `checkpoint` (a block's members are not counted). + 1: the deferred idea's `status`, open
    // until an executor's out-of-scope discovery is resolved. + 1: its `origin`, discussion or execution, so only a
    // discussion idea is part of the phase's context.
    assert_eq!(
        total,
        95 - 1 + 1 + 3 + 2 + 7 + 11 + 1 + 1 + 1,
        "the kinds' fields"
    );
}

#[test]
fn every_research_item_is_scoped_by_its_research() {
    let entities = describe("entities");
    for kind in [
        "finding",
        "stack_choice",
        "pattern",
        "pitfall",
        "dont_hand_roll",
        "open_question",
        "source",
        "package_audit",
    ] {
        let entity = entities["items"]
            .as_array()
            .and_then(|k| k.iter().find(|k| k["name"] == kind))
            .expect("the kind");
        assert_eq!(entity["scoped_by"], "research", "{kind}");
        assert_eq!(fields_of(kind)[0], "research", "{kind}");
    }
}

/// ADR 0356: the pack a research, a spec or a validation was written against is the host's record, a block of the
/// route, its digest and its budget; the context holds no digest of its own.
#[test]
fn the_recorded_context_packs_are_context_values() {
    assert!(fields_of("context").iter().all(|f| f != "digest"));
    for kind in ["research", "phase_spec", "validation"] {
        let field = field_of(kind, "context_pack");
        assert_eq!(field["writer"], "context", "{kind}");
        assert_eq!(field["field_type"], "block", "{kind}");
        let members: Vec<&str> = field["members"]
            .as_array()
            .expect("members")
            .iter()
            .map(|m| m["name"].as_str().expect("a name"))
            .collect();
        assert_eq!(members, ["route", "digest", "max_tokens"], "{kind}");
        assert!(
            fields_of(kind).iter().all(|f| f != "context_digest"),
            "{kind}"
        );
    }
}
