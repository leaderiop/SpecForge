//! The `inferred` lint profile and the inference sessions, through the
//! operations the surfaces call (ADR 0015, "Management operations").

use specforge_ops::check::{CheckOptions, check};
use specforge_project::LintProfile;
use specforge_registry::RegistryBuild;
use specforge_test::prelude::*;

use crate::view_support::Project;

/// A project with an `inferred` check over the manifest `manifest` (or no
/// manifest) and the source file `src/tiny.rs` of two lines.
fn project(manifest: Option<&str>) -> Project {
    let project = Project::new("behavior a \"A\" {\n}\n", RegistryBuild::default());
    let root = project.dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/tiny.rs"), "pub fn a() {}\npub fn b() {}\n").unwrap();
    if let Some(manifest) = manifest {
        std::fs::write(root.join("specforge-infer.json"), manifest).unwrap();
    }
    project
}

/// A manifest indexing `src/tiny.rs` as the source of `entities` entities.
fn indexing(entities: usize) -> String {
    let produced: Vec<String> = (0..entities).map(|i| format!("e{i}")).collect();
    serde_json::json!({
        "version": 1,
        "source_roots": ["src"],
        "source_index": [{
            "path": "src/tiny.rs",
            "content_hash": "h",
            "entities_produced": produced,
            "analyzed_at": "2026-10-01T00:00:00Z",
        }],
    })
    .to_string()
}

/// What `specforge check --lint inferred` reports for `project`, beyond
/// nothing.
fn inferred(project: &Project) -> Vec<String> {
    let options = CheckOptions {
        lint_profiles: vec![LintProfile::Inferred],
        ..Default::default()
    };
    let outcome = check(&project.view(), Vec::new(), &options).unwrap();
    outcome.reported.iter().map(|d| d.code.clone()).collect()
}

/// A lint profile adds nothing when its input is absent: no
/// specforge-infer.json, no I200/I202.
#[test]
fn the_inferred_lint_adds_nothing_without_a_manifest() {
    assert!(inferred(&project(None)).is_empty());
}

/// Pins plan 06 R4: an unusable manifest lints as nothing. T5 flips it.
#[test]
fn the_inferred_lint_ignores_an_unusable_manifest() {
    assert!(inferred(&project(Some("{ nope"))).is_empty());
}

/// The density threshold is the config the compile read, not a second
/// read of `specforge.json`: 2 entities from 2 lines are over the default
/// threshold and not over 1.0, and 3 are over it.
#[specforge_test(
    behavior = "detect_high_inference_density",
    verify = "I202 threshold is configurable via specforge.json"
)]
fn the_density_threshold_is_the_compiled_config() {
    let mut dense = project(Some(&indexing(2)));
    assert!(
        inferred(&dense).contains(&"I202".to_string()),
        "over the default threshold"
    );

    dense.env.config.inference.density_threshold = Some(1.0);
    assert!(!inferred(&dense).contains(&"I202".to_string()));

    let mut denser = project(Some(&indexing(3)));
    denser.env.config.inference.density_threshold = Some(1.0);
    assert!(inferred(&denser).contains(&"I202".to_string()));
}
