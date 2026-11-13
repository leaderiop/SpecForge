//! A context route read over a project (ADR 0356): each section digests as the judged input of its selector, a files
//! section lists its paths with their digests, the settings section its values, and a pack's digest covers exactly
//! the sections it served.

use std::sync::Arc;

use serde_json::json;
use specforge_project::context::{SectionKind, SectionRead, digest, read, served, subject};
use specforge_project::judged::{Cited, file_digest};
use specforge_project::testing::{
    CODE_CORE, CONTEXT_INSTRUCTIONS, CONTEXT_PLAN, GSD_CONTEXT_SPEC, GSD_JUDGED, TestProject,
    gsd_context_project, gsd_context_runtime, gsd_routed_project_over,
};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_protocol_types::{ContextTier, JudgedFile};
use specforge_registry::{ContextRoute, PlainDisk};
use specforge_test::prelude::*;

fn compile(project: &TestProject) -> CompiledProject {
    project.compile_in(Arc::new(gsd_context_runtime()) as SharedRuntime)
}

fn route<'a>(compiled: &'a CompiledProject, name: &str) -> &'a ContextRoute {
    compiled
        .environment()
        .registries()
        .contexts()
        .named(name)
        .unwrap_or_else(|| panic!("the route {name}"))
}

fn sections(compiled: &CompiledProject, name: &str) -> Vec<SectionRead> {
    read(compiled, &PlainDisk, route(compiled, name), CONTEXT_PLAN)
}

fn section<'a>(sections: &'a [SectionRead], name: &str) -> &'a SectionRead {
    sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the section {name}"))
}

/// `(name, selector, digest)` of every section but `left_out`, as a pack lists what it served.
fn listed(sections: &[SectionRead], left_out: &str) -> Vec<(String, String, String)> {
    sections
        .iter()
        .filter(|s| s.name != left_out)
        .map(|s| (s.name.clone(), s.selector.clone(), s.digest.clone()))
        .collect()
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "a section digests as the judged input of the same selector does"
)]
fn a_section_digests_as_the_judged_input_of_its_selector() {
    // The project's directory lives as long as the reads: they read its files.
    let project = gsd_context_project();
    let compiled = compile(&project);
    let read = sections(&compiled, "gsd/plan-checker");
    let rule = compiled
        .environment()
        .registries()
        .judged_rule("gsd/W311")
        .expect("the plan-check rule");
    let inputs = specforge_project::judged::inputs(
        &compiled,
        &PlainDisk,
        CONTEXT_PLAN,
        rule,
        &Cited::default(),
    )
    .expect("the rule judges a plan");
    assert_eq!(read.len(), inputs.reports.len(), "{read:?}");
    for (section, input) in read.iter().zip(&inputs.reports) {
        assert_eq!(section.name, input.name);
        assert_eq!(section.tier, ContextTier::Core);
        assert_eq!(section.selector, input.selector, "{}", section.name);
        assert_eq!(section.members, input.members, "{}", section.name);
        assert_eq!(section.digest, input.digest, "{}", section.name);
    }
    let body = section(&read, "body");
    assert_eq!(body.kind, SectionKind::Entities);
    assert!(
        body.members.contains(&"v1.0/02/02-01/T1".to_string()),
        "{body:?}"
    );
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "a files section lists paths and digests and a settings section the values of its settings"
)]
fn a_files_section_lists_paths_and_digests_and_a_settings_section_its_values() {
    let project = gsd_context_project();
    let compiled = compile(&project);
    let read = sections(&compiled, "gsd/executor");
    let names: Vec<&str> = read.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "plan",
            "body",
            "phase",
            "requirements",
            "context",
            "decisions",
            "instructions",
            "research_items",
            "siblings",
            "settings"
        ]
    );
    let instructions = section(&read, "instructions");
    assert_eq!(instructions.kind, SectionKind::Files);
    assert_eq!(instructions.tier, ContextTier::Enrich);
    assert_eq!(
        instructions.files,
        [
            JudgedFile {
                path: "AGENTS.md".to_string(),
                digest: "absent".to_string()
            },
            JudgedFile {
                path: "CLAUDE.md".to_string(),
                digest: file_digest(CONTEXT_INSTRUCTIONS.as_bytes())
            },
        ]
    );
    assert_eq!(instructions.members, ["AGENTS.md", "CLAUDE.md"]);
    let settings = section(&read, "settings");
    assert_eq!(settings.kind, SectionKind::Settings);
    assert_eq!(settings.tier, ContextTier::Core);
    assert_eq!(settings.selector, "");
    assert_eq!(settings.settings.get("plan_check"), Some(&json!(true)));
    assert!(settings.digest.starts_with("sha256:"), "{settings:?}");
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "the pack digest covers the sections served and recomputes from the pack"
)]
fn the_pack_digest_covers_what_is_served() {
    let project = gsd_context_project();
    let compiled = compile(&project);
    let before = sections(&compiled, "gsd/executor");
    for entities in before.iter().filter(|s| s.kind == SectionKind::Entities) {
        assert_eq!(
            served(&compiled, &PlainDisk, entities, &entities.members),
            entities.digest,
            "{}",
            entities.name
        );
    }
    let body = section(&before, "body");
    assert_ne!(
        served(&compiled, &PlainDisk, body, &body.members[1..]),
        body.digest
    );

    // A decision the executor reads changes: a pack that served its section moves, one that left it out does not.
    let edited = GSD_CONTEXT_SPEC.replace("with no trimming", "trimmed of spaces");
    let edited = gsd_routed_project_over(&edited, &[GSD_JUDGED, CODE_CORE])
        .file("CLAUDE.md", CONTEXT_INSTRUCTIONS);
    let compiled_after = compile(&edited);
    let after = sections(&compiled_after, "gsd/executor");
    let entity = subject(&compiled, &PlainDisk, CONTEXT_PLAN);
    assert_eq!(entity, subject(&compiled_after, &PlainDisk, CONTEXT_PLAN));
    let pack = |sections: &[SectionRead], left_out: &str| {
        digest(
            "gsd/executor",
            CONTEXT_PLAN,
            &entity,
            &listed(sections, left_out),
        )
    };
    assert_eq!(pack(&before, ""), pack(&before, ""));
    assert_ne!(pack(&before, ""), pack(&after, ""));
    assert_eq!(pack(&before, "decisions"), pack(&after, "decisions"));
}

#[specforge_test(
    behavior = "serve_context_pack",
    verify = "the pack digest covers the sections served and recomputes from the pack"
)]
fn the_pack_digest_moves_with_its_subject_when_no_section_serves_it() {
    let project = gsd_context_project();
    let compiled = compile(&project);
    let edited =
        GSD_CONTEXT_SPEC.replace("Save and load tasks.json", "Save and load the task file");
    let edited = gsd_routed_project_over(&edited, &[GSD_JUDGED, CODE_CORE])
        .file("CLAUDE.md", CONTEXT_INSTRUCTIONS);
    let compiled_after = compile(&edited);
    // No section listed: the subject is the only thing the pack served.
    let pack = |compiled: &CompiledProject| {
        let entity = subject(compiled, &PlainDisk, CONTEXT_PLAN);
        digest("gsd/executor", CONTEXT_PLAN, &entity, &[])
    };
    assert_eq!(pack(&compiled), pack(&compiled));
    assert_ne!(pack(&compiled), pack(&compiled_after));
}
