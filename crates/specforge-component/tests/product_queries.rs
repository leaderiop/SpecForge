//! `@specforge/product`'s commands, run as the host runs them: the builtin
//! component's `cmd__product_*` exports, each given a `CommandInput` over a
//! graph built here, in the format asked for (ADR 0011). What each answers,
//! and how one that cannot answer fails.

use serde_json::{Value, json};
use specforge_component::{ComponentRuntime, builtins};
use specforge_test::prelude::*;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

const PRODUCT: &str = "@specforge/product";

/// A graph built entity by entity, in the graph export's shape.
#[derive(Default)]
struct G {
    nodes: Vec<Value>,
    edges: Vec<Value>,
}

impl G {
    fn node(mut self, id: &str, kind: &str, fields: Value) -> Self {
        self.nodes
            .push(json!({"id": id, "kind": kind, "title": id, "fields": fields}));
        self
    }

    fn n(self, id: &str, kind: &str) -> Self {
        self.node(id, kind, json!({}))
    }

    /// A feature with `status`.
    fn feature(self, id: &str, status: &str) -> Self {
        self.node(id, "feature", json!({"status": status}))
    }

    fn edge(mut self, source: &str, target: &str, label: &str) -> Self {
        self.edges
            .push(json!({"source": source, "target": target, "label": label}));
        self
    }

    fn graph(&self) -> Value {
        json!({"nodes": self.nodes, "edges": self.edges})
    }
}

/// What a command printed: its exit code, stdout and stderr.
struct Out {
    exit: i64,
    stdout: String,
    stderr: String,
}

impl Out {
    /// Its stdout, one JSON object.
    fn json(&self) -> Value {
        assert_eq!(self.exit, 0, "{}", self.stderr);
        assert!(self.stderr.is_empty(), "{}", self.stderr);
        let value: Value = serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {}", self.stdout));
        assert!(value.is_object(), "one root object: {value}");
        value
    }

    /// Its stderr, one error object, and nothing on stdout.
    fn error(&self) -> Value {
        assert_ne!(self.exit, 0);
        assert_eq!(self.stdout, "", "nothing on stdout");
        serde_json::from_str(&self.stderr)
            .unwrap_or_else(|e| panic!("stderr is not JSON ({e}): {}", self.stderr))
    }
}

fn runtime() -> ComponentRuntime {
    let runtime = ComponentRuntime::new();
    builtins::load_builtins(&runtime).unwrap();
    runtime
}

/// Run `cmd__product_<id>` with `args` over `g` in `format`.
fn run_in(runtime: &ComponentRuntime, id: &str, args: Value, g: &G, format: &str) -> Out {
    let input = json!({"args": args, "cwd": "/p", "format": format, "today": "2026-10-03",
        "graph": g.graph()});
    let export = format!("cmd__product_{id}");
    let WasmCallResult::Ok(bytes) =
        runtime.call_export(PRODUCT, &export, input.to_string().as_bytes())
    else {
        panic!("{export} trapped");
    };
    let out: Value = serde_json::from_slice(&bytes).unwrap();
    Out {
        exit: out["exit_code"].as_i64().unwrap(),
        stdout: out["stdout"].as_str().unwrap().to_string(),
        stderr: out["stderr"].as_str().unwrap().to_string(),
    }
}

fn json_of(id: &str, args: Value, g: &G) -> Value {
    run_in(&runtime(), id, args, g, "json").json()
}

fn human_of(id: &str, args: Value, g: &G) -> String {
    let out = run_in(&runtime(), id, args, g, "human");
    assert_eq!(out.exit, 0, "{}", out.stderr);
    out.stdout
}

/// The graph most tests ask about: two milestones, a journey, two personas
/// and a channel over three features.
fn plan() -> G {
    G::default()
        .feature("f1", "done")
        .feature("f2", "in_progress")
        .n("f3", "feature")
        .node(
            "ms1",
            "milestone",
            json!({"status": "active", "features": ["f1", "f2"]}),
        )
        .node("ms2", "milestone", json!({"features": ["f1"]}))
        .n("ms3", "milestone")
        .node("j1", "journey", json!({"persona": "dev"}))
        .node("j2", "journey", json!({"persona": "dev"}))
        .n("j3", "journey")
        .n("dev", "persona")
        .n("ops", "persona")
        .n("cli", "channel")
        .n("web", "channel")
        .edge("ms1", "f1", "features")
        .edge("ms1", "f2", "features")
        .edge("ms2", "f1", "features")
        .edge("j1", "f1", "features")
        .edge("j1", "f2", "features")
        .edge("j1", "f3", "features")
        .edge("j2", "f2", "features")
        .edge("j1", "dev", "persona")
        .edge("j2", "dev", "persona")
        .edge("j1", "cli", "channels")
        .edge("j2", "cli", "channels")
        .edge("j3", "web", "channels")
        .edge("f2", "f1", "depends_on")
        .edge("f3", "f1", "depends_on")
}

