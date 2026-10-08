//! The dependency commands: feature ordering, the critical path, module depth and coupling, deliverable dependents.

use super::host::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

// ── dependency graphs ─────────────────────────────────────────────────────

/// Features over `depends_on`, by level:
/// - 0: base (low), solo (medium), loner (no priority)
/// - 1, on base: m_crit (critical), m_high (high), m_none (no priority),
///   m_med (medium), m_low (low)
/// - 2: top, on m_high and m_none
fn layered() -> G {
    let mut g = G::default()
        .node("base", "feature", json!({"priority": "low"}))
        .node("solo", "feature", json!({"priority": "medium"}))
        .n("loner", "feature")
        .node("m_crit", "feature", json!({"priority": "critical"}))
        .node("m_high", "feature", json!({"priority": "high"}))
        .n("m_none", "feature")
        .node("m_med", "feature", json!({"priority": "medium"}))
        .node("m_low", "feature", json!({"priority": "low"}))
        .n("top", "feature")
        .edge("top", "m_high", "depends_on")
        .edge("top", "m_none", "depends_on");
    for mid in ["m_crit", "m_high", "m_none", "m_med", "m_low"] {
        g = g.edge(mid, "base", "depends_on");
    }
    g
}

/// The order `layered` sorts to.
const LAYERED: [&str; 9] = [
    "loner", "solo", "base", "m_crit", "m_high", "m_med", "m_none", "m_low", "top",
];

/// [`layered`] plus x <-> y and z on x.
fn layered_with_a_cycle() -> G {
    layered()
        .n("x", "feature")
        .n("y", "feature")
        .n("z", "feature")
        .edge("x", "y", "depends_on")
        .edge("y", "x", "depends_on")
        .edge("z", "x", "depends_on")
}

fn reversed(mut g: G) -> G {
    g.edges.reverse();
    g.nodes.reverse();
    g
}

#[specforge_test(
    behavior = "surface_feature_ordering",
    verify = "feature-ordering returns FeatureOrderingPayload JSON"
)]
fn feature_ordering_answers_its_payload() {
    let fo = json_of("feature_ordering", json!({}), &layered());
    assert_eq!(
        fo,
        json!({"sorted_features": LAYERED, "has_cycles": false, "cycle_members": []})
    );
    let human = human_of("feature_ordering", json!({}), &layered_with_a_cycle());
    assert!(human.starts_with("  1. loner\n  2. solo\n"), "{human}");
    assert!(human.contains(". x  (cycle)\n"), "{human}");
    assert!(human.contains(". z\n"), "{human}");
    assert!(human.ends_with("Dependency cycle: x, y\n"), "{human}");
}

#[specforge_test(
    behavior = "surface_feature_ordering",
    verify = "cycles present in output does not cause exit code 1"
)]
fn a_feature_cycle_is_reported_with_exit_zero() {
    let out = run_in(
        &runtime(),
        "feature_ordering",
        json!({}),
        &layered_with_a_cycle(),
        "json",
    );
    assert_eq!(out.exit, 0, "{}", out.stderr);
    assert_eq!(out.json()["has_cycles"], true);
}

#[specforge_test(
    behavior = "surface_feature_ordering",
    verify = "empty feature graph returns empty sorted list"
)]
fn feature_ordering_of_no_features_is_empty() {
    let fo = json_of("feature_ordering", json!({}), &G::default());
    assert_eq!(fo["sorted_features"], json!([]));
    assert_eq!(
        human_of("feature_ordering", json!({}), &G::default()),
        "No features.\n"
    );
}

#[specforge_test(
    behavior = "pe_query_feature_ordering",
    verify = "acyclic feature graph returns topological sort"
)]
fn an_acyclic_feature_graph_sorts_dependencies_first() {
    let fo = json_of("feature_ordering", json!({}), &layered());
    let order: Vec<&str> = fo["sorted_features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(order, LAYERED);
    let at = |id: &str| order.iter().position(|f| *f == id).unwrap();
    for mid in ["m_crit", "m_high", "m_none", "m_med", "m_low"] {
        assert!(at("base") < at(mid), "{mid}");
    }
    assert!(at("m_high") < at("top") && at("m_none") < at("top"));
}

#[specforge_test(
    behavior = "pe_query_feature_ordering",
    verify = "features at same level sorted by priority descending"
)]
fn a_levels_features_are_most_important_first() {
    let fo = json_of("feature_ordering", json!({}), &layered());
    assert_eq!(
        fo["sorted_features"].as_array().unwrap()[3..8],
        json!(["m_crit", "m_high", "m_med", "m_none", "m_low"])
            .as_array()
            .unwrap()[..]
    );
}

