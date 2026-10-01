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

#[specforge_test(
    behavior = "validate_module_family_field",
    verify = "module with family=core passes"
)]
#[specforge_test(
    behavior = "validate_module_family_field",
    verify = "module with family=integration passes"
)]
#[specforge_test(
    behavior = "validate_module_family_field",
    verify = "module with family=advisory passes"
)]
#[specforge_test(
    behavior = "validate_module_family_field",
    verify = "module with non-standard family produces I062 listing the standard families"
)]
#[specforge_test(
    behavior = "validate_module_family_field",
    verify = "module with no family produces no I062"
)]
fn i062_reports_non_standard_module_families() {
    let diags = product_diagnostics(
        r#"
module m_core "Core" {
  family core
}
module m_integration "Integration" {
  family integration
}
module m_advisory "Advisory" {
  family advisory
}
module m_custom "Custom" {
  family plugins
}
module m_none "None" {
  description "no family"
}
"#,
    );
    for quiet in ["m_core", "m_integration", "m_advisory", "m_none"] {
        assert_quiet(&diags, "I062", quiet);
    }
    assert_fires(&diags, "I062", "m_custom");
    assert_eq!(
        reported(&diags, "I062", "m_custom")[0],
        "module 'm_custom' has non-standard family 'plugins' — standard families: core, platform, extension, integration, advisory"
    );
}

#[specforge_test(
    behavior = "validate_milestone_target_date_format",
    verify = "milestone with valid YYYY-MM-DD target_date passes"
)]
#[specforge_test(
    behavior = "validate_milestone_target_date_format",
    verify = "milestone with invalid target_date format produces I053"
)]
#[specforge_test(
    behavior = "validate_milestone_target_date_format",
    verify = "milestone with no target_date produces no I053"
)]
fn i053_reports_milestone_target_dates_not_yyyy_mm_dd() {
    let diags = product_diagnostics(
        r#"
milestone dated "Dated" {
  target_date "2026-06-30"
}
milestone free_form "Free form" {
  target_date "Q3 2026"
}
milestone undated "Undated" {
  status planned
}
"#,
    );
    assert_quiet(&diags, "I053", "dated");
    assert_fires(&diags, "I053", "free_form");
    assert_quiet(&diags, "I053", "undated");
    assert!(reported(&diags, "I053", "free_form")[0].contains("target_date 'Q3 2026'"));
}

#[specforge_test(
    behavior = "validate_deliverable_version_format",
    verify = "deliverable with valid semver version passes"
)]
#[specforge_test(
    behavior = "validate_deliverable_version_format",
    verify = "deliverable with invalid version format produces I061"
)]
#[specforge_test(
    behavior = "validate_deliverable_version_format",
    verify = "deliverable with no version produces no I061"
)]
#[specforge_test(
    behavior = "validate_deliverable_version_format",
    verify = "deliverable with pre-release tag passes"
)]
#[specforge_test(
    behavior = "validate_deliverable_version_format",
    verify = "deliverable with build metadata passes"
)]
#[specforge_test(
    behavior = "validate_deliverable_version_format",
    verify = "deliverable with version 'v1.0' produces I061"
)]
fn i061_reports_deliverable_versions_that_are_not_semver() {
    let diags = product_diagnostics(
        r#"
deliverable plain "Plain" {
  artifact_type cli
  version "1.2.3"
}
deliverable prerelease "Pre-release" {
  artifact_type cli
  version "1.0.0-alpha.1"
}
deliverable build "Build" {
  artifact_type cli
  version "1.0.0-beta+exp.sha.5114f85"
}
deliverable latest "Latest" {
  artifact_type cli
  version "latest"
}
deliverable v_prefixed "V prefixed" {
  artifact_type cli
  version "v1.0"
}
deliverable unversioned "Unversioned" {
  artifact_type cli
}
"#,
    );
    for quiet in ["plain", "prerelease", "build", "unversioned"] {
        assert_quiet(&diags, "I061", quiet);
    }
    assert_fires(&diags, "I061", "latest");
    assert_fires(&diags, "I061", "v_prefixed");
    assert!(reported(&diags, "I061", "v_prefixed")[0].contains("version 'v1.0'"));
}