// ── milestone completion ──────────────────────────────────────────────────

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "milestone with all features status=done returns ratio 1.0"
)]
fn a_milestone_whose_features_are_all_done_is_complete() {
    let mc = json_of("milestone_completion", json!({"milestone": "ms2"}), &plan());
    assert_eq!(mc["completion_ratio"], 1.0);
    assert_eq!(mc["done_features"], json!(["f1"]));
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "milestone with no done features returns ratio 0.0"
)]
fn a_milestone_with_no_done_feature_is_at_zero() {
    let g = plan()
        .node("ms4", "milestone", json!({"features": ["f2", "f3"]}))
        .edge("ms4", "f2", "features")
        .edge("ms4", "f3", "features");
    let mc = json_of("milestone_completion", json!({"milestone": "ms4"}), &g);
    assert_eq!(
        (mc["done_count"].clone(), mc["total_features"].clone()),
        (json!(0), json!(2))
    );
    assert_eq!(mc["completion_ratio"], 0.0);
    assert_eq!(mc["done_features"], json!([]));
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "empty milestone returns ratio 0.0 with zero features"
)]
fn an_empty_milestone_is_at_zero_without_dividing_by_zero() {
    let mc = json_of("milestone_completion", json!({"milestone": "ms3"}), &plan());
    assert_eq!(
        mc,
        json!({"milestone_id": "ms3", "total_features": 0, "done_count": 0,
            "completion_ratio": 0.0, "done_features": []})
    );
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "milestone with mix of done and non-done features returns partial ratio"
)]
fn a_milestone_half_done_is_at_one_half() {
    let mc = json_of("milestone_completion", json!({"milestone": "ms1"}), &plan());
    assert_eq!(mc["done_count"], 1);
    assert_eq!(mc["total_features"], 2);
    assert_eq!(mc["completion_ratio"], 0.5);
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "milestone completion is deterministic across repeated queries"
)]
fn milestone_completion_is_deterministic() {
    let runtime = runtime();
    let g = plan();
    let first = run_in(
        &runtime,
        "milestone_completion",
        json!({"milestone": "ms1"}),
        &g,
        "json",
    );
    for _ in 0..3 {
        let again = run_in(
            &runtime,
            "milestone_completion",
            json!({"milestone": "ms1"}),
            &g,
            "json",
        );
        assert_eq!(again.stdout, first.stdout);
    }
}

#[specforge_test(
    behavior = "surface_milestone_completion",
    verify = "milestone-completion returns MilestoneCompletionPayload JSON"
)]
fn milestone_completion_answers_its_payload() {
    let mc = json_of("milestone_completion", json!({"milestone": "ms1"}), &plan());
    let mut keys: Vec<&str> = mc.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "completion_ratio",
            "done_count",
            "done_features",
            "milestone_id",
            "total_features"
        ]
    );
    assert_eq!(mc["milestone_id"], "ms1");
}

#[specforge_test(
    behavior = "surface_milestone_completion",
    verify = "missing milestone ID returns error with suggestion"
)]
fn a_mistyped_milestone_is_not_found_with_the_nearest() {
    let error = run_in(
        &runtime(),
        "milestone_completion",
        json!({"milestone": "ms9"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["entity_id"], "ms9");
    assert_eq!(error["suggestion"], "ms1");
}

#[specforge_test(
    behavior = "surface_milestone_completion",
    verify = "human format shows ratio as percentage"
)]
fn milestone_completion_shows_a_percentage() {
    let out = human_of("milestone_completion", json!({"milestone": "ms1"}), &plan());
    assert_eq!(
        out,
        "Milestone: ms1 (active)\nCompletion: 50% (1/2 features done)\n  f1 [done]\n  f2 [in_progress]\n"
    );
}