#[specforge_test(
    behavior = "pe_query_feature_ordering",
    verify = "features without priority default to medium"
)]
fn a_feature_without_a_priority_ranks_as_medium() {
    // m_none sits with m_med, by id; loner (none) sorts before base (low)
    // and beside solo (medium), by id.
    let fo = json_of("feature_ordering", json!({}), &layered());
    let order = fo["sorted_features"].as_array().unwrap();
    assert_eq!(
        order[..3],
        json!(["loner", "solo", "base"]).as_array().unwrap()[..]
    );
    assert_eq!(
        order[5..7],
        json!(["m_med", "m_none"]).as_array().unwrap()[..]
    );
}

#[specforge_test(
    behavior = "pe_query_feature_ordering",
    verify = "cyclic feature graph returns has_cycles=true with cycle members"
)]
fn a_feature_cycle_names_its_members_once() {
    // z depends on the cycle without being on it: it is listed last, not
    // as a member.
    let g = layered_with_a_cycle().edge("x", "y", "depends_on");
    let fo = json_of("feature_ordering", json!({}), &g);
    assert_eq!(fo["has_cycles"], true);
    assert_eq!(fo["cycle_members"], json!(["x", "y"]));
    let order = fo["sorted_features"].as_array().unwrap();
    assert_eq!(order.len(), 12, "every feature once: {fo}");
    assert_eq!(order[9..], json!(["x", "y", "z"]).as_array().unwrap()[..]);
}

#[specforge_test(
    behavior = "pe_query_feature_ordering",
    verify = "features with no dependencies return stable ordering"
)]
fn independent_features_of_one_priority_sort_by_id() {
    let g = G::default()
        .n("delta", "feature")
        .n("alpha", "feature")
        .n("charlie", "feature")
        .n("bravo", "feature");
    let fo = json_of("feature_ordering", json!({}), &g);
    assert_eq!(
        fo["sorted_features"],
        json!(["alpha", "bravo", "charlie", "delta"])
    );
}

#[specforge_test(
    behavior = "pe_query_feature_ordering",
    verify = "empty feature graph returns empty sorted list and has_cycles=false"
)]
fn an_empty_feature_graph_orders_nothing_and_has_no_cycle() {
    // Other kinds' depends_on are not features'.
    let g = G::default()
        .n("a", "module")
        .n("b", "module")
        .edge("a", "b", "depends_on")
        .edge("b", "a", "depends_on");
    let fo = json_of("feature_ordering", json!({}), &g);
    assert_eq!(
        fo,
        json!({"sorted_features": [], "has_cycles": false, "cycle_members": []})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_ordering",
    verify = "feature ordering is deterministic across repeated queries"
)]
fn feature_ordering_is_the_same_every_time_and_in_any_order() {
    let first = json_of("feature_ordering", json!({}), &layered_with_a_cycle());
    assert_eq!(
        json_of("feature_ordering", json!({}), &layered_with_a_cycle()),
        first
    );
    assert_eq!(
        json_of(
            "feature_ordering",
            json!({}),
            &reversed(layered_with_a_cycle())
        ),
        first
    );
}