#[specforge_test(
    behavior = "validate_tag_format",
    verify = "lowercase hyphen-separated tag passes"
)]
#[specforge_test(
    behavior = "validate_tag_format",
    verify = "uppercase tag produces I068"
)]
#[specforge_test(
    behavior = "validate_tag_format",
    verify = "tag with spaces produces I068"
)]
#[specforge_test(
    behavior = "validate_tag_format",
    verify = "tag with underscores produces I068"
)]
#[specforge_test(
    behavior = "validate_tag_format",
    verify = "tag with special characters produces I068"
)]
#[specforge_test(
    behavior = "validate_tag_format",
    verify = "single-character tag produces I068"
)]
#[specforge_test(
    behavior = "validate_tag_format",
    verify = "tag exceeding 50 chars produces I068"
)]
#[specforge_test(
    behavior = "validate_tag_format",
    verify = "empty string in tags array is silently ignored"
)]
fn i068_reports_tags_that_are_not_lowercase_hyphenated() {
    let long = "a".repeat(51);
    let fifty = "b".repeat(50);
    let spec = format!(
        r#"
term good "Good" {{
  definition "d"
  tags ["core", "mvp-2", "{fifty}"]
}}
term upper "Upper" {{
  definition "d"
  tags ["Core"]
}}
term spaced "Spaced" {{
  definition "d"
  tags ["my tag"]
}}
term underscored "Underscored" {{
  definition "d"
  tags ["my_tag"]
}}
term special "Special" {{
  definition "d"
  tags ["release!"]
}}
term single "Single" {{
  definition "d"
  tags ["a"]
}}
term long "Long" {{
  definition "d"
  tags ["{long}"]
}}
term with_empty "With empty" {{
  definition "d"
  tags ["core", "", "mvp"]
}}
"#
    );
    let diags = product_diagnostics(&spec);
    assert_quiet(&diags, "I068", "good");
    assert_quiet(&diags, "I068", "with_empty");
    for bad in [
        "upper",
        "spaced",
        "underscored",
        "special",
        "single",
        "long",
    ] {
        assert_fires(&diags, "I068", bad);
    }
    assert!(reported(&diags, "I068", "underscored")[0].ends_with("tags [my_tag]"));
}

#[specforge_test(
    behavior = "validate_tag_format",
    verify = "tags on every product kind are checked"
)]
fn i068_checks_tags_on_every_product_kind() {
    let diags = product_diagnostics(
        r#"
feature f1 "F" {
  problem "p"
  tags ["Bad_Tag"]
}
journey j1 "J" {
  flow ["step"]
  tags ["Bad_Tag"]
}
deliverable d1 "D" {
  artifact_type cli
  tags ["Bad_Tag"]
}
milestone ms1 "MS" {
  tags ["Bad_Tag"]
}
module mod1 "Mod" {
  tags ["Bad_Tag"]
}
term t1 "T" {
  definition "d"
  tags ["Bad_Tag"]
}
persona p1 "P" {
  description "d"
  tags ["Bad_Tag"]
}
channel c1 "C" {
  description "d"
  tags ["Bad_Tag"]
}
release r1 "R" {
  version "1.0.0"
  tags ["Bad_Tag"]
}
"#,
    );
    for id in ["f1", "j1", "d1", "ms1", "mod1", "t1", "p1", "c1", "r1"] {
        assert_fires(&diags, "I068", id);
    }
    assert!(reported(&diags, "I068", "r1")[0].starts_with("release 'r1' has a tag"));
}

#[specforge_test(
    behavior = "validate_journey_flow_non_empty",
    verify = "journey with empty flow produces I050"
)]
#[specforge_test(
    behavior = "validate_journey_flow_non_empty",
    verify = "journey with flow steps suppresses I050"
)]
#[specforge_test(
    behavior = "validate_journey_flow_non_empty",
    verify = "journey without a flow field produces E006 and no I050"
)]
fn i050_reports_journeys_with_an_empty_flow() {
    let diags = product_diagnostics(
        r#"
journey empty_flow "Empty" {
  flow []
}
journey with_steps "Steps" {
  flow ["open the app", "log in"]
}
journey no_flow "No flow" {
  description "d"
}
"#,
    );
    assert_fires(&diags, "I050", "empty_flow");
    assert_quiet(&diags, "I050", "with_steps");
    assert_quiet(&diags, "I050", "no_flow");
    assert_fires(&diags, "E006", "no_flow");
    assert_eq!(
        reported(&diags, "I050", "empty_flow")[0],
        "journey 'empty_flow' has an empty flow"
    );
}

#[specforge_test(
    behavior = "detect_term_see_also_non_term_refs",
    verify = "term see_also referencing another term produces no E022"
)]
#[specforge_test(
    behavior = "detect_term_see_also_non_term_refs",
    verify = "term see_also referencing a module produces E022"
)]
#[specforge_test(
    behavior = "detect_term_see_also_non_term_refs",
    verify = "term see_also referencing a feature produces E022"
)]
#[specforge_test(
    behavior = "detect_term_see_also_non_term_refs",
    verify = "term with empty see_also produces no E022"
)]
fn term_see_also_to_another_kind_is_e022() {
    let diags = product_diagnostics(
        r#"
feature f1 "F" {
  problem "p"
}
module mod1 "M" {
  family core
}
term other "Other" {
  definition "d"
}
term to_term "To term" {
  definition "d"
  see_also [other]
}
term to_module "To module" {
  definition "d"
  see_also [mod1]
}
term to_feature "To feature" {
  definition "d"
  see_also [f1]
}
term empty "Empty" {
  definition "d"
  see_also []
}
"#,
    );
    let e022_on = |term: &str| {
        diags
            .iter()
            .filter(|(c, m)| c == "E022" && m.contains(&format!("of term '{term}'")))
            .count()
    };
    assert_eq!(e022_on("to_term"), 0, "{diags:?}");
    assert_eq!(e022_on("to_module"), 1, "{diags:?}");
    assert_eq!(e022_on("to_feature"), 1, "{diags:?}");
    assert_eq!(e022_on("empty"), 0, "{diags:?}");
    assert!(diags.iter().all(|(c, _)| c != "I056"));
}