#[specforge_test(
    behavior = "surface_milestone_completion",
    verify = "exit code 0 on success, 1 on error"
)]
fn milestone_completion_exits_zero_or_one() {
    let runtime = runtime();
    let g = plan();
    for format in ["human", "json"] {
        let ok = run_in(
            &runtime,
            "milestone_completion",
            json!({"milestone": "ms1"}),
            &g,
            format,
        );
        assert_eq!(ok.exit, 0);
        let missing = run_in(
            &runtime,
            "milestone_completion",
            json!({"milestone": "x"}),
            &g,
            format,
        );
        assert_eq!(missing.exit, 1);
        // Not a milestone, though an entity.
        let other = run_in(
            &runtime,
            "milestone_completion",
            json!({"milestone": "f1"}),
            &g,
            format,
        );
        assert_eq!(other.exit, 1);
    }
}

// ── journey coverage ──────────────────────────────────────────────────────

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "journey with all features status=done returns full coverage"
)]
fn a_journey_whose_features_are_done_is_covered() {
    let g = plan().n("j4", "journey").edge("j4", "f1", "features");
    let jc = json_of("journey_coverage", json!({"journey": "j4"}), &g);
    assert_eq!(jc["covered_count"], 1);
    assert_eq!(jc["total_features"], 1);
    assert_eq!(jc["uncovered_features"], json!([]));
}

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "journey with uncovered features lists them"
)]
fn a_journeys_features_not_done_are_uncovered() {
    let jc = json_of("journey_coverage", json!({"journey": "j1"}), &plan());
    assert_eq!(jc["covered_count"], 1);
    assert_eq!(jc["uncovered_features"], json!(["f2", "f3"]));
}

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "journey with zero features returns empty coverage"
)]
fn a_journey_without_features_has_empty_coverage() {
    let jc = json_of("journey_coverage", json!({"journey": "j3"}), &plan());
    assert_eq!(
        jc,
        json!({"journey_id": "j3", "total_features": 0, "covered_count": 0, "uncovered_features": []})
    );
}

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "features without status field are treated as uncovered"
)]
fn a_feature_without_a_status_is_uncovered() {
    let g = plan().n("j4", "journey").edge("j4", "f3", "features");
    let jc = json_of("journey_coverage", json!({"journey": "j4"}), &g);
    assert_eq!(jc["covered_count"], 0);
    assert_eq!(jc["uncovered_features"], json!(["f3"]));
}

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "journey coverage is deterministic across repeated queries"
)]
fn journey_coverage_is_deterministic() {
    let runtime = runtime();
    let g = plan();
    let first = run_in(
        &runtime,
        "journey_coverage",
        json!({"journey": "j1"}),
        &g,
        "json",
    );
    for _ in 0..3 {
        let again = run_in(
            &runtime,
            "journey_coverage",
            json!({"journey": "j1"}),
            &g,
            "json",
        );
        assert_eq!(again.stdout, first.stdout);
    }
}

#[specforge_test(
    behavior = "surface_journey_coverage",
    verify = "journey-coverage returns JourneyCoveragePayload JSON"
)]
fn journey_coverage_answers_its_payload() {
    let jc = json_of("journey_coverage", json!({"journey": "j1"}), &plan());
    assert_eq!(
        jc,
        json!({"journey_id": "j1", "total_features": 3, "covered_count": 1,
            "uncovered_features": ["f2", "f3"]})
    );
    // Its human layout: covered/total and the uncovered list.
    assert_eq!(
        human_of("journey_coverage", json!({"journey": "j1"}), &plan()),
        "Journey: j1 (persona: dev)\nCoverage: 33% (1/3 features done)\nUncovered:\n  f2\n  f3\n"
    );
}

#[specforge_test(
    behavior = "surface_journey_coverage",
    verify = "missing journey ID returns error with suggestion"
)]
fn a_mistyped_journey_is_not_found_with_the_nearest() {
    let error = run_in(
        &runtime(),
        "journey_coverage",
        json!({"journey": "jj1"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "j1");
}

#[specforge_test(
    behavior = "surface_journey_coverage",
    verify = "exit code 0 on success, 1 on error"
)]
fn journey_coverage_exits_zero_or_one() {
    let runtime = runtime();
    let g = plan();
    assert_eq!(
        run_in(
            &runtime,
            "journey_coverage",
            json!({"journey": "j1"}),
            &g,
            "human"
        )
        .exit,
        0
    );
    assert_eq!(
        run_in(
            &runtime,
            "journey_coverage",
            json!({"journey": "nope"}),
            &g,
            "human"
        )
        .exit,
        1
    );
}

