//! Black-box tests: the `specforge` binary on small projects that load the built `@gsd/core` (ADR gsd/0001 D5, D7).

mod ab_protocol;
mod ab_seeds;
mod agents;
mod answer_members;
// The extension's own UTC clock text, to build times relative to now.
#[path = "../../src/time.rs"]
#[allow(dead_code, reason = "only utc is used here")]
mod clock;
// shells out through `bash` and links with `std::os::unix`
#[cfg(unix)]
mod assets_shell;
mod baseline;
// shells out through `bash` and links with `std::os::unix`
#[path = "../common/asset_code.rs"]
mod asset_code;
#[cfg(unix)]
mod capability_points;
mod cmd_attest;
mod cmd_automated_guard;
mod cmd_branch_check;
mod cmd_digest;
mod cmd_edge_probe;
mod cmd_execute;
mod cmd_gate;
mod cmd_learning;
mod cmd_milestone;
mod cmd_phase;
mod cmd_quick;
mod cmd_refactor;
mod cmd_reports;
mod cmd_review;
mod cmd_smart_entry;
mod cmd_state;
mod cmd_surface;
mod cmd_verify;
mod cmd_windows;
mod cmd_workstreams;
mod collect;
#[path = "../common/command_word.rs"]
mod command_word;
mod commands;
mod commands_support;
mod context_proxy;
mod contexts;
mod decision_coverage;
mod discuss;
mod e502_probe;
mod engine_tools;
mod execute;
mod execute_rules;
mod execute_status;
#[path = "../common/files.rs"]
mod files;
mod gates;
mod governance;
mod harness;
mod hooks_community;
mod hooks_config;
mod hooks_context;
mod hooks_guards;
mod hooks_hostile;
mod hooks_isolation;
mod hooks_scan;
mod hooks_spec_root;
mod hooks_statusline;
mod hooks_support;
mod hooks_workflow;
#[path = "../common/host_options.rs"]
mod host_options;
mod install_agents;
mod install_hosts;
mod install_skills;
mod invariants;
mod judged;
mod judged_flow;
mod judged_refusals;
mod ladder;
#[path = "../common/ledger.rs"]
mod ledger;
#[path = "../common/markdown_writes.rs"]
mod markdown_writes;
#[path = "../common/node.rs"]
mod node;
mod node_test;
mod parity_discuss;
mod parity_gate_cutover;
mod parity_ledger;
mod parity_plans;
mod parity_roadmap;
mod parity_scenario;
mod parity_state;
mod parity_thread;
mod parity_verify;
mod plan;
mod plan_rules;
mod planned_paths;
#[path = "../common/planning_names.rs"]
mod planning_names;
mod product;
mod progress;
mod prompts_audit;
mod prompts_autonomous;
mod prompts_configure;
mod prompts_discuss;
mod prompts_execute;
mod prompts_gates;
mod prompts_git;
mod prompts_learning;
mod prompts_navigate;
mod prompts_ns;
mod prompts_onboard;
mod prompts_plan;
mod prompts_project;
mod prompts_quick;
mod prompts_review;
mod prompts_session;
mod prompts_ship;
mod prompts_spec;
mod prompts_support;
mod prompts_verify;
mod prompts_workspace;
mod proof;
mod queries;
mod quick_context;
mod reference;
mod render_prompts;
mod rendered_lines;
mod roadmap;
#[path = "../common/roster.rs"]
mod roster;
mod route;
mod rules_silent;
mod scan;
mod settings;
mod source_items;
#[path = "../common/specforge_only.rs"]
mod specforge_only;
#[path = "../common/tool_names.rs"]
mod tool_names;
mod verify_paths;
mod verify_rules;
mod verify_status;
mod verify_verdicts;

use harness::{Project, project};

#[test]
fn an_empty_project_loads_gsd_core_and_checks_clean() {
    let p = project("empty");
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    let listed = p.run(&["extensions"]);
    assert!(
        listed.all().contains("@gsd/core") && listed.all().contains(env!("CARGO_PKG_VERSION")),
        "extensions: {}",
        listed.all()
    );
}

