//! The @specforge/product extension's declarative validation rules, run end
//! to end: each test compiles a small project with the real product Wasm
//! extension loaded and checks which entities a rule reports.

use specforge_test_macros::test as specforge_test;
use std::fs;
use tempfile::TempDir;

/// Compile `spec` in a project that enables only @specforge/product and
/// return its diagnostics as (code, message) pairs.
fn product_diagnostics(spec: &str) -> Vec<(String, String)> {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/product"]}"#,
    )
    .unwrap();
    fs::write(dir.path().join("main.spec"), spec).unwrap();
    let runtime = specforge_component::project_runtime(dir.path());
    let ctx =
        specforge_project::CompiledProject::compile(dir.path(), Some(&runtime)).into_context();
    ctx.diagnostics
        .iter()
        .map(|d| (d.code.clone(), d.message.clone()))
        .collect()
}

/// Messages of the `code` diagnostics that name entity `id`.
fn reported<'a>(diags: &'a [(String, String)], code: &str, id: &str) -> Vec<&'a str> {
    let quoted = format!("'{id}'");
    diags
        .iter()
        .filter(|(c, m)| c == code && m.contains(&quoted))
        .map(|(_, m)| m.as_str())
        .collect()
}

/// Assert `code` reports `id` exactly once.
fn assert_fires(diags: &[(String, String)], code: &str, id: &str) {
    let hits = reported(diags, code, id);
    assert_eq!(
        hits.len(),
        1,
        "expected one {code} on '{id}', got {hits:?}; all: {diags:?}"
    );
}

/// Assert `code` doesn't report `id`.
fn assert_quiet(diags: &[(String, String)], code: &str, id: &str) {
    let hits = reported(diags, code, id);
    assert!(
        hits.is_empty(),
        "expected no {code} on '{id}', got {hits:?}"
    );
}

const DELIVERABLE_PEERS: &str = r#"
journey j1 "J" {
  flow ["step"]
}
module m1 "M" {
  family core
}
"#;

fn deliverables_spec() -> String {
    format!(
        r#"{DELIVERABLE_PEERS}
deliverable d_bare "Bare" {{
  artifact_type cli
}}
deliverable d_empty "Empty" {{
  artifact_type cli
  journeys []
  modules []
}}
deliverable d_full "Full" {{
  artifact_type cli
  journeys [j1]
  modules [m1]
}}
"#
    )
}

#[specforge_test(
    behavior = "detect_deliverables_with_no_journeys",
    verify = "deliverable with no journeys produces W043"
)]
#[specforge_test(
    behavior = "detect_deliverables_with_no_journeys",
    verify = "deliverable with an empty journeys list produces W043"
)]
#[specforge_test(
    behavior = "detect_deliverables_with_no_journeys",
    verify = "deliverable with journeys suppresses W043"
)]
fn w043_reports_deliverables_without_journeys() {
    let diags = product_diagnostics(&deliverables_spec());
    assert_fires(&diags, "W043", "d_bare");
    assert_fires(&diags, "W043", "d_empty");
    assert_quiet(&diags, "W043", "d_full");
    assert!(reported(&diags, "W043", "d_bare")[0].contains("supports no journeys"));
}

#[specforge_test(
    behavior = "detect_deliverables_with_no_modules",
    verify = "deliverable with no modules produces W046"
)]
#[specforge_test(
    behavior = "detect_deliverables_with_no_modules",
    verify = "deliverable with an empty modules list produces W046"
)]
#[specforge_test(
    behavior = "detect_deliverables_with_no_modules",
    verify = "deliverable with modules suppresses W046"
)]
fn w046_reports_deliverables_without_modules() {
    let diags = product_diagnostics(&deliverables_spec());
    assert_fires(&diags, "W046", "d_bare");
    assert_fires(&diags, "W046", "d_empty");
    assert_quiet(&diags, "W046", "d_full");
    assert!(reported(&diags, "W046", "d_bare")[0].contains("contains no modules"));
}

