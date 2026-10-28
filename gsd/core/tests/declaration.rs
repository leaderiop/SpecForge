//! What `@gsd/core` declares, pinned through the SDK's runtime-free `MockHost`.

mod common;

use gsd_core::build;
use specforge_extension_sdk::testing::MockHost;

fn describe(category: &str) -> serde_json::Value {
    let host = MockHost::new(build());
    let text = host
        .describe_json(category)
        .unwrap_or_else(|| panic!("no category {category}"));
    serde_json::from_str(&text).expect("describe answers JSON")
}

/// The names of the fields a kind declares, in order.
fn fields_of(kind: &str) -> Vec<String> {
    let entities = describe("entities");
    let entity = entities["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == kind))
        .unwrap_or_else(|| panic!("no kind {kind}"));
    entity["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .map(|f| f["name"].as_str().expect("a name").to_string())
        .collect()
}

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

#[test]
fn the_crate_links_serde_json_only_for_its_tests() {
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a [dependencies] table");
    assert!(!dependencies.contains("serde_json"), "{dependencies}");
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
    assert_eq!(book.len(), 60);
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
    // `spec/project.md`, except `key_decisions` (a reference to `decision`, which round r36 declares); a
    // requirement reads its phase back from `phase.requirements`.
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
/// the lesson's `retrospective`; three fields are read back from their inverse, not declared (the requirement's
/// `phase` from `phase.requirements`, the retrospective's `lessons` from `lesson.retrospective`) or wait for a kind a
/// later round declares (the project's `key_decisions`): 72 + 2 - 3.
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
    assert_eq!(total, 72 + 2 - 3, "the kinds' fields");
}