#[specforge_test(
    behavior = "product_query_correctness",
    verify = "feature ordering produces valid topological sort"
)]
fn every_feature_comes_after_the_features_it_depends_on() {
    for g in [layered(), plan(), shipping()] {
        let fo = json_of("feature_ordering", json!({}), &g);
        let order: Vec<&str> = fo["sorted_features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f.as_str().unwrap())
            .collect();
        let at = |id: &str| order.iter().position(|f| *f == id);
        for edge in g.edges.iter().filter(|e| e["label"] == "depends_on") {
            let (from, to) = (
                edge["source"].as_str().unwrap(),
                edge["target"].as_str().unwrap(),
            );
            if let (Some(dependent), Some(dependency)) = (at(from), at(to)) {
                assert!(dependency < dependent, "{to} before {from}: {order:?}");
            }
        }
    }
}

/// Milestones over `depends_on` (each on the one before):
/// - m0 (completed) <- m1 (in_progress, 2026-11-01) <- m2 (blocked,
///   2026-12-01) <- m3 (no date)
/// - n1 (2026-10-15) <- n2 (2026-10-30)
fn schedule() -> G {
    G::default()
        .node(
            "m0",
            "milestone",
            json!({"status": "completed", "target_date": "2026-10-01"}),
        )
        .node(
            "m1",
            "milestone",
            json!({"status": "in_progress", "target_date": "2026-11-01"}),
        )
        .node(
            "m2",
            "milestone",
            json!({"status": "blocked", "target_date": "2026-12-01"}),
        )
        .n("m3", "milestone")
        .node("n1", "milestone", json!({"target_date": "2026-10-15"}))
        .node("n2", "milestone", json!({"target_date": "2026-10-30"}))
        .edge("m1", "m0", "depends_on")
        .edge("m2", "m1", "depends_on")
        .edge("m3", "m2", "depends_on")
        .edge("n2", "n1", "depends_on")
}

fn path_ids(cp: &Value) -> Vec<String> {
    cp["critical_path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["entity_id"].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test(
    behavior = "surface_critical_path",
    verify = "product:critical-path returns CriticalPathPayload"
)]
fn critical_path_answers_its_payload() {
    let cp = json_of("critical_path", json!({}), &schedule());
    assert_eq!(
        cp,
        json!({
            "critical_path": [
                {"entity_id": "m1", "entity_kind": "milestone", "target_date": "2026-11-01",
                    "status": "in_progress", "slack_days": 0},
                {"entity_id": "m2", "entity_kind": "milestone", "target_date": "2026-12-01",
                    "status": "blocked", "slack_days": 0},
                {"entity_id": "m3", "entity_kind": "milestone", "target_date": null,
                    "status": null, "slack_days": null},
            ],
            "path_length": 3,
            "earliest_completion": "2026-11-01",
            "latest_completion": null,
            "bottleneck_ids": ["m1", "m2"],
        })
    );
    let human = human_of("critical_path", json!({}), &schedule());
    assert_eq!(
        human,
        "milestone  target_date  status       slack\n\
         m1         2026-11-01   in_progress  0\n\
         m2         2026-12-01   blocked      0\n\
         m3         -            -            -\n\
         Bottlenecks: m1, m2\n"
    );
}

#[specforge_test(
    behavior = "surface_critical_path",
    verify = "empty graph returns empty path"
)]
fn critical_path_of_no_milestones_is_empty() {
    let cp = json_of("critical_path", json!({}), &G::default());
    assert_eq!(
        cp,
        json!({"critical_path": [], "path_length": 0, "earliest_completion": null,
            "latest_completion": null, "bottleneck_ids": []})
    );
}

#[specforge_test(
    behavior = "surface_critical_path",
    verify = "cycles return empty path with diagnostic message"
)]
fn a_milestone_cycle_gives_an_empty_path_and_says_why() {
    let g = schedule().edge("m1", "m3", "depends_on");
    let out = run_in(&runtime(), "critical_path", json!({}), &g, "json");
    assert_eq!(out.exit, 0);
    let cp = out.json();
    assert_eq!(cp["critical_path"], json!([]));
    assert_eq!(
        cp["message"],
        "milestones depend on each other in a cycle (m1, m2, m3): no critical path"
    );
    assert_eq!(
        human_of("critical_path", json!({}), &g),
        "No critical path: milestones depend on each other in a cycle (m1, m2, m3): no critical path\n"
    );
}

#[specforge_test(
    behavior = "pe_query_critical_path",
    verify = "linear chain of 3 milestones returns all 3 as critical path"
)]
fn a_chain_of_three_open_milestones_is_the_path() {
    let g = G::default()
        .n("a", "milestone")
        .n("b", "milestone")
        .n("c", "milestone")
        .edge("c", "b", "depends_on")
        .edge("b", "a", "depends_on");
    let cp = json_of("critical_path", json!({}), &g);
    assert_eq!(path_ids(&cp), ["a", "b", "c"]);
    assert_eq!(cp["path_length"], 3);
}

#[specforge_test(
    behavior = "pe_query_critical_path",
    verify = "parallel chains return the longer one"
)]
fn of_two_chains_the_longer_is_the_path() {
    // m1-m2-m3 (three open) beats n1-n2.
    let cp = json_of("critical_path", json!({}), &schedule());
    assert_eq!(path_ids(&cp), ["m1", "m2", "m3"]);
}