// ── feature dependents ────────────────────────────────────────────────────

#[specforge_test(
    behavior = "pe_query_feature_dependents",
    verify = "feature with dependent returns that dependent"
)]
fn a_features_dependent_is_listed() {
    let g = G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .edge("f2", "f1", "depends_on");
    let fd = json_of("feature_dependents", json!({"feature": "f1"}), &g);
    assert_eq!(
        fd,
        json!({"feature_id": "f1", "dependents": ["f2"], "count": 1})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_dependents",
    verify = "feature with multiple dependents returns all sorted by ID"
)]
fn a_features_dependents_are_sorted_by_id() {
    // Declared in reverse order; one only relates to f1 (`features`).
    let g = G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .n("f4", "feature")
        .edge("f3", "f1", "depends_on")
        .edge("f2", "f1", "depends_on")
        .edge("f4", "f1", "features");
    let fd = json_of("feature_dependents", json!({"feature": "f1"}), &g);
    assert_eq!(fd["dependents"], json!(["f2", "f3"]));
    assert_eq!(fd["count"], 2);
}

#[specforge_test(
    behavior = "pe_query_feature_dependents",
    verify = "feature with no dependents returns empty list"
)]
fn a_feature_without_dependents_has_none() {
    let fd = json_of("feature_dependents", json!({"feature": "f2"}), &plan());
    assert_eq!(
        fd,
        json!({"feature_id": "f2", "dependents": [], "count": 0})
    );
}

#[specforge_test(
    behavior = "surface_feature_dependents",
    verify = "feature-dependents returns FeatureDependentPayload JSON"
)]
fn feature_dependents_answers_its_payload() {
    let fd = json_of("feature_dependents", json!({"feature": "f1"}), &plan());
    assert_eq!(
        fd,
        json!({"feature_id": "f1", "dependents": ["f2", "f3"], "count": 2})
    );
    assert_eq!(
        human_of("feature_dependents", json!({"feature": "f1"}), &plan()),
        "Features depending on 'f1':\n  f2\n  f3\n"
    );
}

#[specforge_test(
    behavior = "surface_feature_dependents",
    verify = "missing feature ID returns error with suggestion"
)]
fn a_mistyped_feature_is_not_found_with_the_nearest() {
    let runtime = runtime();
    let error = run_in(
        &runtime,
        "feature_dependents",
        json!({"feature": "f11"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "f1");
    // An entity of another kind is not a feature.
    let error = run_in(
        &runtime,
        "feature_dependents",
        json!({"feature": "ms1"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
}

#[specforge_test(
    behavior = "surface_feature_impact",
    verify = "missing feature ID returns error with suggestion"
)]
fn feature_impact_of_a_mistyped_feature_suggests_the_nearest() {
    let error = run_in(
        &runtime(),
        "feature_impact",
        json!({"feature": "f9"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "f1");
}

// ── persona and channel features ──────────────────────────────────────────

#[specforge_test(
    behavior = "pe_query_persona_features",
    verify = "persona with one journey returns that journey's features"
)]
fn a_personas_journey_gives_its_features() {
    let g = G::default()
        .n("p1", "persona")
        .n("j1", "journey")
        .n("f1", "feature")
        .n("f2", "feature")
        .edge("j1", "p1", "persona")
        .edge("j1", "f2", "features")
        .edge("j1", "f1", "features");
    let pf = json_of("persona_features", json!({"persona": "p1"}), &g);
    assert_eq!(
        pf,
        json!({"persona_id": "p1", "features": ["f1", "f2"], "via_journey_ids": ["j1"], "count": 2})
    );
}

#[specforge_test(
    behavior = "pe_query_persona_features",
    verify = "persona with multiple journeys returns deduplicated features"
)]
fn a_personas_journeys_give_each_feature_once() {
    let pf = json_of("persona_features", json!({"persona": "dev"}), &plan());
    assert_eq!(pf["features"], json!(["f1", "f2", "f3"]));
    assert_eq!(pf["via_journey_ids"], json!(["j1", "j2"]));
    assert_eq!(pf["count"], 3);
}

#[specforge_test(
    behavior = "pe_query_persona_features",
    verify = "persona with no journeys returns empty features"
)]
fn a_persona_without_journeys_has_no_features() {
    let pf = json_of("persona_features", json!({"persona": "ops"}), &plan());
    assert_eq!(
        pf,
        json!({"persona_id": "ops", "features": [], "via_journey_ids": [], "count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_persona_features",
    verify = "nonexistent persona returns ENTITY_NOT_FOUND with suggestion"
)]
fn a_mistyped_persona_is_not_found_with_the_nearest() {
    let error = run_in(
        &runtime(),
        "persona_features",
        json!({"persona": "dex"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "dev");
}

#[specforge_test(
    behavior = "surface_persona_features",
    verify = "persona-features returns PersonaFeaturePayload JSON"
)]
fn persona_features_answers_its_payload() {
    let pf = json_of("persona_features", json!({"persona": "dev"}), &plan());
    let mut keys: Vec<&str> = pf.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["count", "features", "persona_id", "via_journey_ids"]);
    assert_eq!(
        human_of("persona_features", json!({"persona": "dev"}), &plan()),
        "Features for persona 'dev':\n  f1\n  f2\n  f3\n"
    );
}

#[specforge_test(
    behavior = "surface_persona_features",
    verify = "missing persona ID returns error with suggestion"
)]
fn persona_features_of_a_mistyped_persona_suggests_the_nearest() {
    let out = run_in(
        &runtime(),
        "persona_features",
        json!({"persona": "ops2"}),
        &plan(),
        "human",
    );
    assert_eq!(out.exit, 1);
    assert_eq!(
        out.stderr,
        "error: persona 'ops2' not found\ndid you mean 'ops'?\n"
    );
}

