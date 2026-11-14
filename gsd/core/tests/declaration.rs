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

/// A truth's `proof` names `@code/core`'s test facts (ADR 0331 D2), and a project works without them: the peer is
/// optional, so `specforge add @gsd/core` does not enable it and a project without it is no W021. A path field
/// needs `@specforge/checks`, a required peer.
#[test]
fn the_handshake_names_code_core_as_an_optional_peer() {
    let host = MockHost::new(build());
    let handshake: serde_json::Value =
        serde_json::from_str(&host.handshake_json()).expect("the handshake is JSON");
    assert_eq!(
        handshake["peer_dependencies"],
        serde_json::json!([
            {"name": "@code/core", "version": "^1.0", "optional": true},
            {"name": "@specforge/checks", "version": "^1.0", "optional": false},
        ])
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
    assert_eq!(book.len(), 70);
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

/// ADR 0356: the context route of each role `@gsd/core` spawns, and the two that bound a written context.
#[test]
fn the_contexts_golden() {
    let contexts = describe("contexts");
    assert_eq!(
        contexts["items"].as_array().map(Vec::len),
        Some(14),
        "{contexts}"
    );
    common::golden("contexts.json", &contexts);
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
        [
            "text", "category", "tier", "reason", "defined", "status", "shapes", "text_en"
        ]
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

/// ADR 0331 D8: the one route, `gsd/requirement-thread` once qualified by the short name, as `specforge trace` follows
/// it from a requirement.
#[test]
fn the_requirement_route_is_declared() {
    assert_eq!(
        describe("routes")["items"],
        serde_json::json!([{
            "name": "requirement-thread",
            "description": "A requirement's thread: its plans, tasks, their code items and commits, proving tests and truths",
            "start": "requirement",
            "steps": [
                {"name": "plan", "via": "<plan.requirements", "label": "plan"},
                {"name": "task", "via": "<task.plan"},
                {"name": "item", "via": ".items"},
                {"name": "commit", "via": "<commit.task", "from": "task",
                    "annotations": [{"field": "linked_by"}]},
                {"name": "test", "via": "<truth.requirement.proof", "from": "start",
                    "annotations": [{"field": "status", "absent": "not run"}]},
                {"name": "truth", "via": "<truth.requirement", "from": "start",
                    "annotations": [{"judgement": "E501"}]},
            ]
        }])
    );
}

/// Every authored path field of `@gsd/core` and the mode its file check follows (ADR 0350): what the kit lists, and
/// what the declaration writes. A history path has no check; every other one checks with `@specforge/checks:file`,
/// waiting for its owner as its mode says.
#[test]
fn the_authored_path_fields_carry_their_owner_mode() {
    use gsd_kit::dsl::{Mode, Writer};
    let authored: Vec<(String, Mode)> = gsd_core::path_fields()
        .into_iter()
        .filter(|p| p.writer == Writer::Author)
        .map(|p| (p.name(), p.mode))
        .collect();
    let row = |name: &str, mode| (name.to_string(), mode);
    let mut expected = vec![
        row("plan.files_modified", Mode::Planned),
        row("plan.files_deleted", Mode::WhileOpen),
        row("plan.context_refs", Mode::Present),
        row("task.files", Mode::Planned),
        row("task.read_first", Mode::Planned),
        row("artifact.path", Mode::Planned),
        row("key_link.from", Mode::Planned),
        row("key_link.to", Mode::Planned),
        row("context.canonical_refs.paths", Mode::Present),
        row("validation.config_file", Mode::Planned),
        row("validation_row.file", Mode::Planned),
        row("wave0_item.path", Mode::Planned),
        row("verify_gap.artifacts.path", Mode::Present),
        row("uat_gap.artifacts.path", Mode::Present),
        row("broken_window.file", Mode::WhileOpen),
        row("refactor_proposal.target_file", Mode::WhileOpen),
        row("deviation.files", Mode::History),
    ];
    let mut found = authored.clone();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(found, expected);
    let mut checked = 0;
    for (name, mode) in authored {
        let mut parts = name.split('.');
        let (kind, field) = (
            parts.next().expect("a kind"),
            parts.next().expect("a field"),
        );
        let mut declared = field_of(kind, field);
        if let Some(member) = parts.next() {
            declared = declared["members"]
                .as_array()
                .and_then(|m| m.iter().find(|m| m["name"] == member))
                .cloned()
                .unwrap_or_else(|| panic!("no member {name}"));
        }
        let checkers: Vec<&str> = declared["checks"]
            .as_array()
            .map(|c| c.iter().filter_map(|c| c["checker"].as_str()).collect())
            .unwrap_or_default();
        let (want_checks, want_while): (&[&str], serde_json::Value) = match mode {
            Mode::History => (&[], serde_json::Value::Null),
            Mode::Present => (&["@specforge/checks:file"], serde_json::Value::Null),
            Mode::Planned => (&["@specforge/checks:file"], "complete".into()),
            Mode::WhileOpen => (&["@specforge/checks:file"], "pending".into()),
        };
        assert_eq!(checkers, want_checks, "{name}");
        assert_eq!(declared["checks_while"], want_while, "{name}");
        checked += usize::from(!checkers.is_empty());
    }
    assert_eq!(checked, 16);
}
