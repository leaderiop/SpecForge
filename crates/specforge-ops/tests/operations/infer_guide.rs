//! The inference guide (plan 04, ADR 0015 "Prompt read views"): what an
//! agent is told to look for to infer a project's entities from code.

use serde_json::json;
use specforge_extension_sdk::prelude::*;
use specforge_ops::infer::{guide, kind_guide};
use specforge_ops::view::ProjectView;
use specforge_project::coverage::RecordedCoverage;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::build_registries;
use specforge_test::prelude::*;

use crate::view_support::Project;

/// A declaration of `name` that declares each of `kinds` (a keyword and an
/// inference guide).
fn extension(name: &str, kinds: &[(&str, Option<&str>)]) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    for (kind, guide) in kinds {
        c.kind(kind, |k| {
            k.description(&format!("A {kind}"));
            if let Some(guide) = guide {
                k.inference_guide(guide);
            }
        });
    }
    c.declaration()
}

/// The project whose source is `source`, built over `declarations`.
fn project_of(source: &str, declarations: Vec<ExtensionDeclaration>) -> Project {
    Project::new(source, build_registries(declarations))
}

#[specforge_test(
    behavior = "compute_inference_guide",
    verify = "a kind's guide is its extension's guide, then the project's guide for the kind under Project-specific"
)]
fn a_guide_is_the_extension_s_then_the_project_s() {
    let mut project = project_of(
        "",
        vec![extension(
            "@t/soft",
            &[("behavior", Some("Look for public functions"))],
        )],
    );
    project.env.config.inference.global = Some("This is a Rust project".to_string());
    project.env.config.inference.kinds.insert(
        "behavior".to_string(),
        "In our codebase, behaviors are in use_cases/".to_string(),
    );
    let view = project.view();

    let guide = guide(&view);
    assert_eq!(
        guide.kinds[0].guide,
        "Look for public functions\n\n**Project-specific:**\nIn our codebase, behaviors are in use_cases/"
    );
    assert_eq!(guide.conventions, Some("This is a Rust project"));
    assert_eq!(
        kind_guide(&view, "behavior").unwrap().guide,
        guide.kinds[0].guide
    );
}

#[specforge_test(
    behavior = "compute_inference_guide",
    verify = "a kind only the project guides has the project's guide alone, and one nobody guides an empty guide"
)]
fn a_kind_without_an_extension_guide_has_the_project_s_alone() {
    let mut project = project_of(
        "",
        vec![extension("@t/soft", &[("behavior", None), ("event", None)])],
    );
    project
        .env
        .config
        .inference
        .kinds
        .insert("behavior".to_string(), "Look in use_cases/".to_string());
    let view = project.view();

    let guide = guide(&view);
    assert_eq!(guide.kind("behavior").unwrap().guide, "Look in use_cases/");
    assert_eq!(guide.kind("event").unwrap().guide, "");
    assert_eq!(guide.conventions, None);
}

#[specforge_test(
    behavior = "compute_inference_guide",
    verify = "a kind guide lists every field registered on the kind, by name, with its type"
)]
fn a_kind_guide_lists_every_registered_field() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/soft", "1.0.0"));
    c.kind("behavior", |k| {
        k.field("description", |f| {
            f.field_type(FieldType::String).description("A description");
        });
    });
    // The extension's shared field is registered on the kind too.
    c.shared_field("tags", |f| {
        f.field_type(FieldType::StringList);
    });
    let project = project_of("", vec![c.declaration()]);
    let guide = kind_guide(&project.view(), "behavior").unwrap();

    let fields: Vec<(&str, &str)> = guide
        .fields
        .iter()
        .map(|f| (f.name(), f.field_type().as_str()))
        .collect();
    assert_eq!(fields, [("description", "string"), ("tags", "string_list")]);
    assert_eq!(guide.to_json()["fields"][0]["description"], "A description");
}