#[specforge_test(
    behavior = "pe_query_critical_path",
    verify = "completed milestones are excluded from critical path"
)]
fn a_completed_milestone_is_off_the_path() {
    // m0 is completed: the chain stops at m1.
    let cp = json_of("critical_path", json!({}), &schedule());
    assert!(!path_ids(&cp).contains(&"m0".to_string()));
    // Completing m1 and m2 leaves the n chain longest.
    let mut g = schedule();
    for node in &mut g.nodes {
        if matches!(node["id"].as_str(), Some("m1" | "m2")) {
            node["fields"]["status"] = json!("completed");
        }
    }
    let cp = json_of("critical_path", json!({}), &g);
    assert_eq!(path_ids(&cp), ["n1", "n2"]);
    assert_eq!(
        (
            cp["earliest_completion"].clone(),
            cp["latest_completion"].clone()
        ),
        (json!("2026-10-15"), json!("2026-10-30"))
    );
}

#[specforge_test(
    behavior = "pe_query_critical_path",
    verify = "milestones without target_date still appear on path"
)]
fn a_milestone_without_a_date_is_on_the_path_without_dates() {
    let cp = json_of("critical_path", json!({}), &schedule());
    let m3 = &cp["critical_path"][2];
    assert_eq!(m3["entity_id"], "m3");
    assert_eq!(
        (m3["target_date"].clone(), m3["slack_days"].clone()),
        (Value::Null, Value::Null)
    );
}

#[specforge_test(
    behavior = "pe_query_critical_path",
    verify = "graph with cycles returns empty path"
)]
fn a_cycle_among_milestones_leaves_no_path() {
    // Even a cycle off the longest chain: E015 fires, so no path is given.
    let g = schedule().edge("n1", "n2", "depends_on");
    let cp = json_of("critical_path", json!({}), &g);
    assert_eq!(
        (cp["critical_path"].clone(), cp["path_length"].clone()),
        (json!([]), json!(0))
    );
    assert!(cp["message"].as_str().unwrap().contains("(n1, n2)"), "{cp}");
}

#[specforge_test(
    behavior = "product_new_query_correctness",
    verify = "critical path is the longest incomplete chain"
)]
fn no_chain_of_open_milestones_is_longer_than_the_critical_path() {
    // A diamond and a tail: e <- {f, g} <- h <- i; j alone; equal branches
    // are taken by id.
    let g = G::default()
        .n("e", "milestone")
        .n("f", "milestone")
        .n("g", "milestone")
        .n("h", "milestone")
        .n("i", "milestone")
        .n("j", "milestone")
        .edge("f", "e", "depends_on")
        .edge("g", "e", "depends_on")
        .edge("h", "f", "depends_on")
        .edge("h", "g", "depends_on")
        .edge("i", "h", "depends_on");
    let cp = json_of("critical_path", json!({}), &g);
    assert_eq!(path_ids(&cp), ["e", "f", "h", "i"]);
    assert_eq!(
        json_of("critical_path", json!({}), &reversed(g)),
        cp,
        "declaration order does not change it"
    );
}

/// Modules over `depends_on`: api -> util -> core; web -> api, util;
/// cli -> api, util; iso alone.
fn layers() -> G {
    G::default()
        .n("core", "module")
        .n("util", "module")
        .n("api", "module")
        .n("web", "module")
        .n("cli", "module")
        .n("iso", "module")
        .edge("util", "core", "depends_on")
        .edge("api", "util", "depends_on")
        .edge("web", "api", "depends_on")
        .edge("web", "util", "depends_on")
        .edge("cli", "api", "depends_on")
        .edge("cli", "util", "depends_on")
}

#[specforge_test(
    behavior = "surface_module_dependency_depth",
    verify = "product:module-depth returns ModuleDependencyDepthPayload"
)]
fn module_depth_answers_its_payload() {
    let md = json_of("module_depth", json!({"module": "web"}), &layers());
    assert_eq!(
        md,
        json!({"module_id": "web", "depth": 3, "longest_chain": ["web", "api", "util", "core"]})
    );
    assert_eq!(
        human_of("module_depth", json!({"module": "web"}), &layers()),
        "Module: web (depth 3)\nChain: web -> api -> util -> core\n"
    );
}

#[specforge_test(
    behavior = "surface_module_dependency_depth",
    verify = "missing module returns error with suggestion"
)]
fn module_depth_of_a_mistyped_module_suggests_the_nearest() {
    let error = not_found_suggesting("module_depth", json!({"module": "utl"}), &layers());
    assert_eq!(error["suggestion"], "util");
}