#[specforge_test(
    behavior = "pe_query_channel_features",
    verify = "channel with one journey and features returns those features"
)]
fn a_channels_journey_gives_its_features() {
    let g = G::default()
        .n("c1", "channel")
        .n("j1", "journey")
        .n("f1", "feature")
        .edge("j1", "c1", "channels")
        .edge("j1", "f1", "features");
    let cf = json_of("channel_features", json!({"channel": "c1"}), &g);
    assert_eq!(cf["features"], json!(["f1"]));
    assert_eq!(cf["count"], 1);
}

#[specforge_test(
    behavior = "pe_query_channel_features",
    verify = "channel with multiple journeys sharing features deduplicates"
)]
fn a_channels_journeys_give_each_feature_once() {
    let cf = json_of("channel_features", json!({"channel": "cli"}), &plan());
    assert_eq!(cf["features"], json!(["f1", "f2", "f3"]));
    assert_eq!(cf["count"], 3);
}

#[specforge_test(
    behavior = "pe_query_channel_features",
    verify = "channel with no journeys returns empty list"
)]
fn a_channel_without_journeys_has_no_features() {
    let g = plan().n("tui", "channel");
    let cf = json_of("channel_features", json!({"channel": "tui"}), &g);
    assert_eq!(
        cf,
        json!({"channel_id": "tui", "features": [], "via_journey_ids": [], "count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_channel_features",
    verify = "channel with journeys without features returns empty list"
)]
fn a_channel_whose_journeys_have_no_features_has_none() {
    let cf = json_of("channel_features", json!({"channel": "web"}), &plan());
    assert_eq!(cf["features"], json!([]));
    assert_eq!(cf["via_journey_ids"], json!(["j3"]));
}

#[specforge_test(
    behavior = "pe_query_channel_features",
    verify = "result includes via_journey_ids for traceability"
)]
fn channel_features_name_the_journeys_they_came_through() {
    let cf = json_of("channel_features", json!({"channel": "cli"}), &plan());
    assert_eq!(cf["via_journey_ids"], json!(["j1", "j2"]));
}

#[specforge_test(
    behavior = "surface_channel_features",
    verify = "channel-features returns ChannelFeaturePayload JSON"
)]
fn channel_features_answers_its_payload() {
    let cf = json_of("channel_features", json!({"channel": "cli"}), &plan());
    assert_eq!(
        cf,
        json!({"channel_id": "cli", "features": ["f1", "f2", "f3"],
            "via_journey_ids": ["j1", "j2"], "count": 3})
    );
}