#[specforge_test(
    behavior = "pe_validate_deliverable_completeness",
    verify = "deliverable with journeys and modules passes both checks"
)]
#[specforge_test(
    behavior = "pe_validate_deliverable_completeness",
    verify = "deliverable with no journeys produces W043"
)]
#[specforge_test(
    behavior = "pe_validate_deliverable_completeness",
    verify = "deliverable with no modules produces W046"
)]
fn deliverable_completeness_checks_journeys_and_modules_separately() {
    let spec = format!(
        r#"{DELIVERABLE_PEERS}
deliverable only_modules "Modules only" {{
  artifact_type cli
  modules [m1]
}}
deliverable only_journeys "Journeys only" {{
  artifact_type cli
  journeys [j1]
}}
deliverable both "Both" {{
  artifact_type cli
  journeys [j1]
  modules [m1]
}}
"#
    );
    let diags = product_diagnostics(&spec);
    assert_fires(&diags, "W043", "only_modules");
    assert_quiet(&diags, "W046", "only_modules");
    assert_fires(&diags, "W046", "only_journeys");
    assert_quiet(&diags, "W043", "only_journeys");
    assert_quiet(&diags, "W043", "both");
    assert_quiet(&diags, "W046", "both");
}

#[specforge_test(
    behavior = "detect_empty_milestones",
    verify = "milestone with no features and no modules produces W049"
)]
#[specforge_test(
    behavior = "detect_empty_milestones",
    verify = "milestone with features suppresses W049"
)]
#[specforge_test(
    behavior = "detect_empty_milestones",
    verify = "milestone with modules but no features produces W049"
)]
fn w049_reads_only_the_features_field() {
    let diags = product_diagnostics(
        r#"
feature f1 "F" {
  problem "p"
}
module m1 "M" {
  family core
}
milestone bare "Bare" {
  status planned
}
milestone with_features "With features" {
  features [f1]
}
milestone modules_only "Modules only" {
  modules [m1]
}
"#,
    );
    assert_fires(&diags, "W049", "bare");
    assert_quiet(&diags, "W049", "with_features");
    assert_fires(&diags, "W049", "modules_only");
    assert_eq!(
        reported(&diags, "W049", "bare")[0],
        "milestone 'bare' has no features — it may be empty"
    );
}

#[specforge_test(
    behavior = "detect_modules_with_no_features",
    verify = "module with features suppresses I067"
)]
#[specforge_test(
    behavior = "detect_modules_with_no_features",
    verify = "module with empty features produces I067"
)]
#[specforge_test(
    behavior = "detect_modules_with_no_features",
    verify = "module with no features field produces I067"
)]
fn i067_reports_modules_without_features() {
    let diags = product_diagnostics(
        r#"
feature f1 "F" {
  problem "p"
}
module with_features "With" {
  features [f1]
}
module empty_features "Empty" {
  features []
}
module bare "Bare" {
  family core
}
"#,
    );
    assert_quiet(&diags, "I067", "with_features");
    assert_fires(&diags, "I067", "empty_features");
    assert_fires(&diags, "I067", "bare");
    assert!(reported(&diags, "I067", "bare")[0].contains("contains no features"));
}

#[specforge_test(
    behavior = "detect_features_with_no_acceptance",
    verify = "feature with no acceptance criteria produces I048"
)]
#[specforge_test(
    behavior = "detect_features_with_no_acceptance",
    verify = "feature with an empty acceptance list produces I048"
)]
#[specforge_test(
    behavior = "detect_features_with_no_acceptance",
    verify = "feature with acceptance criteria suppresses I048"
)]
fn i048_reports_features_without_acceptance() {
    let diags = product_diagnostics(
        r#"
feature bare "Bare" {
  problem "p"
}
feature empty_acceptance "Empty" {
  problem "p"
  acceptance []
}
feature accepted "Accepted" {
  problem "p"
  acceptance ["the user can log in"]
}
"#,
    );
    assert_fires(&diags, "I048", "bare");
    assert_fires(&diags, "I048", "empty_acceptance");
    assert_quiet(&diags, "I048", "accepted");
    assert!(reported(&diags, "I048", "bare")[0].contains("has no acceptance criteria"));
}