#[specforge_test(
    behavior = "pe_query_module_dependency_depth",
    verify = "module with no dependencies returns depth=0"
)]
fn a_module_without_dependencies_is_at_depth_zero() {
    let md = json_of("module_depth", json!({"module": "core"}), &layers());
    assert_eq!(
        (md["depth"].clone(), md["longest_chain"].clone()),
        (json!(0), json!(["core"]))
    );
}

#[specforge_test(
    behavior = "pe_query_module_dependency_depth",
    verify = "module depending on two layers returns depth=2"
)]
fn a_module_two_layers_up_is_at_depth_two() {
    let md = json_of("module_depth", json!({"module": "api"}), &layers());
    assert_eq!(
        (md["depth"].clone(), md["longest_chain"].clone()),
        (json!(2), json!(["api", "util", "core"]))
    );
}

#[specforge_test(
    behavior = "pe_query_module_dependency_depth",
    verify = "module in a cycle returns depth=-1"
)]
fn a_module_on_a_cycle_is_at_depth_minus_one() {
    let g = layers().edge("core", "api", "depends_on");
    let md = json_of("module_depth", json!({"module": "util"}), &g);
    assert_eq!(
        md,
        json!({"module_id": "util", "depth": -1, "longest_chain": ["api", "core", "util"]})
    );
    assert_eq!(
        human_of("module_depth", json!({"module": "util"}), &g),
        "Module: util (depth -1: on or behind a dependency cycle)\nCycle: api, core, util\n"
    );
}

#[specforge_test(
    behavior = "pe_query_module_dependency_depth",
    verify = "longest_chain includes all modules in the longest path"
)]
fn the_chain_follows_the_longest_path_not_the_shortest() {
    // cli reaches util directly and through api: the chain goes through api.
    let md = json_of("module_depth", json!({"module": "cli"}), &layers());
    assert_eq!(md["longest_chain"], json!(["cli", "api", "util", "core"]));
    assert_eq!(md["depth"], 3);
}

#[specforge_test(
    behavior = "pe_query_module_dependency_depth",
    verify = "non-existent module returns ENTITY_NOT_FOUND with suggestion"
)]
fn module_depth_of_no_module_is_not_found() {
    let error = not_found_suggesting("module_depth", json!({"module": "cor"}), &layers());
    assert_eq!(
        (error["entity_id"].clone(), error["suggestion"].clone()),
        (json!("cor"), json!("core"))
    );
    // A feature's id is not a module's.
    let g = layers().n("core2", "feature");
    let error = not_found_suggesting("module_depth", json!({"module": "core2"}), &g);
    assert_eq!(error["message"], "module 'core2' not found");
}

#[specforge_test(
    behavior = "pe_query_module_dependency_depth",
    verify = "result is deterministic across repeated queries"
)]
fn module_depth_is_the_same_every_time_and_in_any_order() {
    for module in ["web", "cli", "iso"] {
        let args = json!({ "module": module });
        let first = json_of("module_depth", args.clone(), &layers());
        assert_eq!(json_of("module_depth", args.clone(), &layers()), first);
        assert_eq!(json_of("module_depth", args, &reversed(layers())), first);
    }
}