#[specforge_test(
    behavior = "surface_channel_features",
    verify = "missing channel ID returns error with suggestion"
)]
fn a_mistyped_channel_is_not_found_with_the_nearest() {
    let error = run_in(
        &runtime(),
        "channel_features",
        json!({"channel": "cly"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "cli");
}

// ── project-wide ──────────────────────────────────────────────────────────

#[specforge_test(
    behavior = "surface_bulk_status",
    verify = "bulk-status counts each kind's entities by status"
)]
fn bulk_status_counts_each_kind_by_status() {
    let bs = json_of("bulk_status", json!({}), &plan());
    assert_eq!(
        bs,
        json!({"kinds": [
            {"kind": "feature", "total": 3, "by_status": [
                {"status": "(none)", "count": 1},
                {"status": "done", "count": 1},
                {"status": "in_progress", "count": 1}]},
            {"kind": "milestone", "total": 3, "by_status": [
                {"status": "(none)", "count": 2},
                {"status": "active", "count": 1}]},
            {"kind": "persona", "total": 2, "by_status": [{"status": "(none)", "count": 2}]},
            {"kind": "channel", "total": 2, "by_status": [{"status": "(none)", "count": 2}]},
        ]})
    );
    // Each kind's total is the sum of its counts.
    for kind in bs["kinds"].as_array().unwrap() {
        let sum: u64 = kind["by_status"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["count"].as_u64().unwrap())
            .sum();
        assert_eq!(kind["total"], sum);
    }
}

#[specforge_test(
    behavior = "surface_format_conventions",
    verify = "human table output has header and aligned columns"
)]
fn bulk_status_is_a_table_for_people() {
    let out = human_of("bulk_status", json!({}), &plan());
    assert_eq!(
        out,
        "kind       status       count\n\
         feature    (none)       1\n\
         feature    done         1\n\
         feature    in_progress  1\n\
         milestone  (none)       2\n\
         milestone  active       1\n\
         persona    (none)       2\n\
         channel    (none)       2\n"
    );
    // Every column starts where its header does, two spaces after the
    // widest cell before it.
    let lines: Vec<&str> = out.lines().collect();
    for header in ["status", "count"] {
        let at = lines[0].find(header).unwrap();
        for line in &lines[1..] {
            assert_eq!(&line[at - 2..at], "  ", "{line}");
            assert_ne!(&line[at..at + 1], " ", "{line}");
        }
    }
}

#[specforge_test(
    behavior = "surface_health",
    verify = "health reports the score, counts and orphans"
)]
fn health_reports_the_score_counts_and_orphans() {
    let h = json_of("health", json!({}), &plan());
    let mut keys: Vec<&str> = h.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["completeness", "entity_counts", "orphan_counts", "score"]
    );
    let score = &h["score"];
    for part in ["overall", "coverage", "connectivity", "completeness"] {
        let value = score[part].as_f64().unwrap();
        assert!((0.0..=100.0).contains(&value), "{part}: {value}");
    }
    let mean = (score["coverage"].as_f64().unwrap()
        + score["connectivity"].as_f64().unwrap()
        + score["completeness"].as_f64().unwrap())
        / 3.0;
    assert!((score["overall"].as_f64().unwrap() - mean).abs() < 1e-9);
    let count = |kind: &str| {
        h["entity_counts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["kind"] == kind)
            .unwrap()["count"]
            .clone()
    };
    assert_eq!((count("feature"), count("journey")), (json!(3), json!(3)));
    let orphans = h["orphan_counts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "milestone")
        .unwrap();
    assert_eq!(
        (orphans["orphans"].clone(), orphans["total"].clone()),
        (json!(3), json!(3))
    );
    assert_eq!(h["completeness"]["features_total"], 3);
    assert_eq!(h["completeness"]["milestones_with_features"], 2);
}

// ── the shared contract ───────────────────────────────────────────────────

/// Every entity-scoped command, with its positional arg and an id of its
/// kind in [`plan`].
const ENTITY_SCOPED: &[(&str, &str, &str)] = &[
    ("milestone_completion", "milestone", "ms1"),
    ("journey_coverage", "journey", "j1"),
    ("feature_impact", "feature", "f1"),
    ("feature_dependents", "feature", "f1"),
    ("persona_features", "persona", "dev"),
    ("channel_features", "channel", "cli"),
];

/// Every command's id, with the args that make it answer over [`plan`].
fn every_command() -> Vec<(&'static str, Value)> {
    let mut commands: Vec<(&str, Value)> = [
        "features",
        "journeys",
        "deliverables",
        "milestones",
        "modules",
        "terms",
        "personas",
        "channels",
        "releases",
        "bulk_status",
        "health",
    ]
    .into_iter()
    .map(|id| (id, json!({})))
    .collect();
    for (id, arg, value) in ENTITY_SCOPED {
        commands.push((id, json!({ *arg: value })));
    }
    commands
}

#[specforge_test(
    behavior = "surface_query_command_contract",
    verify = "entity-scoped query with valid ID returns typed payload"
)]
fn an_entity_scoped_query_answers_about_its_entity() {
    let runtime = runtime();
    let g = plan();
    for (id, arg, value) in ENTITY_SCOPED {
        let payload = run_in(&runtime, id, json!({ *arg: value }), &g, "json").json();
        assert_eq!(payload[format!("{arg}_id")], *value, "{id}: {payload}");
    }
}