#[test]
fn explain_names_a_gsd_code() {
    let p = project("empty");
    let explained = p.run(&["explain", "gsd/E101"]);
    assert_eq!(explained.code, 0, "explain: {}", explained.all());
    assert!(
        explained
            .all()
            .contains("Requirement not mapped to a phase"),
        "explain: {}",
        explained.all()
    );
}

#[test]
fn a_project_with_requirements_checks_clean() {
    let p = project("roadmap");
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    assert!(
        check.all().contains("0 errors, 0 warnings"),
        "check: {}",
        check.all()
    );
}

#[test]
fn a_requirement_with_an_unknown_tier_is_refused() {
    let p = project("roadmap");
    p.write(
        "spec/bad.spec",
        "requirement TASK-09 \"Bad\" {\n  text \"x\"\n  area \"Tasks\"\n  tier v3\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(
        check.all().contains("E061") && check.all().contains("v1, v2, out_of_scope"),
        "check: {}",
        check.all()
    );
}

#[test]
fn a_requirement_without_its_area_is_refused() {
    let p = project("roadmap");
    p.write(
        "spec/bad.spec",
        "requirement TASK-09 \"Bad\" {\n  text \"x\"\n  tier v2\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(
        check.all().contains("TASK-09") && check.all().contains("'area'"),
        "check: {}",
        check.all()
    );
}

