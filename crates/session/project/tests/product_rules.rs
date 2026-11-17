//! The @specforge/product extension's declarative validation rules, run end
//! to end: each test compiles a small project with the real product Wasm
//! extension loaded and checks which entities a rule reports.

use specforge_project::testing::TestProject;
use specforge_test_macros::test as specforge_test;

/// Compile `spec` in a project that enables only @specforge/product and
/// return its diagnostics as (code, message) pairs.
fn product_diagnostics(spec: &str) -> Vec<(String, String)> {
    TestProject::new()
        .enabling(&["@specforge/product"])
        .file("main.spec", spec)
        .compile()
        .diagnostics()
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

#[specforge_test(requirement = "FEATURE-097")]
fn w043_reports_deliverables_without_journeys() {
    let diags = product_diagnostics(&deliverables_spec());
    assert_fires(&diags, "W043", "d_bare");
    assert_fires(&diags, "W043", "d_empty");
    assert_quiet(&diags, "W043", "d_full");
    assert!(reported(&diags, "W043", "d_bare")[0].contains("supports no journeys"));
}

#[specforge_test(requirement = "FEATURE-097")]
fn w046_reports_deliverables_without_modules() {
    let diags = product_diagnostics(&deliverables_spec());
    assert_fires(&diags, "W046", "d_bare");
    assert_fires(&diags, "W046", "d_empty");
    assert_quiet(&diags, "W046", "d_full");
    assert!(reported(&diags, "W046", "d_bare")[0].contains("contains no modules"));
}

#[specforge_test(requirement = "FEATURE-097")]
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

#[specforge_test(requirement = "FEATURE-097")]
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

#[specforge_test(requirement = "FEATURE-097")]
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

#[specforge_test(requirement = "FEATURE-097")]
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

#[specforge_test(requirement = "FEATURE-097")]
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

#[specforge_test(requirement = "FEATURE-097")]
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

#[specforge_test(requirement = "FEATURE-097")]
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

#[specforge_test(requirement = "FEATURE-097")]
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

#[specforge_test(requirement = "FEATURE-097")]
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

#[specforge_test(requirement = "FEATURE-097")]
fn i057_reports_blocked_milestones_without_dependencies() {
    let diags = product_diagnostics(
        r#"
milestone base "Base" {
  status planned
}
milestone blocked_with_deps "Blocked, with deps" {
  status blocked
  blockers ["vendor"]
  depends_on [base]
}
milestone blocked_bare "Blocked, bare" {
  status blocked
  blockers ["vendor"]
}
milestone blocked_empty "Blocked, empty deps" {
  status blocked
  blockers ["vendor"]
  depends_on []
}
milestone running "Running" {
  status in_progress
}
"#,
    );
    assert_quiet(&diags, "I057", "blocked_with_deps");
    assert_fires(&diags, "I057", "blocked_bare");
    assert_fires(&diags, "I057", "blocked_empty");
    assert_quiet(&diags, "I057", "running");
    assert_eq!(
        reported(&diags, "I057", "blocked_bare")[0],
        "milestone 'blocked_bare' has status 'blocked' but no depends_on"
    );
}

#[specforge_test(requirement = "FEATURE-097")]
fn i060_reads_blockers_not_reason() {
    let diags = product_diagnostics(
        r#"
milestone with_reason "Reason only" {
  status blocked
  reason "waiting on legal"
}
milestone with_blockers "Blockers only" {
  status blocked
  blockers ["legal review"]
}
"#,
    );
    assert_fires(&diags, "I060", "with_reason");
    assert_quiet(&diags, "I060", "with_blockers");
}

#[specforge_test(requirement = "FEATURE-097")]
fn i054_reports_journeys_without_a_persona() {
    let diags = product_diagnostics(
        r#"
persona dev "Developer" {
  description "d"
}
journey with_persona "With" {
  flow ["step"]
  persona dev
}
journey without_persona "Without" {
  flow ["step"]
}
"#,
    );
    assert_quiet(&diags, "I054", "with_persona");
    assert_fires(&diags, "I054", "without_persona");
    assert_eq!(
        reported(&diags, "I054", "without_persona")[0],
        "journey 'without_persona' has no persona"
    );
}

#[specforge_test(requirement = "FEATURE-097")]
fn i055_reports_journeys_without_channels() {
    let diags = product_diagnostics(
        r#"
channel web "Web" {
  description "d"
}
journey with_channels "With" {
  flow ["step"]
  channels [web]
}
journey without_channels "Without" {
  flow ["step"]
}
journey empty_channels "Empty" {
  flow ["step"]
  channels []
}
"#,
    );
    assert_quiet(&diags, "I055", "with_channels");
    assert_fires(&diags, "I055", "without_channels");
    assert_fires(&diags, "I055", "empty_channels");
    assert!(reported(&diags, "I055", "without_channels")[0].contains("uses no channels"));
}

const RELEASE_PEERS: &str = r#"
deliverable d1 "D" {
  artifact_type cli
}
milestone ms1 "MS" {
  status completed
}
"#;

#[specforge_test(requirement = "FEATURE-094")]
fn i082_i083_report_releases_without_deliverables_or_milestones() {
    let spec = format!(
        r#"{RELEASE_PEERS}
release full "Full" {{
  version "1.0.0"
  deliverables [d1]
  milestones [ms1]
}}
release empty "Empty" {{
  version "1.1.0"
  deliverables []
}}
release bare "Bare" {{
  version "1.2.0"
}}
"#
    );
    let diags = product_diagnostics(&spec);
    assert_quiet(&diags, "I082", "full");
    assert_fires(&diags, "I082", "empty");
    assert_fires(&diags, "I082", "bare");
    assert_quiet(&diags, "I083", "full");
    assert_fires(&diags, "I083", "bare");
    assert!(reported(&diags, "I082", "bare")[0].contains("includes no deliverables"));
    assert!(reported(&diags, "I083", "bare")[0].contains("completes no milestones"));
}

#[specforge_test(requirement = "FEATURE-094")]
fn i089_reports_recalled_releases_without_a_reason() {
    let diags = product_diagnostics(
        r#"
release explained "Explained" {
  version "1.0.0"
  status recalled
  reason "data loss bug"
}
release unexplained "Unexplained" {
  version "1.0.1"
  status recalled
}
release shipped "Shipped" {
  version "1.0.2"
  status released
}
"#,
    );
    assert_quiet(&diags, "I089", "explained");
    assert_fires(&diags, "I089", "unexplained");
    assert_quiet(&diags, "I089", "shipped");
}

#[specforge_test(requirement = "FEATURE-097")]
#[specforge_test(requirement = "FEATURE-094")]
#[specforge_test(requirement = "FEATURE-096")]
fn a_product_date_that_is_no_calendar_day_is_e061() {
    let diags = product_diagnostics(
        r#"
milestone dated "Dated" {
  target_date 2026-06-30
  start_date  2026-01-15
}
milestone prose "Prose" {
  target_date "Q3 2026"
  start_date  "Jan 15"
}
milestone undated "Undated" {
  status planned
}
release iso "ISO" {
  version      "1.0.0"
  release_date 2026-06-01
}
release prose_release "Prose release" {
  version      "1.1.0"
  release_date "June 2026"
}
"#,
    );
    assert_quiet(&diags, "E061", "dated");
    assert_quiet(&diags, "E061", "undated");
    assert_quiet(&diags, "E061", "iso");
    let prose = reported(&diags, "E061", "prose");
    assert_eq!(prose.len(), 2, "{prose:?}");
    assert!(
        prose
            .iter()
            .any(|m| m.contains("'target_date'") && m.contains("\"Q3 2026\""))
    );
    assert!(
        prose
            .iter()
            .any(|m| m.contains("'start_date'") && m.contains("\"Jan 15\""))
    );
    assert_fires(&diags, "E061", "prose_release");
    assert!(reported(&diags, "E061", "prose_release")[0].contains("\"June 2026\""));
    for retired in ["I053", "I086", "I087"] {
        assert!(diags.iter().all(|(code, _)| code != retired), "{retired}");
    }
}

#[specforge_test(requirement = "FEATURE-087")]
fn i080_reports_owned_kinds_without_an_owner() {
    let diags = product_diagnostics(
        r#"
feature owned "Owned" {
  problem "p"
  owner "team-a"
}
feature unowned "Unowned" {
  problem "p"
}
milestone ms1 "MS" {
  status planned
}
deliverable d1 "D" {
  artifact_type cli
}
release r1 "R" {
  version "1.0.0"
}
journey j1 "J" {
  flow ["step"]
}
"#,
    );
    assert_quiet(&diags, "I080", "owned");
    for id in ["unowned", "ms1", "d1", "r1"] {
        assert_fires(&diags, "I080", id);
    }
    assert_quiet(&diags, "I080", "j1");
    assert_eq!(
        reported(&diags, "I080", "r1")[0],
        "release 'r1' has no owner"
    );
}

#[specforge_test(requirement = "FEATURE-082")]
fn i081_reports_features_without_effort() {
    let diags = product_diagnostics(
        r#"
feature sized "Sized" {
  problem "p"
  effort m
}
feature unsized "Unsized" {
  problem "p"
}
"#,
    );
    assert_quiet(&diags, "I081", "sized");
    assert_fires(&diags, "I081", "unsized");
    assert!(reported(&diags, "I081", "unsized")[0].contains("has no effort estimate"));
}

// ── lifecycle pass (check phase): W154, I063, I064, I065 ───────────────────

#[specforge_test(requirement = "FEATURE-097")]
fn w154_reports_unfinished_features_of_completed_milestones() {
    let diags = product_diagnostics(
        r#"
feature f_done "Done" {
  status done
}
feature f_gone "Gone" {
  status deprecated
}
feature f_open "Open" {
  status in_progress
}
feature f_bare "Bare" {
}
milestone m_claims "Claims" {
  status completed
  features [f_done, f_open, f_bare]
  exit_criteria ["x"]
}
milestone m_true "True" {
  status completed
  features [f_done, f_gone]
  exit_criteria ["x"]
}
milestone m_open "Open" {
  status in_progress
  features [f_open, f_bare]
}
"#,
    );
    let mut w154: Vec<&str> = diags
        .iter()
        .filter(|(c, _)| c == "W154")
        .map(|(_, m)| m.as_str())
        .collect();
    w154.sort_unstable();
    assert_eq!(
        w154,
        [
            "milestone 'm_claims' is completed but its feature 'f_bare' is proposed",
            "milestone 'm_claims' is completed but its feature 'f_open' is in_progress",
        ],
        "{diags:?}"
    );
    assert_quiet(&diags, "W154", "m_true");
    assert_quiet(&diags, "W154", "m_open");
}

#[specforge_test(requirement = "FEATURE-097")]
fn i063_reports_done_features_depending_on_unfinished_ones() {
    let diags = product_diagnostics(
        r#"
feature base_done "Base" {
  status done
}
feature base_open "Open base" {
  status accepted
}
feature on_done "On done" {
  status done
  depends_on [base_done]
}
feature on_open "On open" {
  status done
  depends_on [base_done, base_open]
}
feature busy "Busy" {
  status in_progress
  depends_on [base_open]
}
"#,
    );
    assert_fires(&diags, "I063", "on_open");
    assert!(
        reported(&diags, "I063", "on_open")[0].contains("'base_open', which is accepted"),
        "{diags:?}"
    );
    assert_quiet(&diags, "I063", "on_done");
    assert_quiet(&diags, "I063", "busy");
    assert_quiet(&diags, "I063", "base_done");
}

#[specforge_test(requirement = "FEATURE-097")]
fn i064_reports_milestones_due_before_their_dependencies() {
    let diags = product_diagnostics(
        r#"
feature f "F" {
}
milestone first "First" {
  target_date "2026-03-01"
  features [f]
}
milestone early "Early" {
  target_date "2026-02-01"
  depends_on [first]
  features [f]
}
milestone later "Later" {
  target_date "2026-04-01"
  depends_on [first]
  features [f]
}
milestone undated "Undated" {
  depends_on [first]
  features [f]
}
milestone on_undated "On undated" {
  target_date "2026-01-01"
  depends_on [undated]
  features [f]
}
"#,
    );
    assert_fires(&diags, "I064", "early");
    assert_quiet(&diags, "I064", "later");
    assert_quiet(&diags, "I064", "undated");
    assert_quiet(&diags, "I064", "on_undated");
}

#[specforge_test(requirement = "RELIABILITY-09")]
#[specforge_test(requirement = "FEATURE-097")]
fn i065_reports_shipped_deliverables_with_incomplete_milestones() {
    let diags = product_diagnostics(&format!(
        r#"{DELIVERABLE_PEERS}
feature f "F" {{
  status done
}}
milestone done_m "Done" {{
  status completed
  features [f]
  exit_criteria ["x"]
}}
milestone open_m "Open" {{
  status in_progress
  features [f]
}}
deliverable shipped_ok "OK" {{
  artifact_type cli
  status shipped
  journeys [j1]
  modules [m1]
  milestones [done_m]
}}
deliverable shipped_early "Early" {{
  artifact_type cli
  status shipped
  journeys [j1]
  modules [m1]
  milestones [done_m, open_m]
}}
deliverable drafted "Draft" {{
  artifact_type cli
  status draft
  journeys [j1]
  modules [m1]
  milestones [open_m]
}}
deliverable shipped_bare "Bare" {{
  artifact_type cli
  status shipped
  journeys [j1]
  modules [m1]
}}
"#
    ));
    assert_fires(&diags, "I065", "shipped_early");
    assert_quiet(&diags, "I065", "shipped_ok");
    assert_quiet(&diags, "I065", "drafted");
    assert_quiet(&diags, "I065", "shipped_bare");
}

/// A product project with the milestone `m1` due in 2000: past due, and still no I058 from the compile.
const OVERDUE_MILESTONE_PROJECT: &str = r#"persona dev "Developer" {
    description "A software developer"
    status active
}

channel cli "CLI" {
    description "Command-line interface"
    status active
}

feature f1 "Core Feature" {
    status proposed
    priority high
}

feature f2 "Secondary Feature" {
    status done
    priority medium
    depends_on [f1]
}

journey j1 "Dev Flow" {
    persona dev
    channels [cli]
    features [f1, f2]
    description "Developer uses CLI"
}

module mod1 "Core Module" {
    features [f1]
}

milestone m1 "Launch" {
    status planned
    target_date "2000-01-01"
    features [f1, f2]
    modules [mod1]
}

deliverable d1 "CLI App" {
    artifact_type cli
    status draft
    journeys [j1]
    modules [mod1]
    milestones [m1]
}

term specification "Specification" {
    definition "A formal description of system behavior"
}

release r1 "Initial Release" {
    version "1.0.0"
    status planned
    deliverables [d1]
    milestones [m1]
}
"#;

#[specforge_test(requirement = "FEATURE-092")]
fn check_reports_no_overdue_milestone() {
    let diagnostics = product_diagnostics(OVERDUE_MILESTONE_PROJECT);
    // Infos are always reported, so an I058 would be among them.
    assert!(
        diagnostics.iter().all(|(code, _)| code != "I058"),
        "{diagnostics:?}"
    );
}
