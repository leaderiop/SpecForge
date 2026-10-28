//! What `@gsd/core` declares, pinned through the SDK's runtime-free `MockHost`.

mod common;

use common::{describe, field_of, fields_of};
use gsd_core::build;
use specforge_extension_sdk::testing::MockHost;

#[test]
fn the_handshake_names_gsd_core_and_tracks_1_18_0() {
    let host = MockHost::new(build());
    let handshake: serde_json::Value =
        serde_json::from_str(&host.handshake_json()).expect("the handshake is JSON");
    assert_eq!(handshake["name"], "@gsd/core");
    assert_eq!(handshake["ext_short"], "gsd");
    assert_eq!(handshake["version"], "0.1.0");
    assert_eq!(
        handshake["keywords"],
        serde_json::json!(["tracks:gsd-core@1.18.0"])
    );
}

/// r34 L12: no dependency the library does not use. `serde_json` is the SDK's JSON crate, linked already; the
/// manifest reader of the verify-path pass uses it.
#[test]
fn serde_json_is_a_dependency_only_while_the_library_uses_it() {
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a [dependencies] table");
    let used = include_str!("../src/passes/verify_paths/manifest.rs")
        .split("#[cfg(test)]")
        .next()
        .is_some_and(|code| code.contains("serde_json::"));
    assert_eq!(dependencies.contains("serde_json"), used, "{dependencies}");
}

#[test]
fn the_family_carries_its_own_licence_texts() {
    assert!(include_str!("../../LICENSE-MIT").contains("MIT License"));
    assert!(include_str!("../../LICENSE-APACHE").contains("Apache License"));
}

#[test]
fn the_tracked_release_is_the_manifests() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains(&format!("tracks = \"{}\"", gsd_core::TRACKS)),
        "Cargo.toml tracks a different release"
    );
}

#[test]
fn the_code_book_matches_rules_md() {
    let rows = common::rules_md();
    let book = gsd_core::codes::CODES;
    let mut from_md: Vec<&str> = rows.iter().map(|r| r.code.as_str()).collect();
    let mut from_book: Vec<&str> = book.iter().map(|c| c.id).collect();
    from_md.sort_unstable();
    from_book.sort_unstable();
    assert_eq!(
        from_book, from_md,
        "the code book and rules.md list different codes"
    );
    assert_eq!(book.len(), 63);
    for row in &rows {
        let code = book
            .iter()
            .find(|c| c.id == row.code)
            .expect("listed above");
        // A rule off under compat has its severity when on in the strict column.
        let letter = if row.compat == "—" {
            &row.strict
        } else {
            &row.compat
        };
        assert_eq!(
            code.severity.letter().to_string(),
            letter[..1],
            "{}: compat severity",
            row.code
        );
        let strict = if row.strict == "—" {
            letter
        } else {
            &row.strict
        };
        assert_eq!(
            code.strict.letter().to_string(),
            strict[..1],
            "{}: strict severity",
            row.code
        );
        assert_eq!(
            code.id[..1],
            *code.severity.letter().to_string(),
            "{}: a code's letter is its severity",
            row.code
        );
    }
}

#[test]
fn the_code_book_golden() {
    common::golden("codebook.json", &describe("codes"));
}

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
        ["text", "category", "tier", "reason", "defined", "status"]
    );
}

#[test]
fn the_kinds_golden() {
    common::golden(
        "kinds.json",
        &serde_json::json!({ "entities": describe("entities"), "edges": describe("edges") }),
    );
}

#[test]
fn milestone_and_phase_declare_the_fields_of_the_model() {
    assert_eq!(
        fields_of("milestone"),
        [
            "name",
            "delivered",
            "accomplishments",
            "whats_next",
            "abandoned",
            "workstream",
            "completion",
            "state",
            "shipped",
            "stats",
            "git_range"
        ]
    );
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
        ["retrospective", "text", "milestone", "top", "kind"]
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
/// the lesson's `retrospective`; two fields are read back from their inverse, not declared (the requirement's
/// `phase` from `phase.requirements`, the retrospective's `lessons` from `lesson.retrospective`): 72 + 2 - 2.
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
    assert_eq!(total, 72 + 2 - 2, "the kinds' fields");
}

#[test]
fn context_decision_and_deferred_idea_declare_the_fields_of_the_model() {
    assert_eq!(
        fields_of("context"),
        [
            "phase",
            "gathered",
            "boundary",
            "discretion",
            "specifics",
            "canonical_refs",
            "reusable_assets",
            "patterns",
            "integration_points",
            "status",
            "digest"
        ]
    );
    assert_eq!(
        fields_of("decision"),
        [
            "phase",
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
    assert_eq!(fields_of("deferred_idea"), ["text", "phase", "why"]);
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
    assert_eq!(paths["file_reference"], true);
    assert_eq!(paths["file_base"], "project");
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
            "context_digest",
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
        ["spec", "title", "current", "target", "acceptance", "traces"]
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
    assert_eq!(fields_of("discussion"), ["phase", "entries"]);
    let entries = field_of("discussion", "entries");
    assert_eq!(entries["field_type"], "block_list");
    assert_eq!(entries["append_only"], true);
    let members: Vec<&str> = entries["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| m["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(members, ["area", "question", "options", "chosen", "mode"]);
}

/// `discuss.md` listed 95 fields over its 16 kinds before r36 added two rows (`context.digest`, `source.research`). The model fixes: the decision's `from_summary` waits for `summary`
/// (r38) and `source` gains `research` (the model says every item has one); the context gains its computed `digest`
/// (the fix of round r36): 95 - 1 + 1 + 1 = 96, the research's and the spec's `context_digest` included.
#[test]
fn the_discuss_kinds_declare_every_field_of_discuss_md() {
    let kinds = [
        "context",
        "decision",
        "deferred_idea",
        "phase_spec",
        "locked_req",
        "interview",
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
    assert_eq!(total, 95 - 1 + 1 + 1, "the kinds' fields");
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

#[test]
fn the_context_digest_is_computed_and_the_recorded_copies_are_harness_values() {
    assert_eq!(field_of("context", "digest")["writer"], "computed");
    for kind in ["research", "phase_spec"] {
        assert_eq!(
            field_of(kind, "context_digest")["writer"],
            "harness",
            "{kind}"
        );
    }
}