#[specforge_test(
    behavior = "surface_query_command_contract",
    verify = "entity-scoped query with invalid ID returns ENTITY_NOT_FOUND"
)]
fn an_entity_scoped_query_about_no_entity_is_not_found() {
    let runtime = runtime();
    let g = plan();
    for (id, arg, _) in ENTITY_SCOPED {
        let out = run_in(&runtime, id, json!({ *arg: "zzzzzz" }), &g, "json");
        assert_eq!(out.exit, 1, "{id}");
        let error = out.error();
        assert_eq!(error["code"], "ENTITY_NOT_FOUND", "{id}");
        assert_eq!(error["entity_id"], "zzzzzz", "{id}");
        assert!(error.get("suggestion").is_none(), "{id}: {error}");
        assert_eq!(error["message"], format!("{arg} 'zzzzzz' not found"));
    }
}

#[specforge_test(
    behavior = "surface_query_command_contract",
    verify = "entity-scoped query with close typo returns suggestion"
)]
fn an_entity_scoped_query_with_a_typo_suggests_its_kinds_nearest() {
    let runtime = runtime();
    let g = plan();
    for (id, arg, value) in ENTITY_SCOPED {
        let typo = format!("{value}x");
        let error = run_in(&runtime, id, json!({ *arg: typo }), &g, "json").error();
        assert_eq!(error["suggestion"], *value, "{id}: {error}");
    }
}

#[specforge_test(
    behavior = "surface_query_command_contract",
    verify = "project-wide query returns typed payload"
)]
fn a_project_wide_query_answers_its_payload() {
    let runtime = runtime();
    let g = plan();
    let bs = run_in(&runtime, "bulk_status", json!({}), &g, "json").json();
    assert!(bs["kinds"].is_array(), "{bs}");
    let h = run_in(&runtime, "health", json!({}), &g, "json").json();
    assert!(h["score"]["overall"].is_number(), "{h}");
}

#[specforge_test(
    behavior = "surface_query_command_contract",
    verify = "query result respects --format=json"
)]
fn a_query_prints_json_only_when_asked() {
    let runtime = runtime();
    let g = plan();
    for (id, args) in every_command() {
        let json = run_in(&runtime, id, args.clone(), &g, "json");
        let human = run_in(&runtime, id, args, &g, "human");
        json.json();
        assert!(
            serde_json::from_str::<Value>(&human.stdout).is_err(),
            "{id}: human is not JSON: {}",
            human.stdout
        );
    }
}

#[specforge_test(
    behavior = "surface_format_conventions",
    verify = "json output is valid JSON"
)]
fn every_command_prints_one_json_object() {
    let runtime = runtime();
    for g in [G::default(), plan()] {
        for (id, args) in every_command() {
            let out = run_in(&runtime, id, args, &g, "json");
            if out.exit == 0 {
                out.json();
            } else {
                // Over the empty graph an entity-scoped one is not found:
                // its error is JSON too.
                assert!(out.error().is_object(), "{id}");
            }
        }
    }
}

