//! What `@gsd/core` declares, pinned through the SDK's runtime-free `MockHost`.

mod common;

use common::{describe, field_of, fields_of};
use gsd_core::build;
use specforge_extension_sdk::testing::MockHost;

#[test]
fn the_handshake_names_gsd_core_and_its_tracked_release() {
    let host = MockHost::new(build());
    let handshake: serde_json::Value =
        serde_json::from_str(&host.handshake_json()).expect("the handshake is JSON");
    assert_eq!(handshake["name"], "@gsd/core");
    assert_eq!(handshake["ext_short"], "gsd");
    assert_eq!(handshake["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        handshake["keywords"],
        serde_json::json!([format!("tracks:{}", gsd_core::TRACKS)])
    );
}

/// D-r44-23: the extension\'s version is the crate\'s, so a bump moves `Cargo.toml` and the lock and no test.
#[test]
fn the_handshake_names_the_crate_version() {
    let host = MockHost::new(build());
    let handshake: serde_json::Value =
        serde_json::from_str(&host.handshake_json()).expect("the handshake is JSON");
    assert_eq!(handshake["version"], env!("CARGO_PKG_VERSION"));
    assert!(
        env!("CARGO_PKG_VERSION").split('.').count() == 3,
        "a release version"
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

/// Every `.rs` file under `dir`.
fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// D-r44-20: `tracks` names the snapshot that is vendored. Moving it fails the build until every check reads the new
/// snapshot, which is the README's "moves only when the suite passes".
#[test]
fn tracks_names_the_vendored_snapshot() {
    let version = gsd_core::TRACKS
        .strip_prefix("gsd-core@")
        .expect("TRACKS is gsd-core@<version>");
    let core = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut snapshots: Vec<String> = std::fs::read_dir(core.join("../parity/upstream"))
        .expect("the vendored snapshots")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    snapshots.sort();
    assert_eq!(
        snapshots,
        [format!("v{version}")],
        "one snapshot, the tracked one"
    );
    // Every path in the three crates that names a snapshot directory names that one, and the checks name it through
    // `gsd_kit::snapshot!`, which reads the tracked version. (The needles are built, so this file does not match itself.)
    let needle = ["parity/upstream/", "v"].concat();
    let call = ["snapshot", "!("].concat();
    let mut files = Vec::new();
    for dir in [
        "src",
        "tests",
        "../kit/src",
        "../security/src",
        "../security/tests",
    ] {
        rust_files(&core.join(dir), &mut files);
    }
    let mut named = 0;
    for file in files {
        let text = std::fs::read_to_string(&file).expect("a source file");
        named += text.matches(&call).count();
        for (at, _) in text.match_indices(&needle) {
            let rest = &text[at + needle.len()..];
            let found: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            assert_eq!(
                found.trim_end_matches('.'),
                version,
                "{} names another snapshot",
                file.display()
            );
            named += 1;
        }
    }
    assert!(named > 10, "the checks read the snapshot: {named}");
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
            "git_range",
            "progress"
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
    assert_eq!(total, 72 + 3 - 2, "the kinds' fields");
}

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
            "status",
            "digest"
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
    // + 3: the quick plan's `plan` on context, decision and research (G3).
    assert_eq!(total, 95 - 1 + 1 + 1 + 3, "the kinds' fields");
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