fn node_ids(query: &harness::Run) -> Vec<String> {
    let graph: serde_json::Value = serde_json::from_str(&query.stdout)
        .unwrap_or_else(|e| panic!("query is JSON ({e}): {}", query.all()));
    graph["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|n| n["id"].as_str().expect("an id").to_string())
        .collect()
}

#[test]
fn a_phase_is_found_by_its_full_path_and_by_a_unique_short_form() {
    let p = project("roadmap");
    for name in ["v1.0/01.1", "01.1"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(query.code, 0, "query {name}: {}", query.all());
        assert_eq!(node_ids(&query), ["v1.0/01.1"], "query {name}");
    }
}

#[test]
fn two_milestones_may_each_have_a_phase_01() {
    let p = project("roadmap");
    p.write(
        "spec/v2.0.spec",
        "milestone v2.0 \"Taskboard 2.0\" {\n  name \"Taskboard 2.0\"\n}\n\nphase 01 \"Sharing\" {\n  milestone v2.0\n  name \"Sharing\"\n  slug \"sharing\"\n  goal \"Boards are shared\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    for name in ["v1.0/01", "v2.0/01"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(node_ids(&query), [name], "query {name}: {}", query.all());
    }
}

#[test]
fn an_ambiguous_short_form_names_each_candidate() {
    let p = project("roadmap");
    p.write(
        "spec/v2.0.spec",
        "milestone v2.0 \"Taskboard 2.0\" {\n  name \"Taskboard 2.0\"\n}\n\nphase 01 \"Sharing\" {\n  milestone v2.0\n  name \"Sharing\"\n  slug \"sharing\"\n  goal \"Boards are shared\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    let query = p.run(&["query", "01", "--depth", "0", "--format", "brief"]);
    assert_ne!(
        query.code,
        0,
        "an ambiguous short form is refused: {}",
        query.all()
    );
    assert!(
        query.all().contains("v1.0/01") && query.all().contains("v2.0/01"),
        "candidates: {}",
        query.all()
    );
}

#[test]
fn a_milestone_in_a_workstream_is_named_by_its_workstream() {
    let p = project("roadmap");
    // `v1.0` of the workstream is a second milestone beside the project's own `v1.0`.
    p.write(
        "spec/learning.spec",
        "workstream auth-rework \"Auth rework\" {\n  purpose \"Replace the session store\"\n}\n\nmilestone v1.0 \"Auth rework 1.0\" {\n  name \"Auth rework 1.0\"\n  workstream auth-rework\n}\n\nphase 01 \"Store\" {\n  milestone auth-rework/v1.0\n  name \"Store\"\n  slug \"store\"\n  goal \"The store is replaced\"\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    for name in ["v1.0", "auth-rework/v1.0", "auth-rework/v1.0/01", "v1.0/01"] {
        let query = p.run(&["query", name, "--depth", "0", "--format", "brief"]);
        assert_eq!(node_ids(&query), [name], "query {name}: {}", query.all());
    }
}

#[test]
fn a_lesson_is_named_within_its_retrospective() {
    let p = project("roadmap");
    p.write(
        "spec/learning.spec",
        "retrospective v1.0-RETROSPECTIVE \"Taskboard 1.0 retrospective\" {\n  milestone v1.0\n  built [\"Add and complete tasks\"]\n}\n\nlesson L1 \"Plan the reload first\" {\n  retrospective v1.0-RETROSPECTIVE\n  text \"Persistence decides the data model\"\n}\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    let query = p.run(&[
        "query",
        "v1.0/v1.0-RETROSPECTIVE/L1",
        "--depth",
        "0",
        "--format",
        "brief",
    ]);
    assert_eq!(
        node_ids(&query),
        ["v1.0/v1.0-RETROSPECTIVE/L1"],
        "{}",
        query.all()
    );
}

/// The ref a requirement's `source` names in these tests.
const BOARD_EXPORT: &str = "ref gh.issue:42 \"Board export\"\n";

/// The `roadmap` project with `spec/quality.spec` holding `blocks`, and [`BOARD_EXPORT`] declared when they name an
/// issue (a ref nothing names is W012).
fn with_quality(blocks: &str) -> Project {
    let p = project("roadmap");
    if blocks.contains("gh.issue:") {
        p.write("spec/refs.spec", BOARD_EXPORT);
    }
    p.write("spec/quality.spec", blocks);
    p
}

/// `check`'s exit code and its whole output.
fn checked(p: &Project) -> (i32, String) {
    let check = p.run(&["check"]);
    (check.code, check.all())
}

fn assert_clean(p: &Project) {
    let (code, all) = checked(p);
    assert_eq!(code, 0, "check: {all}");
    assert!(all.contains("0 errors, 0 warnings"), "check: {all}");
}

const PERF_01: &str = "requirement PERF-01 \"Incremental diagnostics\" {
  text     \"A file change shows its diagnostics within 100 ms for 500 spec files\"
  area     \"Compiler\"
  tier     v2
  category performance
  priority must
  metric   { name \"file change to diagnostics\" threshold 100 unit \"ms\" direction at_most }
  source   [\"Alice Martin\", gh.issue:42]
  tags     [\"latency\"]
}
";

#[test]
fn a_requirement_takes_its_quality_fields() {
    let p = with_quality(&format!(
        "{PERF_01}\nrequirement PERF-02 \"Drift\" {{\n  text     \"A clock drifts little\"\n  area     \
         \"Compiler\"\n  tier     v2\n  metric   {{ name \"drift\" threshold 0.25 unit \"s\" direction at_most }}\n}}\n"
    ));
    assert_clean(&p);
    let perf = p.fields_of("PERF-01");
    assert_eq!(perf["category"], "performance", "{perf:#}");
    assert_eq!(perf["priority"], "must", "{perf:#}");
    assert_eq!(
        perf["metric"]["threshold"],
        serde_json::json!(100),
        "{perf:#}"
    );
    assert!(perf["metric"]["threshold"].is_number(), "{perf:#}");
    assert_eq!(perf["metric"]["unit"], "ms", "{perf:#}");
    assert_eq!(perf["metric"]["direction"], "at_most", "{perf:#}");
    assert_eq!(
        perf["source"],
        serde_json::json!(["Alice Martin", "gh.issue:42"]),
        "{perf:#}"
    );
    assert_eq!(perf["tags"], serde_json::json!(["latency"]), "{perf:#}");
    let drift = p.fields_of("PERF-02");
    assert_eq!(
        drift["metric"]["threshold"],
        serde_json::json!(0.25),
        "{drift:#}"
    );
}

#[test]
fn a_requirement_with_an_unknown_category_priority_or_direction_is_refused() {
    for (from, to, named) in [
        (
            "category performance",
            "category latency",
            "functional, performance, compatibility, usability, reliability, security, maintainability, \
             portability, safety, regulatory",
        ),
        ("priority must", "priority high", "must, should, could"),
        (
            "direction at_most",
            "direction below",
            "at_most, at_least, exactly",
        ),
    ] {
        let p = with_quality(&PERF_01.replace(from, to));
        let (code, all) = checked(&p);
        assert_eq!(code, 1, "{to}: {all}");
        assert!(all.contains("E061") && all.contains(named), "{to}: {all}");
    }
}

#[test]
fn a_requirement_written_with_category_as_its_heading_is_refused() {
    let p = with_quality(
        "requirement TASK-09 \"Old\" {\n  text     \"x\"\n  area     \"Tasks\"\n  category \"Tasks\"\n  tier     \
         v2\n}\n",
    );
    let (code, all) = checked(&p);
    assert_eq!(code, 1, "{all}");
    assert!(all.contains("E061") && all.contains("functional"), "{all}");
}

#[test]
fn a_requirement_source_names_only_a_declared_ref() {
    let p = with_quality(&PERF_01.replace("gh.issue:42]", "gh.issue:43]"));
    let (code, all) = checked(&p);
    assert_eq!(code, 1, "{all}");
    assert!(all.contains("E003") && all.contains("gh.issue:43"), "{all}");
}

#[test]
fn a_requirement_metric_without_its_direction_is_refused() {
    let p = with_quality(&PERF_01.replace(" direction at_most", ""));
    let (code, all) = checked(&p);
    assert_eq!(code, 1, "{all}");
    assert!(all.contains("direction"), "{all}");
}

#[test]
fn a_requirement_metric_that_counts_needs_no_unit() {
    let p = with_quality(
        "requirement GATE-01 \"No blocker\" {\n  text   \"A release ships with no open blocker\"\n  area   \
         \"Release\"\n  tier   v2\n  metric { name \"open blockers\" threshold 0 direction at_most }\n}\n",
    );
    assert_clean(&p);
    let gate = p.fields_of("GATE-01");
    assert_eq!(
        gate["metric"]["threshold"],
        serde_json::json!(0),
        "{gate:#}"
    );
    assert!(gate["metric"].get("unit").is_none(), "{gate:#}");
}

#[test]
fn a_requirement_takes_its_quality_fields_as_flags() {
    let p = with_quality("");
    p.write("spec/refs.spec", BOARD_EXPORT);
    let args = |dry_run: bool| {
        let mut args = vec![
            "new",
            "requirement",
            "P95-01",
            "Fast enough",
            "--in",
            "spec/quality.spec",
            "--text",
            "A request answers fast",
            "--area",
            "Serving",
            "--tier",
            "v2",
            "--metric",
            r#"{"name": "p95", "threshold": 0.5, "unit": "s", "direction": "at_most"}"#,
            "--source",
            "Alice Martin",
            "--source",
            "gh.issue:42",
            "--category",
            "security",
            "--priority",
            "should",
        ];
        if dry_run {
            args.push("--dry-run");
        }
        args
    };
    let dry = p.run(&args(true));
    assert_eq!(dry.code, 0, "{}", dry.all());
    assert!(dry.stdout.contains("+requirement P95-01"), "{}", dry.all());
    assert!(!p.read("spec/quality.spec").contains("P95-01"));
    let written = p.run(&args(false));
    assert_eq!(written.code, 0, "{}", written.all());
    assert_clean(&p);
    let fields = p.fields_of("P95-01");
    assert_eq!(
        fields["metric"]["threshold"],
        serde_json::json!(0.5),
        "{fields:#}"
    );
    assert_eq!(
        fields["source"],
        serde_json::json!(["Alice Martin", "gh.issue:42"]),
        "{fields:#}"
    );
    assert_eq!(
        (&fields["category"], &fields["priority"]),
        (&"security".into(), &"should".into()),
        "{fields:#}"
    );
}