#[specforge_test(
    behavior = "surface_error_handling",
    verify = "entity-not-found returns ENTITY_NOT_FOUND code"
)]
fn a_missing_entity_is_entity_not_found_exiting_one() {
    let out = run_in(
        &runtime(),
        "journey_coverage",
        json!({"journey": "nope"}),
        &plan(),
        "json",
    );
    assert_eq!(out.exit, 1);
    assert_eq!(
        out.error(),
        json!({"code": "ENTITY_NOT_FOUND", "message": "journey 'nope' not found", "entity_id": "nope"})
    );
}

#[specforge_test(
    behavior = "surface_error_handling",
    verify = "fuzzy-match suggestion present when close match exists"
)]
fn a_close_id_of_the_same_kind_is_suggested() {
    let runtime = runtime();
    let g = plan();
    // Within two edits, the nearest of the kind.
    let error = run_in(
        &runtime,
        "milestone_completion",
        json!({"milestone": "mx1"}),
        &g,
        "json",
    )
    .error();
    assert_eq!(error["suggestion"], "ms1");
    // Three edits away is too far.
    let error = run_in(
        &runtime,
        "milestone_completion",
        json!({"milestone": "xyz1"}),
        &g,
        "json",
    )
    .error();
    assert!(error.get("suggestion").is_none(), "{error}");
    // An id of another kind is not suggested: f1 is a feature.
    let error = run_in(
        &runtime,
        "milestone_completion",
        json!({"milestone": "f1"}),
        &g,
        "json",
    )
    .error();
    assert_ne!(error["suggestion"], "f1", "{error}");
}

#[specforge_test(
    behavior = "surface_error_handling",
    verify = "no surface panics on null, empty, or malformed input"
)]
fn no_command_panics_on_odd_input() {
    let runtime = runtime();
    let odd_args = [
        json!({}),
        json!({"milestone": null, "journey": null, "feature": null, "persona": null,
            "channel": null, "status": null, "limit": null}),
        json!({"milestone": 3, "journey": [], "feature": {}, "persona": true, "channel": 1.5,
            "status": 7, "priority": [], "limit": "many", "offset": -1}),
        json!({"milestone": "", "limit": 1e30, "offset": "9999999999999999999999"}),
    ];
    for (id, _) in every_command() {
        let export = format!("cmd__product_{id}");
        for args in &odd_args {
            for graph in [json!({}), json!({"nodes": [], "edges": []}), plan().graph()] {
                let input = json!({"args": args, "graph": graph});
                let result = runtime.call_export(PRODUCT, &export, input.to_string().as_bytes());
                assert!(
                    matches!(result, WasmCallResult::Ok(_)),
                    "{export} with {args}"
                );
            }
        }
        // Input that is not a CommandInput is refused, not a panic.
        for malformed in [&b"null"[..], b"", b"{", b"[1]", b"{\"args\": 3}"] {
            let result = runtime.call_export(PRODUCT, &export, malformed);
            if let WasmCallResult::Trap(trap) = &result {
                assert!(
                    !trap.message.contains("panicked"),
                    "{export} panicked on {malformed:?}: {}",
                    trap.message
                );
            }
        }
    }
}

#[specforge_test(
    behavior = "pe_query_entity_not_found",
    verify = "query with missing entity ID returns error"
)]
fn a_query_about_a_missing_id_errs() {
    let error = run_in(
        &runtime(),
        "feature_dependents",
        json!({"feature": "gone"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["message"], "feature 'gone' not found");
}

#[specforge_test(
    behavior = "pe_query_entity_not_found",
    verify = "query with missing ID and close match includes suggestion"
)]
fn a_query_about_a_near_miss_suggests() {
    let error = run_in(
        &runtime(),
        "channel_features",
        json!({"channel": "webb"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["suggestion"], "web");
}

#[specforge_test(
    behavior = "pe_query_entity_not_found",
    verify = "query with missing ID and no close match omits suggestion"
)]
fn a_query_about_a_far_miss_suggests_nothing() {
    let runtime = runtime();
    let out = run_in(
        &runtime,
        "channel_features",
        json!({"channel": "satellite"}),
        &plan(),
        "human",
    );
    assert_eq!(out.stderr, "error: channel 'satellite' not found\n");
    let error = run_in(
        &runtime,
        "channel_features",
        json!({"channel": "satellite"}),
        &plan(),
        "json",
    )
    .error();
    assert!(error.get("suggestion").is_none(), "{error}");
}