#[specforge_test(
    behavior = "surface_module_coupling",
    verify = "product:module-coupling returns ModuleCouplingPayload"
)]
fn module_coupling_answers_its_payload() {
    let mc = json_of("module_coupling", json!({}), &layers());
    assert_eq!(
        mc,
        json!({
            "modules": [
                {"module_id": "util", "fan_in": 3, "fan_out": 1, "coupling": 4},
                {"module_id": "api", "fan_in": 2, "fan_out": 1, "coupling": 3},
                {"module_id": "cli", "fan_in": 0, "fan_out": 2, "coupling": 2},
                {"module_id": "web", "fan_in": 0, "fan_out": 2, "coupling": 2},
                {"module_id": "core", "fan_in": 1, "fan_out": 0, "coupling": 1},
                {"module_id": "iso", "fan_in": 0, "fan_out": 0, "coupling": 0},
            ],
            "avg_fan_in": 1.0, "avg_fan_out": 1.0, "most_coupled_id": "util",
            "total_modules": 6, "total": 6, "offset": 0, "limit": 100, "has_more": false,
        })
    );
    // Paged by offset and limit, with the total.
    let page = json_of(
        "module_coupling",
        json!({"offset": 2, "limit": 2}),
        &layers(),
    );
    assert_eq!(
        page["modules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["module_id"].clone())
            .collect::<Vec<_>>(),
        [json!("cli"), json!("web")]
    );
    assert_eq!(
        (
            page["total"].clone(),
            page["has_more"].clone(),
            page["most_coupled_id"].clone()
        ),
        (json!(6), json!(true), json!("util"))
    );
    let error = run_in(
        &runtime(),
        "module_coupling",
        json!({"offset": -1}),
        &layers(),
        "json",
    );
    assert_eq!(error.exit, 2);
    assert_eq!(error.error()["code"], "INVALID_INPUT");
    let human = human_of("module_coupling", json!({}), &layers());
    assert!(
        human.starts_with("module  fan_in  fan_out  coupling\nutil    3       1        4\n"),
        "{human}"
    );
}

#[specforge_test(
    behavior = "surface_module_coupling",
    verify = "empty graph returns empty modules array"
)]
fn module_coupling_of_no_modules_is_empty() {
    let mc = json_of("module_coupling", json!({}), &G::default());
    assert_eq!(
        mc,
        json!({"modules": [], "avg_fan_in": null, "avg_fan_out": null, "most_coupled_id": null,
            "total_modules": 0, "total": 0, "offset": 0, "limit": 100, "has_more": false})
    );
}

fn coupling_of(mc: &Value, module: &str) -> Value {
    mc["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["module_id"] == module)
        .unwrap()
        .clone()
}

#[specforge_test(
    behavior = "pe_query_module_coupling",
    verify = "module with 3 dependents and 1 dependency has fan_in=3 fan_out=1 coupling=4"
)]
fn a_module_with_three_dependents_and_one_dependency_couples_four() {
    let mc = json_of("module_coupling", json!({}), &layers());
    assert_eq!(
        coupling_of(&mc, "util"),
        json!({"module_id": "util", "fan_in": 3, "fan_out": 1, "coupling": 4})
    );
}

#[specforge_test(
    behavior = "pe_query_module_coupling",
    verify = "isolated module has fan_in=0 fan_out=0 coupling=0"
)]
fn an_isolated_module_couples_nothing() {
    // A feature's depends_on on it does not couple it.
    let g = layers().n("f1", "feature").edge("f1", "iso", "depends_on");
    let mc = json_of("module_coupling", json!({}), &g);
    assert_eq!(
        coupling_of(&mc, "iso"),
        json!({"module_id": "iso", "fan_in": 0, "fan_out": 0, "coupling": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_module_coupling",
    verify = "empty module graph returns empty modules array"
)]
fn a_graph_without_modules_has_no_coupling() {
    let mc = json_of("module_coupling", json!({}), &layered());
    assert_eq!(
        (mc["modules"].clone(), mc["total_modules"].clone()),
        (json!([]), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_module_coupling",
    verify = "most_coupled_id identifies the highest-coupling module"
)]
fn the_most_coupled_module_is_named_first_by_id_on_ties() {
    let mc = json_of("module_coupling", json!({}), &layers());
    assert_eq!(mc["most_coupled_id"], "util");
    // Two modules depending on each other tie: the first by id.
    let g = G::default()
        .n("b", "module")
        .n("a", "module")
        .edge("a", "b", "depends_on")
        .edge("b", "a", "depends_on");
    assert_eq!(
        json_of("module_coupling", json!({}), &g)["most_coupled_id"],
        "a"
    );
}

#[specforge_test(
    behavior = "pe_query_module_coupling",
    verify = "averages are computed correctly across all modules"
)]
fn the_averages_are_over_every_module() {
    // Six dependencies among six modules; a duplicate reference counts once.
    let g = layers()
        .edge("web", "api", "depends_on")
        .n("iso2", "module");
    let mc = json_of("module_coupling", json!({}), &g);
    let (fan_in, fan_out) = (
        mc["avg_fan_in"].as_f64().unwrap(),
        mc["avg_fan_out"].as_f64().unwrap(),
    );
    assert!((fan_in - 6.0 / 7.0).abs() < 1e-9, "{fan_in}");
    assert!((fan_out - 6.0 / 7.0).abs() < 1e-9, "{fan_out}");
}

#[specforge_test(
    behavior = "pe_query_module_coupling",
    verify = "result is deterministic across repeated queries"
)]
fn module_coupling_is_the_same_every_time_and_in_any_order() {
    let first = json_of("module_coupling", json!({}), &layers());
    assert_eq!(json_of("module_coupling", json!({}), &layers()), first);
    assert_eq!(
        json_of("module_coupling", json!({}), &reversed(layers())),
        first
    );
}