#[specforge_test(
    behavior = "compute_inference_guide",
    verify = "the example entity writes its required fields and three optional ones, each the way its type is written"
)]
fn the_example_writes_each_field_by_its_type() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/soft", "1.0.0"));
    c.kind("widget", |k| {
        k.field("owner", |f| {
            f.field_type(FieldType::Reference)
                .target_kind("widget")
                .required();
        });
        k.field("steps", |f| {
            f.field_type(FieldType::StringList).required();
        });
        k.field("active", |f| {
            f.field_type(FieldType::Bool);
        });
        k.field("count", |f| {
            f.field_type(FieldType::Integer);
        });
        k.field("methods", |f| {
            f.field_type(FieldType::Block);
        });
        k.field("refs_to", |f| {
            f.field_type(FieldType::ReferenceList).target_kind("widget");
        });
    });
    let project = project_of("", vec![c.declaration()]);
    let example = kind_guide(&project.view(), "widget").unwrap().example();

    // Required fields first (in name order), then the first three optional
    // ones by name: active, count, methods; refs_to is the fourth.
    assert_eq!(
        example,
        "widget example_widget \"Example Title\" {\n  owner ref_id\n  steps [\"item1\", \"item2\"]\n  active true\n  count 0\n  methods {\n  }\n}"
    );

    // And the remaining types.
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/soft", "1.0.0"));
    c.kind("gadget", |k| {
        k.field("level", |f| {
            f.field_type(FieldType::Enum).enum_values(&["low", "high"]);
        });
        k.field("note", |f| {
            f.field_type(FieldType::String);
        });
        k.field("refs_to", |f| {
            f.field_type(FieldType::ReferenceList).target_kind("gadget");
        });
    });
    let project = project_of("", vec![c.declaration()]);
    assert_eq!(
        kind_guide(&project.view(), "gadget").unwrap().example(),
        "gadget example_gadget \"Example Title\" {\n  level low\n  note \"...\"\n  refs_to [ref_1, ref_2]\n}"
    );
}

#[specforge_test(
    behavior = "compute_inference_guide",
    verify = "a kind declared by two extensions is guided by the one that registered it"
)]
fn a_kind_declared_twice_is_guided_by_the_first() {
    let project = project_of(
        "",
        vec![
            extension("@t/first", &[("behavior", Some("first"))]),
            extension("@t/second", &[("behavior", Some("second"))]),
        ],
    );
    let view = project.view();

    let kind = kind_guide(&view, "behavior").unwrap();
    assert_eq!(kind.guide, "first");
    assert_eq!(kind.extension, "@t/first");
    let guide = guide(&view);
    assert_eq!(guide.kinds.len(), 1);
    assert_eq!(guide.kinds[0].extension, "@t/first");
}

#[specforge_test(
    behavior = "compute_inference_guide",
    verify = "the guide's spec directory is the project's spec root, relative to its root"
)]
fn the_spec_directory_is_the_spec_root_relative_to_the_root() {
    let mut project = project_of("", Vec::new());
    project.env.root = project.dir.path().to_path_buf();

    project.env.spec_root = project.dir.path().join("model");
    assert_eq!(guide(&project.view()).spec_directory, "model/");
    project.env.spec_root = project.dir.path().join("docs").join("spec");
    assert_eq!(guide(&project.view()).spec_directory, "docs/spec/");
    project.env.spec_root = project.dir.path().to_path_buf();
    assert_eq!(guide(&project.view()).spec_directory, "./");

    // A view without a root has no relative directory.
    project.env.spec_root = project.dir.path().join("model");
    let recorded = RecordedCoverage::over(&project.graph, &project.env);
    let rootless = ProjectView::new(&project.graph, &project.env, None, &recorded);
    assert_eq!(guide(&rootless).spec_directory, "./");
}

#[specforge_test(
    behavior = "compute_inference_guide",
    verify = "the guide lists each loaded extension, every declared kind in declaration order, and the entities of every written kind"
)]
fn the_guide_lists_extensions_kinds_and_entities() {
    let project = project_of(
        "behavior zeta \"Z\" {\n}\nbehavior alpha \"A\" {\n}\nwidget w \"W\" {\n}\n",
        vec![
            extension("@t/soft", &[("behavior", None), ("event", None)]),
            extension("@t/more", &[("type", None)]),
        ],
    );
    let view = project.view();
    let guide = guide(&view);

    assert_eq!(guide.extensions, ["@t/soft", "@t/more"]);
    let kinds: Vec<&str> = guide.kinds.iter().map(|k| k.keyword).collect();
    assert_eq!(kinds, ["behavior", "event", "type"]);
    // `widget` is no declared kind (E024's), and still counted.
    assert_eq!(
        guide.entities_by_kind.into_iter().collect::<Vec<_>>(),
        [("behavior", 2), ("widget", 1)]
    );
    assert_eq!(guide.kinds[0].existing, ["alpha", "zeta"]);
    let json = kind_guide(&view, "behavior").unwrap().to_json();
    assert_eq!(json["existing_entity_ids"], json!(["alpha", "zeta"]));
}

#[specforge_test(
    behavior = "read_views_over_the_project_view",
    verify = "an argument naming an undeclared kind is refused with unknown_kind naming the closest declared kind"
)]
fn kind_guide_refuses_an_undeclared_kind() {
    let project = project_of("", vec![extension("@t/soft", &[("behavior", None)])]);
    let error = kind_guide(&project.view(), "behaviour").unwrap_err();
    assert_eq!(error.code, "unknown_kind");
    assert_eq!(
        error.suggestion.as_deref(),
        Some("did you mean 'behavior'?")
    );
}