/// Deliverables over `depends_on`: d2 and d3 on d1, d4 on d2.
fn deliverable_chain() -> G {
    G::default()
        .n("d1", "deliverable")
        .n("d2", "deliverable")
        .n("d3", "deliverable")
        .n("d4", "deliverable")
        .n("f1", "feature")
        .edge("d2", "d1", "depends_on")
        .edge("d3", "d1", "depends_on")
        .edge("d4", "d2", "depends_on")
        // A feature's depends_on on a deliverable is not a dependent.
        .edge("f1", "d1", "depends_on")
}

#[specforge_test(
    behavior = "surface_deliverable_dependents",
    verify = "deliverable-dependents returns DeliverableDependentPayload JSON"
)]
fn deliverable_dependents_answers_its_payload() {
    let dd = json_of(
        "deliverable_dependents",
        json!({"deliverable": "d1"}),
        &deliverable_chain(),
    );
    assert_eq!(
        dd,
        json!({"deliverable_id": "d1", "dependents": ["d2", "d3"], "count": 2})
    );
}

#[specforge_test(
    behavior = "surface_deliverable_dependents",
    verify = "missing deliverable ID returns error with suggestion"
)]
fn deliverable_dependents_of_a_mistyped_deliverable_suggests_the_nearest() {
    let error = not_found_suggesting(
        "deliverable_dependents",
        json!({"deliverable": "d5"}),
        &deliverable_chain(),
    );
    assert_eq!(error["suggestion"], "d1");
}

#[specforge_test(
    behavior = "pe_query_deliverable_dependents",
    verify = "deliverable with dependent returns that dependent"
)]
fn a_deliverable_depended_on_once_has_that_dependent() {
    let dd = json_of(
        "deliverable_dependents",
        json!({"deliverable": "d2"}),
        &deliverable_chain(),
    );
    assert_eq!(dd["dependents"], json!(["d4"]));
}

#[specforge_test(
    behavior = "pe_query_deliverable_dependents",
    verify = "deliverable with multiple dependents returns all sorted by ID"
)]
fn a_deliverables_dependents_are_sorted_by_id() {
    let dd = json_of(
        "deliverable_dependents",
        json!({"deliverable": "d1"}),
        &reversed(deliverable_chain()),
    );
    assert_eq!(dd["dependents"], json!(["d2", "d3"]));
}

#[specforge_test(
    behavior = "pe_query_deliverable_dependents",
    verify = "deliverable with no dependents returns empty list"
)]
fn a_deliverable_nothing_depends_on_has_no_dependents() {
    let dd = json_of(
        "deliverable_dependents",
        json!({"deliverable": "d4"}),
        &deliverable_chain(),
    );
    assert_eq!(
        dd,
        json!({"deliverable_id": "d4", "dependents": [], "count": 0})
    );
}

#[specforge_test(
    behavior = "product_query_correctness",
    verify = "reverse queries return exact inverse of forward queries"
)]
fn dependents_are_exactly_the_depends_on_references_reversed() {
    let g = deliverable_chain();
    for d in ["d1", "d2", "d3", "d4"] {
        let dd = json_of("deliverable_dependents", json!({ "deliverable": d }), &g);
        let mut expected: Vec<&str> = g
            .edges
            .iter()
            .filter(|e| e["label"] == "depends_on" && e["target"] == d)
            .map(|e| e["source"].as_str().unwrap())
            .filter(|s| s.starts_with('d'))
            .collect();
        expected.sort_unstable();
        assert_eq!(dd["dependents"], json!(expected), "{d}");
    }
    let g = layered();
    for f in ["base", "m_high", "top"] {
        let fd = json_of("feature_dependents", json!({ "feature": f }), &g);
        let mut expected: Vec<&str> = g
            .edges
            .iter()
            .filter(|e| e["label"] == "depends_on" && e["target"] == f)
            .map(|e| e["source"].as_str().unwrap())
            .collect();
        expected.sort_unstable();
        assert_eq!(fd["dependents"], json!(expected), "{f}");
    }
}
