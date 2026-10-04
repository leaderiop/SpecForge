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

thread_local! {
    /// One runtime per test thread, for the helpers that run many commands.
    static RUNTIME: ComponentRuntime = runtime();
}

fn json_of(id: &str, args: Value, g: &G) -> Value {
    RUNTIME.with(|runtime| run_in(runtime, id, args, g, "json").json())
}

fn human_of(id: &str, args: Value, g: &G) -> String {
    let out = run_in(&runtime(), id, args, g, "human");
    assert_eq!(out.exit, 0, "{}", out.stderr);
    out.stdout
}

/// The graph most tests ask about: milestones, journeys, personas, channels
/// and a deliverable over three features.
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
        .n("d1", "deliverable")
        .n("r1", "release")
        .n("mod1", "module")
        .edge("d1", "j1", "journeys")
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

// ── lists ─────────────────────────────────────────────────────────────────

/// The nine list commands and the key each payload's entries are under.
const LISTS: &[&str] = &[
    "features",
    "journeys",
    "deliverables",
    "milestones",
    "modules",
    "terms",
    "personas",
    "channels",
    "releases",
];

/// A few entities of every kind, with the fields the lists filter on.
fn catalog() -> G {
    G::default()
        .node(
            "f1",
            "feature",
            json!({"status": "done", "priority": "high", "tags": ["core", "cli"], "problem": "p"}),
        )
        .node(
            "f2",
            "feature",
            json!({"status": "done", "priority": "low", "tags": ["web"]}),
        )
        .node("f3", "feature", json!({"priority": "high"}))
        .node(
            "f4",
            "feature",
            json!({"status": "deferred", "priority": "critical"}),
        )
        .node(
            "j1",
            "journey",
            json!({"persona": "dev", "priority": "high"}),
        )
        .node("j2", "journey", json!({"persona": "ops"}))
        .n("j3", "journey")
        .node(
            "d1",
            "deliverable",
            json!({"artifact_type": "cli", "status": "shipped"}),
        )
        .node("d2", "deliverable", json!({"artifact_type": "web_app"}))
        .node(
            "ms1",
            "milestone",
            json!({"status": "completed", "target_date": "2026-01-01"}),
        )
        .n("ms2", "milestone")
        .node("mod1", "module", json!({"family": "core"}))
        .node("mod2", "module", json!({"family": "experimental"}))
        .node(
            "t1",
            "term",
            json!({"definition": "one", "aliases": ["uno", "eins"]}),
        )
        .node("t2", "term", json!({"definition": "two"}))
        .node(
            "dev",
            "persona",
            json!({"technical_level": "expert", "status": "active"}),
        )
        .node("ops", "persona", json!({"technical_level": "beginner"}))
        .node(
            "cli",
            "channel",
            json!({"interaction_model": "batch", "status": "active"}),
        )
        .node("web", "channel", json!({"interaction_model": "streaming"}))
        .node(
            "r1",
            "release",
            json!({"version": "1.0.0", "status": "released"}),
        )
        .n("r2", "release")
        .edge("j1", "dev", "persona")
        .edge("j2", "ops", "persona")
        .edge("j1", "cli", "channels")
        .edge("j1", "web", "channels")
        .edge("j2", "cli", "channels")
        .edge("j1", "f1", "features")
        .edge("j1", "f2", "features")
        .edge("j3", "f3", "features")
        .edge("d1", "j1", "journeys")
        .edge("d1", "mod1", "modules")
        .edge("d1", "mod2", "modules")
        .edge("d2", "j2", "journeys")
        .edge("ms1", "f1", "features")
        .edge("ms1", "f2", "features")
        .edge("mod1", "f1", "features")
        .edge("mod1", "mod2", "depends_on")
        .edge("r1", "d1", "deliverables")
}

/// The ids `list` answers with `args` over `g`, in order.
fn ids_of(list: &str, args: Value, g: &G) -> Vec<String> {
    let payload = json_of(list, args, g);
    payload[list]
        .as_array()
        .unwrap_or_else(|| panic!("{list}: no {list} array: {payload}"))
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect()
}

/// The entry `id` of `list` over [`catalog`].
fn entry_of(list: &str, id: &str) -> Value {
    json_of(list, json!({}), &catalog())[list]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id)
        .unwrap_or_else(|| panic!("{list}: no {id}"))
        .clone()
}

/// What `list` answers when it refuses `args`: its exit code and error.
fn refused(list: &str, args: Value) -> (i64, Value) {
    let out = RUNTIME.with(|runtime| run_in(runtime, list, args, &catalog(), "json"));
    let exit = out.exit;
    (exit, out.error())
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "filter by status returns only matching entities"
)]
fn a_list_filtered_by_status_has_only_that_status() {
    let g = catalog();
    assert_eq!(
        ids_of("features", json!({"status": "done"}), &g),
        ["f1", "f2"]
    );
    // A feature without a status is proposed.
    assert_eq!(
        ids_of("features", json!({"status": "proposed"}), &g),
        ["f3"]
    );
    assert_eq!(
        ids_of("milestones", json!({"status": "completed"}), &g),
        ["ms1"]
    );
    // A persona or channel without a status is active.
    assert_eq!(
        ids_of("personas", json!({"status": "active"}), &g),
        ["dev", "ops"]
    );
    assert_eq!(
        ids_of("channels", json!({"status": "active"}), &g),
        ["cli", "web"]
    );
    assert!(ids_of("personas", json!({"status": "deprecated"}), &g).is_empty());
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "filter by priority returns only matching entities"
)]
fn a_list_filtered_by_priority_has_only_that_priority() {
    let g = catalog();
    assert_eq!(
        ids_of("features", json!({"priority": "high"}), &g),
        ["f1", "f3"]
    );
    assert_eq!(ids_of("journeys", json!({"priority": "high"}), &g), ["j1"]);
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "combined status+priority filter uses AND logic"
)]
fn list_filters_combine_with_and() {
    let g = catalog();
    let args = json!({"status": "done", "priority": "high"});
    assert_eq!(ids_of("features", args, &g), ["f1"]);
    let args = json!({"status": "done", "priority": "critical"});
    assert!(ids_of("features", args, &g).is_empty());
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "invalid status filter returns INVALID_INPUT"
)]
fn a_status_outside_the_kinds_enum_is_invalid_input() {
    // `draft` is a deliverable's status, not a feature's.
    let (exit, error) = refused("features", json!({"status": "draft"}));
    assert_eq!(exit, 2);
    assert_eq!(error["code"], "INVALID_INPUT");
    assert_eq!(
        error["message"],
        "status must be one of proposed, accepted, in_progress, done, deferred, deprecated, got 'draft'"
    );
    // The same value is one a deliverable's status takes.
    assert_eq!(
        ids_of("deliverables", json!({"status": "draft"}), &catalog()),
        ["d2"]
    );
}

#[specforge_test(
    behavior = "surface_error_handling",
    verify = "invalid filter value returns INVALID_INPUT code"
)]
fn every_closed_filter_refuses_a_value_outside_its_enum() {
    for (list, arg) in [
        ("features", "status"),
        ("features", "priority"),
        ("journeys", "priority"),
        ("deliverables", "status"),
        ("deliverables", "artifact_type"),
        ("milestones", "status"),
        ("milestones", "priority"),
        ("personas", "status"),
        ("personas", "technical_level"),
        ("channels", "status"),
        ("channels", "interaction_model"),
        ("releases", "status"),
    ] {
        let (exit, error) = refused(list, json!({ arg: "nonesuch" }));
        assert_eq!(exit, 2, "{list} --{arg}");
        assert_eq!(error["code"], "INVALID_INPUT", "{list} --{arg}");
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .starts_with(&format!("{arg} must be one of ")),
            "{list} --{arg}: {error}"
        );
    }
    // An open filter matches what it is given: a non-standard family is a
    // family (I062 says so, as an info).
    assert_eq!(
        ids_of("modules", json!({"family": "experimental"}), &catalog()),
        ["mod2"]
    );
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "sort by priority with tie-break by ID is deterministic"
)]
fn a_list_sorted_by_priority_ties_by_id() {
    let g = catalog();
    let asc = json!({"sort_by": "priority"});
    // Priority's order, most important first; ties by id.
    assert_eq!(
        ids_of("features", asc.clone(), &g),
        ["f4", "f1", "f3", "f2"]
    );
    assert_eq!(ids_of("features", asc, &g), ["f4", "f1", "f3", "f2"]);
    let desc = json!({"sort_by": "priority", "sort_order": "desc"});
    assert_eq!(ids_of("features", desc, &g), ["f2", "f1", "f3", "f4"]);
    // A text field sorts by its text.
    let by_version = json!({"sort_by": "version", "sort_order": "desc"});
    assert_eq!(ids_of("releases", by_version, &g), ["r1", "r2"]);
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "invalid sort_by field returns INVALID_INPUT"
)]
fn a_sort_field_the_kind_lacks_is_invalid_input() {
    let (exit, error) = refused("terms", json!({"sort_by": "priority"}));
    assert_eq!(exit, 2);
    assert_eq!(error["code"], "INVALID_INPUT");
    assert_eq!(error["message"], "sort_by: a term has no field 'priority'");
    let (exit, error) = refused("features", json!({"sort_order": "sideways"}));
    assert_eq!((exit, error["code"].clone()), (2, json!("INVALID_INPUT")));
}

/// A graph of `n` features, `f00` to `f<n-1>`.
fn features(n: usize) -> G {
    (0..n).fold(G::default(), |g, i| g.n(&format!("f{i:02}"), "feature"))
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "limit=0 is clamped to 1"
)]
fn a_limit_of_zero_is_one() {
    let page = json_of("features", json!({"limit": 0}), &features(3));
    assert_eq!(page["limit"], 1);
    assert_eq!(page["features"].as_array().unwrap().len(), 1);
    assert_eq!(page["has_more"], true);
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "limit=5000 is clamped to 1000"
)]
fn a_limit_past_a_thousand_is_a_thousand() {
    let page = json_of("features", json!({"limit": 5000}), &features(1001));
    assert_eq!(page["limit"], 1000);
    assert_eq!(page["features"].as_array().unwrap().len(), 1000);
    assert_eq!(
        (page["total"].clone(), page["has_more"].clone()),
        (json!(1001), json!(true))
    );
    // Unset, a page is 100.
    let page = json_of("features", json!({}), &features(101));
    assert_eq!(page["limit"], 100);
    assert_eq!(page["features"].as_array().unwrap().len(), 100);
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "offset beyond total returns empty list"
)]
fn an_offset_past_the_end_is_an_empty_page() {
    let page = json_of("features", json!({"offset": 10}), &features(3));
    assert_eq!(
        page,
        json!({"features": [], "total": 3, "offset": 10, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "empty graph returns total=0 and has_more=false"
)]
fn every_list_over_an_empty_graph_is_empty() {
    let runtime = runtime();
    for list in LISTS {
        let page = run_in(&runtime, list, json!({}), &G::default(), "json").json();
        assert_eq!(
            page,
            json!({ *list: [], "total": 0, "offset": 0, "limit": 100, "has_more": false }),
            "{list}"
        );
    }
}

/// Every list's pages over [`catalog`] for a range of offsets and limits.
fn every_page() -> Vec<(&'static str, usize, usize, Value)> {
    let runtime = runtime();
    let g = catalog();
    let mut pages = Vec::new();
    for list in LISTS {
        for offset in 0..5 {
            for limit in 1..5 {
                let args = json!({"offset": offset, "limit": limit});
                let page = run_in(&runtime, list, args, &g, "json").json();
                pages.push((*list, offset, limit, page));
            }
        }
    }
    pages
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "for all list commands: entities.length <= limit"
)]
fn no_page_is_longer_than_its_limit() {
    for (list, _, limit, page) in every_page() {
        assert!(
            page[list].as_array().unwrap().len() <= limit,
            "{list}: {page}"
        );
        assert_eq!(page["limit"], limit);
    }
}

#[specforge_test(
    behavior = "surface_list_command_contract",
    verify = "for all list commands: has_more == (offset + entities.length < total)"
)]
fn a_page_has_more_iff_entries_remain_after_it() {
    for (list, offset, _, page) in every_page() {
        let shown = page[list].as_array().unwrap().len();
        let total = page["total"].as_u64().unwrap() as usize;
        assert_eq!(page["has_more"], offset + shown < total, "{list}: {page}");
        assert_eq!(page["offset"], offset);
    }
}

#[specforge_test(
    behavior = "product_pagination_sort_stability",
    verify = "page 0 + page 1 concatenation equals unpaginated result"
)]
fn two_pages_are_the_whole_list() {
    let g = features(7);
    let args = |offset: usize| json!({"offset": offset, "limit": 4, "sort_by": "title"});
    let mut paged = ids_of("features", args(0), &g);
    paged.extend(ids_of("features", args(4), &g));
    assert_eq!(paged, ids_of("features", json!({"sort_by": "title"}), &g));
}

#[specforge_test(
    behavior = "product_pagination_sort_stability",
    verify = "entities with same priority sorted alphabetically by ID"
)]
fn entities_of_one_priority_are_by_id() {
    let g = ["c", "a", "b"].iter().fold(G::default(), |g, id| {
        g.node(id, "feature", json!({"priority": "medium"}))
    });
    for order in ["asc", "desc"] {
        let args = json!({"sort_by": "priority", "sort_order": order});
        assert_eq!(ids_of("features", args, &g), ["a", "b", "c"], "{order}");
    }
}

#[specforge_test(
    behavior = "product_pagination_sort_stability",
    verify = "union of all pages equals full result set with no duplicates"
)]
fn the_pages_of_a_list_are_its_entries_once_each() {
    let g = catalog();
    for list in LISTS {
        let all = ids_of(list, json!({}), &g);
        for limit in 1..4 {
            let mut seen: Vec<String> = Vec::new();
            let mut offset = 0;
            loop {
                let page = json_of(list, json!({"offset": offset, "limit": limit}), &g);
                let ids: Vec<String> = page[*list]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| e["id"].as_str().unwrap().to_string())
                    .collect();
                offset += ids.len();
                seen.extend(ids);
                if page["has_more"] == false {
                    break;
                }
            }
            assert_eq!(seen, all, "{list} by {limit}");
        }
    }
}

// ── each list ─────────────────────────────────────────────────────────────

#[specforge_test(
    behavior = "surface_list_features",
    verify = "list features returns paginated FeatureListResult"
)]
fn the_features_list_is_a_feature_list_result() {
    let page = json_of("features", json!({"limit": 2}), &catalog());
    assert_eq!(
        page,
        json!({"features": [
            {"id": "f1", "title": "f1", "status": "done", "priority": "high", "problem": "p",
                "tags": ["core", "cli"]},
            {"id": "f2", "title": "f2", "status": "done", "priority": "low", "tags": ["web"]},
        ], "total": 4, "offset": 0, "limit": 2, "has_more": true})
    );
}

#[specforge_test(
    behavior = "surface_list_features",
    verify = "priority filter reduces result set"
)]
fn the_features_list_filters_by_priority() {
    assert_eq!(
        ids_of("features", json!({"priority": "critical"}), &catalog()),
        ["f4"]
    );
}

#[specforge_test(
    behavior = "surface_list_features",
    verify = "tags filter intersects correctly"
)]
fn the_features_list_keeps_features_sharing_a_tag() {
    let g = catalog();
    assert_eq!(ids_of("features", json!({"tags": "cli"}), &g), ["f1"]);
    assert_eq!(
        ids_of("features", json!({"tags": "web, core"}), &g),
        ["f1", "f2"]
    );
    assert!(ids_of("features", json!({"tags": "mobile"}), &g).is_empty());
    // An empty tag list filters nothing.
    assert_eq!(ids_of("features", json!({"tags": ""}), &g).len(), 4);
}

#[specforge_test(
    behavior = "surface_list_features",
    verify = "empty project returns total=0 and empty list"
)]
fn the_features_list_of_no_features_is_empty() {
    let page = json_of("features", json!({}), &G::default().n("j1", "journey"));
    assert_eq!(
        (page["total"].clone(), page["features"].clone()),
        (json!(0), json!([]))
    );
}

#[specforge_test(
    behavior = "surface_list_features",
    verify = "human format is a table with a header row"
)]
fn the_features_list_is_a_table_for_people() {
    let out = human_of("features", json!({"limit": 3}), &catalog());
    assert_eq!(
        out,
        "id  title  status  priority\n\
         f1  f1     done    high\n\
         f2  f2     done    low\n\
         f3  f3     -       high\n\
         3 of 4 features; --offset 3 for more\n"
    );
}

#[specforge_test(
    behavior = "surface_list_journeys",
    verify = "list journeys returns paginated JourneyListResult"
)]
fn the_journeys_list_is_a_journey_list_result() {
    let page = json_of("journeys", json!({}), &catalog());
    assert_eq!(page["total"], 3);
    assert_eq!(
        page["journeys"][0],
        json!({"id": "j1", "title": "j1", "persona": "dev", "channel_count": 2,
            "feature_count": 2, "priority": "high"})
    );
}

#[specforge_test(
    behavior = "surface_list_journeys",
    verify = "persona filter reduces result set to matching journeys"
)]
fn the_journeys_list_filters_by_persona() {
    let g = catalog();
    assert_eq!(ids_of("journeys", json!({"persona": "ops"}), &g), ["j2"]);
    assert!(ids_of("journeys", json!({"persona": "nobody"}), &g).is_empty());
}

#[specforge_test(
    behavior = "surface_list_journeys",
    verify = "channel_count and feature_count are accurate per entry"
)]
fn a_journey_entry_counts_its_channels_and_features() {
    let j2 = entry_of("journeys", "j2");
    assert_eq!(
        (j2["channel_count"].clone(), j2["feature_count"].clone()),
        (json!(1), json!(0))
    );
    let j3 = entry_of("journeys", "j3");
    assert_eq!(
        (j3["channel_count"].clone(), j3["feature_count"].clone()),
        (json!(0), json!(1))
    );
}

#[specforge_test(
    behavior = "surface_list_deliverables",
    verify = "list deliverables returns paginated DeliverableListResult"
)]
fn the_deliverables_list_is_a_deliverable_list_result() {
    let page = json_of("deliverables", json!({}), &catalog());
    assert_eq!(
        page,
        json!({"deliverables": [
            {"id": "d1", "title": "d1", "artifact_type": "cli", "status": "shipped",
                "journey_count": 1, "module_count": 2},
            {"id": "d2", "title": "d2", "artifact_type": "web_app", "journey_count": 1,
                "module_count": 0},
        ], "total": 2, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "surface_list_deliverables",
    verify = "artifact-type filter reduces result set"
)]
fn the_deliverables_list_filters_by_artifact_type() {
    let g = catalog();
    assert_eq!(
        ids_of("deliverables", json!({"artifact_type": "web_app"}), &g),
        ["d2"]
    );
    // A deliverable without a status is a draft.
    assert_eq!(
        ids_of("deliverables", json!({"status": "draft"}), &g),
        ["d2"]
    );
}

#[specforge_test(
    behavior = "surface_list_deliverables",
    verify = "journey_count and module_count are accurate per entry"
)]
fn a_deliverable_entry_counts_its_journeys_and_modules() {
    let d1 = entry_of("deliverables", "d1");
    assert_eq!(
        (d1["journey_count"].clone(), d1["module_count"].clone()),
        (json!(1), json!(2))
    );
}

#[specforge_test(
    behavior = "surface_list_milestones",
    verify = "list milestones returns paginated MilestoneListResult"
)]
fn the_milestones_list_is_a_milestone_list_result() {
    let page = json_of("milestones", json!({}), &catalog());
    assert_eq!(
        page["milestones"],
        json!([
            {"id": "ms1", "title": "ms1", "status": "completed", "target_date": "2026-01-01",
                "feature_count": 2},
            {"id": "ms2", "title": "ms2", "feature_count": 0},
        ])
    );
    assert_eq!(
        (page["total"].clone(), page["has_more"].clone()),
        (json!(2), json!(false))
    );
}

#[specforge_test(
    behavior = "surface_list_milestones",
    verify = "status filter reduces result set"
)]
fn the_milestones_list_filters_by_status() {
    let g = catalog();
    // A milestone without a status is planned.
    assert_eq!(
        ids_of("milestones", json!({"status": "planned"}), &g),
        ["ms2"]
    );
    assert!(ids_of("milestones", json!({"status": "blocked"}), &g).is_empty());
}

#[specforge_test(
    behavior = "surface_list_milestones",
    verify = "feature_count is accurate per entry"
)]
fn a_milestone_entry_counts_its_features() {
    assert_eq!(entry_of("milestones", "ms1")["feature_count"], 2);
    assert_eq!(entry_of("milestones", "ms2")["feature_count"], 0);
}

#[specforge_test(
    behavior = "surface_list_modules",
    verify = "list modules returns paginated ModuleListResult"
)]
fn the_modules_list_is_a_module_list_result() {
    let page = json_of("modules", json!({}), &catalog());
    assert_eq!(
        page,
        json!({"modules": [
            {"id": "mod1", "title": "mod1", "family": "core", "feature_count": 1,
                "depends_on": ["mod2"]},
            {"id": "mod2", "title": "mod2", "family": "experimental", "feature_count": 0,
                "depends_on": []},
        ], "total": 2, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "surface_list_modules",
    verify = "family filter reduces result set"
)]
fn the_modules_list_filters_by_family() {
    assert_eq!(
        ids_of("modules", json!({"family": "core"}), &catalog()),
        ["mod1"]
    );
}

#[specforge_test(
    behavior = "surface_list_modules",
    verify = "feature_count and depends_on are accurate per entry"
)]
fn a_module_entry_counts_its_features_and_lists_its_dependencies() {
    let mod1 = entry_of("modules", "mod1");
    assert_eq!(
        (mod1["feature_count"].clone(), mod1["depends_on"].clone()),
        (json!(1), json!(["mod2"]))
    );
}

#[specforge_test(
    behavior = "surface_list_terms",
    verify = "list terms returns paginated TermListResult"
)]
fn the_terms_list_is_a_term_list_result() {
    let page = json_of("terms", json!({}), &catalog());
    assert_eq!(
        page,
        json!({"terms": [
            {"id": "t1", "title": "t1", "definition": "one", "alias_count": 2},
            {"id": "t2", "title": "t2", "definition": "two", "alias_count": 0},
        ], "total": 2, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "surface_list_terms",
    verify = "alias_count is accurate per entry"
)]
fn a_term_entry_counts_its_aliases() {
    assert_eq!(entry_of("terms", "t1")["alias_count"], 2);
    assert_eq!(entry_of("terms", "t2")["alias_count"], 0);
}

#[specforge_test(
    behavior = "surface_list_personas",
    verify = "list personas returns paginated PersonaListResult"
)]
fn the_personas_list_is_a_persona_list_result() {
    let page = json_of("personas", json!({}), &catalog());
    assert_eq!(
        page["personas"],
        json!([
            {"id": "dev", "title": "dev", "technical_level": "expert", "status": "active",
                "journey_count": 1},
            {"id": "ops", "title": "ops", "technical_level": "beginner", "journey_count": 1},
        ])
    );
    assert_eq!(page["total"], 2);
}

#[specforge_test(
    behavior = "surface_list_personas",
    verify = "technical-level filter reduces result set"
)]
fn the_personas_list_filters_by_technical_level() {
    assert_eq!(
        ids_of(
            "personas",
            json!({"technical_level": "beginner"}),
            &catalog()
        ),
        ["ops"]
    );
}

#[specforge_test(
    behavior = "surface_list_personas",
    verify = "journey_count is accurate per entry"
)]
fn a_persona_entry_counts_the_journeys_targeting_it() {
    let g = catalog().n("j4", "journey").edge("j4", "dev", "persona");
    let page = json_of("personas", json!({}), &g);
    assert_eq!(page["personas"][0]["journey_count"], 2);
    assert_eq!(page["personas"][1]["journey_count"], 1);
}

#[specforge_test(
    behavior = "surface_list_channels",
    verify = "list channels returns paginated ChannelListResult"
)]
fn the_channels_list_is_a_channel_list_result() {
    let page = json_of("channels", json!({}), &catalog());
    assert_eq!(
        page["channels"],
        json!([
            {"id": "cli", "title": "cli", "interaction_model": "batch", "status": "active",
                "journey_count": 2},
            {"id": "web", "title": "web", "interaction_model": "streaming", "journey_count": 1},
        ])
    );
    assert_eq!(page["total"], 2);
}

#[specforge_test(
    behavior = "surface_list_channels",
    verify = "interaction-model filter reduces result set"
)]
fn the_channels_list_filters_by_interaction_model() {
    assert_eq!(
        ids_of(
            "channels",
            json!({"interaction_model": "streaming"}),
            &catalog()
        ),
        ["web"]
    );
}

#[specforge_test(
    behavior = "surface_list_channels",
    verify = "journey_count is accurate per entry"
)]
fn a_channel_entry_counts_the_journeys_using_it() {
    assert_eq!(entry_of("channels", "cli")["journey_count"], 2);
    assert_eq!(entry_of("channels", "web")["journey_count"], 1);
}

#[specforge_test(
    behavior = "surface_list_releases",
    verify = "list-releases returns all releases with default pagination"
)]
fn the_releases_list_has_every_release_on_one_default_page() {
    let page = json_of("releases", json!({}), &catalog());
    assert_eq!(
        page,
        json!({"releases": [
            {"id": "r1", "title": "r1", "version": "1.0.0", "status": "released",
                "deliverable_count": 1},
            {"id": "r2", "title": "r2", "deliverable_count": 0},
        ], "total": 2, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "surface_list_releases",
    verify = "list-releases --status=released filters correctly"
)]
fn the_releases_list_filters_by_status() {
    let g = catalog();
    assert_eq!(
        ids_of("releases", json!({"status": "released"}), &g),
        ["r1"]
    );
    // A release without a status is planned.
    assert_eq!(ids_of("releases", json!({"status": "planned"}), &g), ["r2"]);
}

#[specforge_test(
    behavior = "surface_list_releases",
    verify = "list-releases --format=json returns valid JSON"
)]
fn the_releases_list_is_json_when_asked() {
    let out = run_in(&runtime(), "releases", json!({}), &catalog(), "json");
    assert!(out.json()["releases"].is_array());
    let human = run_in(&runtime(), "releases", json!({}), &catalog(), "human");
    assert!(
        human.stdout.starts_with("id  title  version  status"),
        "{}",
        human.stdout
    );
}

// ── multi-hop traceability ────────────────────────────────────────────────

/// Deliverables over journeys and modules: d1 holds j1, j2 and mod1, d2
/// only mod2, d3 nothing, d4 a journey without a persona. f1 is reached
/// both ways, f2 through journeys, f3 through modules; f4 depends on f1,
/// f5 on f4, and f6 only relates to f1.
fn shipping() -> G {
    G::default()
        .n("d1", "deliverable")
        .n("d2", "deliverable")
        .n("d3", "deliverable")
        .n("d4", "deliverable")
        .n("j1", "journey")
        .n("j2", "journey")
        .n("j3", "journey")
        .n("j5", "journey")
        .n("mod1", "module")
        .n("mod2", "module")
        .n("ms1", "milestone")
        .n("dev", "persona")
        .n("ops", "persona")
        .n("loner", "persona")
        .n("cli", "channel")
        .n("web", "channel")
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .n("f4", "feature")
        .n("f5", "feature")
        .n("f6", "feature")
        .n("f7", "feature")
        .edge("d1", "j1", "journeys")
        .edge("d1", "j2", "journeys")
        .edge("d1", "mod1", "modules")
        .edge("d2", "mod2", "modules")
        .edge("d4", "j5", "journeys")
        .edge("j1", "f1", "features")
        .edge("j1", "f2", "features")
        .edge("j2", "f2", "features")
        .edge("mod1", "f1", "features")
        .edge("mod1", "f3", "features")
        .edge("mod2", "f3", "features")
        .edge("ms1", "f1", "features")
        .edge("j1", "dev", "persona")
        .edge("j2", "dev", "persona")
        .edge("j3", "ops", "persona")
        .edge("j1", "cli", "channels")
        .edge("j1", "web", "channels")
        .edge("j2", "cli", "channels")
        .edge("j3", "web", "channels")
        .edge("f4", "f1", "depends_on")
        .edge("f5", "f4", "depends_on")
        .edge("f6", "f1", "features")
}

/// [`shipping`] with its references declared in the opposite order.
fn shipping_reversed() -> G {
    let mut g = shipping();
    g.edges.reverse();
    g.nodes.reverse();
    g
}

#[specforge_test(
    behavior = "surface_deliverable_traceability",
    verify = "deliverable-traceability returns DeliverableTraceabilityPayload JSON"
)]
fn deliverable_traceability_answers_its_payload() {
    let dt = json_of(
        "deliverable_traceability",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    assert_eq!(
        dt,
        json!({"deliverable_id": "d1", "transitive_features": ["f1", "f2", "f3"],
            "journey_path_count": 2, "module_path_count": 2})
    );
}

#[specforge_test(
    behavior = "surface_deliverable_traceability",
    verify = "missing deliverable ID returns error with suggestion"
)]
fn deliverable_traceability_of_a_mistyped_deliverable_suggests_the_nearest() {
    let error = RUNTIME.with(|runtime| {
        run_in(
            runtime,
            "deliverable_traceability",
            json!({"deliverable": "dd1"}),
            &shipping(),
            "json",
        )
        .error()
    });
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "d1");
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable with journeys and modules returns union of features"
)]
fn a_deliverables_features_are_its_journeys_and_its_modules() {
    let dt = json_of(
        "deliverable_traceability",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    assert_eq!(dt["transitive_features"], json!(["f1", "f2", "f3"]));
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable with only journeys returns journey features"
)]
fn a_deliverable_with_only_journeys_has_their_features() {
    let g = shipping()
        .n("d5", "deliverable")
        .edge("d5", "j2", "journeys");
    let dt = json_of("deliverable_traceability", json!({"deliverable": "d5"}), &g);
    assert_eq!(
        (
            dt["transitive_features"].clone(),
            dt["module_path_count"].clone()
        ),
        (json!(["f2"]), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable with no journeys or modules returns empty feature set"
)]
fn a_deliverable_with_nothing_reaches_no_feature() {
    let dt = json_of(
        "deliverable_traceability",
        json!({"deliverable": "d3"}),
        &shipping(),
    );
    assert_eq!(
        dt,
        json!({"deliverable_id": "d3", "transitive_features": [], "journey_path_count": 0,
            "module_path_count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable with overlapping journey and module features deduplicates"
)]
fn a_feature_reached_both_ways_is_listed_once() {
    let dt = json_of(
        "deliverable_traceability",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    // f1 is on both paths: counted on each, listed once.
    let features = dt["transitive_features"].as_array().unwrap();
    assert_eq!(features.iter().filter(|f| **f == "f1").count(), 1);
    assert_eq!(
        dt["journey_path_count"].as_u64().unwrap() + dt["module_path_count"].as_u64().unwrap(),
        features.len() as u64 + 1
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable traceability is deterministic across repeated queries"
)]
fn deliverable_traceability_is_deterministic() {
    let args = json!({"deliverable": "d1"});
    let first = json_of("deliverable_traceability", args.clone(), &shipping());
    assert_eq!(
        json_of("deliverable_traceability", args.clone(), &shipping()),
        first
    );
    assert_eq!(
        json_of("deliverable_traceability", args, &shipping_reversed()),
        first
    );
}

#[specforge_test(
    behavior = "surface_feature_deliverables",
    verify = "feature-deliverables returns FeatureDeliverablePayload JSON"
)]
fn feature_deliverables_answers_its_payload() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f1"}),
        &shipping(),
    );
    assert_eq!(
        fd,
        json!({"feature_id": "f1", "deliverables": ["d1"], "via_journey_count": 1,
            "via_module_count": 1})
    );
}

#[specforge_test(
    behavior = "surface_feature_deliverables",
    verify = "missing feature ID returns error with suggestion"
)]
fn feature_deliverables_of_a_mistyped_feature_suggests_the_nearest() {
    let error = RUNTIME.with(|runtime| {
        run_in(
            runtime,
            "feature_deliverables",
            json!({"feature": "f11"}),
            &shipping(),
            "json",
        )
        .error()
    });
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "f1");
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature reachable via journey path returns deliverable"
)]
fn a_feature_in_a_deliverables_journey_is_in_the_deliverable() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f2"}),
        &shipping(),
    );
    assert_eq!(
        (fd["deliverables"].clone(), fd["via_journey_count"].clone()),
        (json!(["d1"]), json!(1))
    );
    assert_eq!(fd["via_module_count"], 0);
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature reachable via module path returns deliverable"
)]
fn a_feature_in_a_deliverables_module_is_in_the_deliverable() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f3"}),
        &shipping(),
    );
    assert_eq!(
        (fd["deliverables"].clone(), fd["via_module_count"].clone()),
        (json!(["d1", "d2"]), json!(2))
    );
    assert_eq!(fd["via_journey_count"], 0);
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature reachable via both paths deduplicates deliverables"
)]
fn a_deliverable_reached_both_ways_is_listed_once() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f1"}),
        &shipping(),
    );
    assert_eq!(fd["deliverables"], json!(["d1"]));
    assert_eq!(
        (
            fd["via_journey_count"].clone(),
            fd["via_module_count"].clone()
        ),
        (json!(1), json!(1))
    );
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature with no incoming edges returns empty deliverables"
)]
fn a_feature_nothing_holds_is_in_no_deliverable() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f7"}),
        &shipping(),
    );
    assert_eq!(
        fd,
        json!({"feature_id": "f7", "deliverables": [], "via_journey_count": 0,
            "via_module_count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature deliverable query is deterministic across repeated queries"
)]
fn feature_deliverables_is_deterministic() {
    let args = json!({"feature": "f3"});
    let first = json_of("feature_deliverables", args.clone(), &shipping());
    assert_eq!(
        json_of("feature_deliverables", args.clone(), &shipping()),
        first
    );
    assert_eq!(
        json_of("feature_deliverables", args, &shipping_reversed()),
        first
    );
}

#[specforge_test(
    behavior = "surface_persona_channels",
    verify = "persona-channels returns PersonaChannelPayload JSON"
)]
fn persona_channels_answers_its_payload() {
    let pc = json_of("persona_channels", json!({"persona": "dev"}), &shipping());
    assert_eq!(
        pc,
        json!({"persona_id": "dev", "channels": ["cli", "web"], "count": 2})
    );
}

#[specforge_test(
    behavior = "surface_persona_channels",
    verify = "missing persona ID returns error with suggestion"
)]
fn persona_channels_of_a_mistyped_persona_suggests_the_nearest() {
    let error = RUNTIME.with(|runtime| {
        run_in(
            runtime,
            "persona_channels",
            json!({"persona": "dve"}),
            &shipping(),
            "json",
        )
        .error()
    });
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "dev");
}

#[specforge_test(
    behavior = "pe_query_persona_channels",
    verify = "persona with one journey returns that journey's channels"
)]
fn a_personas_one_journey_gives_its_channels() {
    let pc = json_of("persona_channels", json!({"persona": "ops"}), &shipping());
    assert_eq!(pc["channels"], json!(["web"]));
}

#[specforge_test(
    behavior = "pe_query_persona_channels",
    verify = "persona with multiple journeys returns deduplicated channels"
)]
fn a_personas_journeys_give_each_channel_once() {
    // j1 and j2 both use cli.
    let pc = json_of("persona_channels", json!({"persona": "dev"}), &shipping());
    assert_eq!(
        (pc["channels"].clone(), pc["count"].clone()),
        (json!(["cli", "web"]), json!(2))
    );
}

#[specforge_test(
    behavior = "pe_query_persona_channels",
    verify = "persona with no journeys returns empty channel list"
)]
fn a_persona_without_journeys_uses_no_channel() {
    let pc = json_of("persona_channels", json!({"persona": "loner"}), &shipping());
    assert_eq!(
        pc,
        json!({"persona_id": "loner", "channels": [], "count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_persona_channels",
    verify = "persona channels query is deterministic across repeated queries"
)]
fn persona_channels_is_deterministic() {
    let args = json!({"persona": "dev"});
    let first = json_of("persona_channels", args.clone(), &shipping());
    assert_eq!(
        json_of("persona_channels", args.clone(), &shipping()),
        first
    );
    assert_eq!(
        json_of("persona_channels", args, &shipping_reversed()),
        first
    );
}

#[specforge_test(
    behavior = "surface_deliverable_personas",
    verify = "deliverable-personas returns DeliverablePersonaPayload JSON"
)]
fn deliverable_personas_answers_its_payload() {
    let dp = json_of(
        "deliverable_personas",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    assert_eq!(
        dp,
        json!({"deliverable_id": "d1", "personas": ["dev"], "via_journey_ids": ["j1", "j2"],
            "count": 1})
    );
}

#[specforge_test(
    behavior = "surface_deliverable_personas",
    verify = "missing deliverable ID returns error with suggestion"
)]
fn deliverable_personas_of_a_mistyped_deliverable_suggests_the_nearest() {
    let error = RUNTIME.with(|runtime| {
        run_in(
            runtime,
            "deliverable_personas",
            json!({"deliverable": "d9"}),
            &shipping(),
            "json",
        )
        .error()
    });
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "d1");
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "deliverable with one journey and one persona returns that persona"
)]
fn a_deliverables_journey_gives_its_persona() {
    let g = shipping()
        .n("d6", "deliverable")
        .edge("d6", "j3", "journeys");
    let dp = json_of("deliverable_personas", json!({"deliverable": "d6"}), &g);
    assert_eq!(
        (dp["personas"].clone(), dp["count"].clone()),
        (json!(["ops"]), json!(1))
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "deliverable with multiple journeys sharing a persona deduplicates"
)]
fn a_persona_shared_by_journeys_is_listed_once() {
    let dp = json_of(
        "deliverable_personas",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    assert_eq!(dp["personas"], json!(["dev"]));
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "deliverable with no journeys returns empty list"
)]
fn a_deliverable_without_journeys_serves_no_persona() {
    let dp = json_of(
        "deliverable_personas",
        json!({"deliverable": "d2"}),
        &shipping(),
    );
    assert_eq!(
        dp,
        json!({"deliverable_id": "d2", "personas": [], "via_journey_ids": [], "count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "deliverable with journeys without personas returns empty list"
)]
fn a_deliverable_whose_journeys_target_no_one_serves_no_persona() {
    let dp = json_of(
        "deliverable_personas",
        json!({"deliverable": "d4"}),
        &shipping(),
    );
    assert_eq!(
        (dp["personas"].clone(), dp["count"].clone()),
        (json!([]), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "result includes via_journey_ids for traceability"
)]
fn deliverable_personas_name_the_journeys_between() {
    let g = shipping().edge("d1", "j3", "journeys");
    let dp = json_of("deliverable_personas", json!({"deliverable": "d1"}), &g);
    assert_eq!(dp["personas"], json!(["dev", "ops"]));
    assert_eq!(dp["via_journey_ids"], json!(["j1", "j2", "j3"]));
}

#[specforge_test(
    behavior = "product_deliverable_persona_correctness",
    verify = "via_journey_ids includes all intermediate journeys"
)]
fn every_journey_between_a_deliverable_and_a_persona_is_named() {
    // d1's journeys j1 and j2 both reach dev; j5 (on d4) reaches no one.
    let g = shipping().edge("d1", "j5", "journeys");
    let dp = json_of("deliverable_personas", json!({"deliverable": "d1"}), &g);
    assert_eq!(dp["via_journey_ids"], json!(["j1", "j2"]));
}

#[specforge_test(
    behavior = "product_deliverable_persona_correctness",
    verify = "traversal order does not affect result"
)]
fn deliverable_personas_do_not_depend_on_declaration_order() {
    let args = json!({"deliverable": "d1"});
    assert_eq!(
        json_of("deliverable_personas", args.clone(), &shipping_reversed()),
        json_of("deliverable_personas", args, &shipping())
    );
}

// ── feature impact ────────────────────────────────────────────────────────

#[specforge_test(
    behavior = "surface_feature_impact",
    verify = "feature-impact returns FeatureImpactPayload JSON"
)]
fn feature_impact_answers_its_payload() {
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &shipping());
    assert_eq!(
        fi,
        json!({"feature_id": "f1", "affected_journeys": ["j1"], "affected_milestones": ["ms1"],
            "affected_deliverables": ["d1"], "affected_modules": ["mod1"],
            "dependent_features": ["f4", "f5"], "total_affected_entities": 6})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "feature in one journey and one milestone returns both as affected"
)]
fn a_feature_in_a_journey_and_a_milestone_affects_both() {
    let g = G::default()
        .n("f1", "feature")
        .n("j1", "journey")
        .n("ms1", "milestone")
        .edge("j1", "f1", "features")
        .edge("ms1", "f1", "features");
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &g);
    assert_eq!(fi["affected_journeys"], json!(["j1"]));
    assert_eq!(fi["affected_milestones"], json!(["ms1"]));
    assert_eq!(fi["total_affected_entities"], 2);
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "feature with dependent features includes transitive dependents"
)]
fn a_features_dependents_include_their_dependents() {
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &shipping());
    // f5 depends on f4, which depends on f1; f6 only relates to f1.
    assert_eq!(fi["dependent_features"], json!(["f4", "f5"]));
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "feature with no references returns zero affected entities"
)]
fn a_feature_nothing_references_affects_nothing() {
    let fi = json_of("feature_impact", json!({"feature": "f7"}), &shipping());
    assert_eq!(
        fi,
        json!({"feature_id": "f7", "affected_journeys": [], "affected_milestones": [],
            "affected_deliverables": [], "affected_modules": [], "dependent_features": [],
            "total_affected_entities": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "affected deliverables found via both journey and module paths"
)]
fn a_features_deliverables_come_through_journeys_and_modules() {
    // d7 holds f3 only through its journey j2, d2 only through mod2.
    let g = shipping()
        .n("d7", "deliverable")
        .edge("d7", "j2", "journeys")
        .edge("j2", "f3", "features");
    let fi = json_of("feature_impact", json!({"feature": "f3"}), &g);
    assert_eq!(fi["affected_deliverables"], json!(["d1", "d2", "d7"]));
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "total_affected_entities is deduplicated count"
)]
fn the_total_counts_each_affected_entity_once() {
    // d1 holds f1 through j1 and through mod1: one deliverable.
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &shipping());
    let listed: usize = [
        "affected_journeys",
        "affected_milestones",
        "affected_deliverables",
        "affected_modules",
        "dependent_features",
    ]
    .iter()
    .map(|k| fi[*k].as_array().unwrap().len())
    .sum();
    assert_eq!(fi["total_affected_entities"], listed);
    assert_eq!(fi["affected_deliverables"], json!(["d1"]));
}

#[specforge_test(
    behavior = "product_impact_query_correctness",
    verify = "feature impact returns complete transitive closure"
)]
fn feature_impact_follows_dependencies_to_the_end() {
    // A chain f1 <- c1 <- c2 <- c3, and a cycle back to c1.
    let g = shipping()
        .n("c1", "feature")
        .n("c2", "feature")
        .n("c3", "feature")
        .edge("c1", "f1", "depends_on")
        .edge("c2", "c1", "depends_on")
        .edge("c3", "c2", "depends_on")
        .edge("c1", "c3", "depends_on");
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &g);
    assert_eq!(
        fi["dependent_features"],
        json!(["c1", "c2", "c3", "f4", "f5"])
    );
}

#[specforge_test(
    behavior = "product_impact_query_correctness",
    verify = "traversal order does not affect results"
)]
fn impact_and_persona_features_do_not_depend_on_declaration_order() {
    for (id, args) in [
        ("feature_impact", json!({"feature": "f1"})),
        ("persona_features", json!({"persona": "dev"})),
    ] {
        assert_eq!(
            json_of(id, args.clone(), &shipping_reversed()),
            json_of(id, args, &shipping()),
            "{id}"
        );
    }
}

// ── status and progress rollups ───────────────────────────────────────────

/// Deliverables tracked by milestones, a release over them, features some
/// milestone schedules and owners:
/// - d1: ms1 (completed, critical), ms2 (completed); shipped, owner bo
/// - d2: ms2, ms3 (in_progress, medium); j1 (high); shipped
/// - d3: nothing; no status (draft)
/// - d4: ms4 (no status); j2
/// - d5: j1 (high) and five constituents without a priority
/// - r1: d1, d2, d3; owner bo. r2: nothing
/// - f1 (ms1), f2 (ms3) scheduled; f3 (only a journey's), f4 not
fn progress() -> G {
    G::default()
        .node(
            "d1",
            "deliverable",
            json!({"status": "shipped", "owner": "bo"}),
        )
        .node("d2", "deliverable", json!({"status": "shipped"}))
        .n("d3", "deliverable")
        .n("d4", "deliverable")
        .n("d5", "deliverable")
        .node(
            "ms1",
            "milestone",
            json!({"status": "completed", "priority": "critical", "owner": "al"}),
        )
        .node("ms2", "milestone", json!({"status": "completed"}))
        .node(
            "ms3",
            "milestone",
            json!({"status": "in_progress", "priority": "medium"}),
        )
        .n("ms4", "milestone")
        .node("j1", "journey", json!({"priority": "high"}))
        .n("j2", "journey")
        .n("j3", "journey")
        .n("j4", "journey")
        .node("r1", "release", json!({"owner": "bo"}))
        .n("r2", "release")
        .node("f1", "feature", json!({"status": "done", "owner": "al"}))
        .node("f2", "feature", json!({"owner": "al"}))
        .node(
            "f3",
            "feature",
            json!({"status": "accepted", "owner": "cy"}),
        )
        .n("f4", "feature")
        .edge("d1", "ms1", "milestones")
        .edge("d1", "ms2", "milestones")
        .edge("d2", "ms2", "milestones")
        .edge("d2", "ms3", "milestones")
        .edge("d2", "j1", "journeys")
        .edge("d4", "ms4", "milestones")
        .edge("d4", "j2", "journeys")
        .edge("d5", "j1", "journeys")
        .edge("d5", "j2", "journeys")
        .edge("d5", "j3", "journeys")
        .edge("d5", "j4", "journeys")
        .edge("d5", "ms2", "milestones")
        .edge("d5", "ms4", "milestones")
        .edge("r1", "d1", "deliverables")
        .edge("r1", "d2", "deliverables")
        .edge("r1", "d3", "deliverables")
        .edge("ms1", "f1", "features")
        .edge("ms3", "f2", "features")
        .edge("j1", "f3", "features")
}

/// [`progress`] with its references declared in the opposite order.
fn progress_reversed() -> G {
    let mut g = progress();
    g.edges.reverse();
    g.nodes.reverse();
    g
}

fn not_found_suggesting(id: &str, args: Value, g: &G) -> Value {
    let error = RUNTIME.with(|runtime| run_in(runtime, id, args, g, "json").error());
    assert_eq!(error["code"], "ENTITY_NOT_FOUND", "{error}");
    error
}

#[specforge_test(
    behavior = "surface_deliverable_completion",
    verify = "deliverable-completion returns DeliverableCompletionPayload JSON"
)]
fn deliverable_completion_answers_its_payload() {
    let dc = json_of(
        "deliverable_completion",
        json!({"deliverable": "d2"}),
        &progress(),
    );
    assert_eq!(
        dc,
        json!({"deliverable_id": "d2", "milestone_count": 2, "completed_count": 1,
            "completion_ratio": 0.5})
    );
    // --details adds each milestone's own completion.
    let dc = json_of(
        "deliverable_completion",
        json!({"deliverable": "d2", "details": true}),
        &progress(),
    );
    assert_eq!(
        dc["milestone_details"],
        json!([
            {"milestone_id": "ms2", "total_features": 0, "done_count": 0,
                "completion_ratio": 0.0, "done_features": []},
            {"milestone_id": "ms3", "total_features": 1, "done_count": 0,
                "completion_ratio": 0.0, "done_features": []},
        ])
    );
    let human = human_of(
        "deliverable_completion",
        json!({"deliverable": "d2", "details": true}),
        &progress(),
    );
    assert!(
        human.contains("Completion: 50% (1/2 milestones completed)"),
        "{human}"
    );
    assert!(human.contains("milestone  status"), "{human}");
}

#[specforge_test(
    behavior = "surface_deliverable_completion",
    verify = "missing deliverable ID returns error with suggestion"
)]
fn deliverable_completion_of_a_mistyped_deliverable_suggests_the_nearest() {
    let error = not_found_suggesting(
        "deliverable_completion",
        json!({"deliverable": "dd1"}),
        &progress(),
    );
    assert_eq!(error["suggestion"], "d1");
}

#[specforge_test(
    behavior = "pe_query_deliverable_completion",
    verify = "deliverable with all completed milestones returns ratio 1.0"
)]
fn a_deliverable_whose_milestones_are_all_completed_is_complete() {
    let dc = json_of(
        "deliverable_completion",
        json!({"deliverable": "d1"}),
        &progress(),
    );
    assert_eq!(
        (
            dc["completed_count"].clone(),
            dc["completion_ratio"].clone()
        ),
        (json!(2), json!(1.0))
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_completion",
    verify = "deliverable with no completed milestones returns ratio 0.0"
)]
fn a_deliverable_without_a_completed_milestone_is_at_zero() {
    // ms4 has no status: it is planned, not completed.
    let dc = json_of(
        "deliverable_completion",
        json!({"deliverable": "d4"}),
        &progress(),
    );
    assert_eq!(
        (dc["milestone_count"].clone(), dc["completed_count"].clone()),
        (json!(1), json!(0))
    );
    assert_eq!(dc["completion_ratio"], 0.0);
}

#[specforge_test(
    behavior = "pe_query_deliverable_completion",
    verify = "deliverable with zero milestones returns ratio 0.0"
)]
fn a_deliverable_without_milestones_is_at_zero_without_dividing_by_zero() {
    let dc = json_of(
        "deliverable_completion",
        json!({"deliverable": "d3"}),
        &progress(),
    );
    assert_eq!(
        dc,
        json!({"deliverable_id": "d3", "milestone_count": 0, "completed_count": 0,
            "completion_ratio": 0.0})
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_completion",
    verify = "deliverable with mix of completed and non-completed milestones returns partial ratio"
)]
fn a_deliverable_half_completed_is_at_one_half() {
    let dc = json_of(
        "deliverable_completion",
        json!({"deliverable": "d2"}),
        &progress(),
    );
    assert_eq!(dc["completion_ratio"], 0.5);
}

#[specforge_test(
    behavior = "pe_query_deliverable_completion",
    verify = "deliverable completion is deterministic across repeated queries"
)]
fn deliverable_completion_is_the_same_every_time_and_in_any_order() {
    let args = json!({"deliverable": "d2", "details": true});
    let first = json_of("deliverable_completion", args.clone(), &progress());
    assert_eq!(
        json_of("deliverable_completion", args.clone(), &progress()),
        first
    );
    assert_eq!(
        json_of("deliverable_completion", args, &progress_reversed()),
        first
    );
}

#[specforge_test(
    behavior = "surface_release_completion",
    verify = "release-completion returns correct shipped/total ratio"
)]
fn release_completion_answers_shipped_over_total() {
    let rc = json_of("release_completion", json!({"release": "r1"}), &progress());
    assert_eq!(
        rc,
        json!({"release_id": "r1", "total": 3, "shipped": 2, "completion_ratio": 2.0 / 3.0})
    );
    let human = human_of("release_completion", json!({"release": "r1"}), &progress());
    assert!(
        human.contains("Completion: 67% (2/3 deliverables shipped)"),
        "{human}"
    );
    let error = not_found_suggesting("release_completion", json!({"release": "r9"}), &progress());
    assert_eq!(error["suggestion"], "r1");
}

#[specforge_test(
    behavior = "pe_query_release_completion",
    verify = "release with 2/3 shipped returns ratio 0.667"
)]
fn a_release_with_two_of_three_shipped_is_at_two_thirds() {
    // d3 has no status: it is a draft, not shipped.
    let rc = json_of("release_completion", json!({"release": "r1"}), &progress());
    let ratio = rc["completion_ratio"].as_f64().unwrap();
    assert!((ratio - 0.667).abs() < 0.001, "{ratio}");
}

#[specforge_test(
    behavior = "pe_query_release_completion",
    verify = "release with no deliverables returns null ratio"
)]
fn a_release_without_deliverables_has_no_ratio() {
    let rc = json_of("release_completion", json!({"release": "r2"}), &progress());
    assert_eq!(
        rc,
        json!({"release_id": "r2", "total": 0, "shipped": 0, "completion_ratio": null})
    );
}

#[specforge_test(
    behavior = "surface_deliverable_priority",
    verify = "deliverable-priority returns DeliverablePriorityPayload JSON"
)]
fn deliverable_priority_answers_its_payload() {
    let dp = json_of(
        "deliverable_priority",
        json!({"deliverable": "d1"}),
        &progress(),
    );
    assert_eq!(
        dp,
        json!({"deliverable_id": "d1", "priority": "critical", "source_count": 1})
    );
    // With nothing to derive it from, the priority is null.
    let dp = json_of(
        "deliverable_priority",
        json!({"deliverable": "d3"}),
        &progress(),
    );
    assert_eq!(dp["priority"], Value::Null);
}

#[specforge_test(
    behavior = "surface_deliverable_priority",
    verify = "missing deliverable ID returns error with suggestion"
)]
fn deliverable_priority_of_a_mistyped_deliverable_suggests_the_nearest() {
    let error = not_found_suggesting(
        "deliverable_priority",
        json!({"deliverable": "d22"}),
        &progress(),
    );
    assert_eq!(error["suggestion"], "d2");
}

#[specforge_test(
    behavior = "pe_query_deliverable_priority",
    verify = "deliverable with critical milestone returns critical priority"
)]
fn a_critical_milestone_makes_its_deliverable_critical() {
    let dp = json_of(
        "deliverable_priority",
        json!({"deliverable": "d1"}),
        &progress(),
    );
    assert_eq!(dp["priority"], "critical");
}

#[specforge_test(
    behavior = "pe_query_deliverable_priority",
    verify = "deliverable with high journey and medium milestone returns high priority"
)]
fn a_high_journey_outranks_a_medium_milestone() {
    let dp = json_of(
        "deliverable_priority",
        json!({"deliverable": "d2"}),
        &progress(),
    );
    assert_eq!(
        (dp["priority"].clone(), dp["source_count"].clone()),
        (json!("high"), json!(2))
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_priority",
    verify = "deliverable with no milestones and no journeys returns null priority"
)]
fn a_deliverable_with_no_constituents_has_no_priority() {
    let dp = json_of(
        "deliverable_priority",
        json!({"deliverable": "d3"}),
        &progress(),
    );
    assert_eq!(
        dp,
        json!({"deliverable_id": "d3", "priority": null, "source_count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_priority",
    verify = "deliverable where all constituents have null priority returns null priority"
)]
fn a_deliverable_whose_constituents_declare_no_priority_has_none() {
    // d4's ms4 and j2 declare none; none is not medium.
    let dp = json_of(
        "deliverable_priority",
        json!({"deliverable": "d4"}),
        &progress(),
    );
    assert_eq!(
        (dp["priority"].clone(), dp["source_count"].clone()),
        (Value::Null, json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_priority",
    verify = "deliverable with one prioritized and five unprioritized returns the one priority"
)]
fn one_declared_priority_among_six_constituents_is_the_priority() {
    let dp = json_of(
        "deliverable_priority",
        json!({"deliverable": "d5"}),
        &progress(),
    );
    assert_eq!(dp["priority"], "high");
}

#[specforge_test(
    behavior = "pe_query_deliverable_priority",
    verify = "source_count counts only entities with explicit priority"
)]
fn the_source_count_is_the_constituents_with_a_priority() {
    // d5 has six constituents; only j1 declares a priority.
    let dp = json_of(
        "deliverable_priority",
        json!({"deliverable": "d5"}),
        &progress(),
    );
    assert_eq!(dp["source_count"], 1);
}

#[specforge_test(
    behavior = "pe_query_deliverable_priority",
    verify = "deliverable priority is deterministic across repeated queries"
)]
fn deliverable_priority_is_the_same_every_time_and_in_any_order() {
    for d in ["d1", "d2", "d5"] {
        let args = json!({ "deliverable": d });
        let first = json_of("deliverable_priority", args.clone(), &progress());
        assert_eq!(
            json_of("deliverable_priority", args.clone(), &progress()),
            first
        );
        assert_eq!(
            json_of("deliverable_priority", args, &progress_reversed()),
            first
        );
    }
}

#[specforge_test(
    behavior = "surface_unscheduled_features",
    verify = "product:unscheduled-features returns UnscheduledFeaturesPayload"
)]
fn unscheduled_features_answers_its_payload() {
    let uf = json_of("unscheduled_features", json!({}), &progress());
    assert_eq!(
        uf,
        json!({"features": ["f3", "f4"], "count": 2, "total_features": 4, "scheduled_count": 2})
    );
    let human = human_of("unscheduled_features", json!({}), &progress());
    assert_eq!(
        human,
        "id  status\nf3  accepted\nf4  -\n2 of 4 features unscheduled\n"
    );
}

#[specforge_test(
    behavior = "surface_unscheduled_features",
    verify = "no unscheduled features returns empty list"
)]
fn a_plan_with_every_feature_scheduled_has_none_unscheduled() {
    let g = progress()
        .edge("ms2", "f3", "features")
        .edge("ms2", "f4", "features");
    let uf = json_of("unscheduled_features", json!({}), &g);
    assert_eq!(
        (uf["features"].clone(), uf["count"].clone()),
        (json!([]), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_unscheduled_features",
    verify = "feature in no milestone appears in unscheduled list"
)]
fn a_feature_in_no_milestone_is_unscheduled() {
    // f3 is a journey's feature, but no milestone's.
    let uf = json_of("unscheduled_features", json!({}), &progress());
    assert!(uf["features"].as_array().unwrap().contains(&json!("f3")));
    assert!(uf["features"].as_array().unwrap().contains(&json!("f4")));
}

#[specforge_test(
    behavior = "pe_query_unscheduled_features",
    verify = "feature in one milestone is excluded from unscheduled list"
)]
fn a_feature_in_a_milestone_is_scheduled() {
    let uf = json_of("unscheduled_features", json!({}), &progress());
    assert!(!uf["features"].as_array().unwrap().contains(&json!("f1")));
    assert!(!uf["features"].as_array().unwrap().contains(&json!("f2")));
}

#[specforge_test(
    behavior = "pe_query_unscheduled_features",
    verify = "empty graph returns empty list"
)]
fn an_empty_graph_has_nothing_unscheduled() {
    let uf = json_of("unscheduled_features", json!({}), &G::default());
    assert_eq!(
        uf,
        json!({"features": [], "count": 0, "total_features": 0, "scheduled_count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_unscheduled_features",
    verify = "count + scheduled_count == total_features"
)]
fn unscheduled_and_scheduled_add_up_to_every_feature() {
    for g in [progress(), plan(), shipping(), G::default()] {
        let uf = json_of("unscheduled_features", json!({}), &g);
        let count = uf["count"].as_u64().unwrap();
        assert_eq!(count, uf["features"].as_array().unwrap().len() as u64);
        assert_eq!(
            count + uf["scheduled_count"].as_u64().unwrap(),
            uf["total_features"].as_u64().unwrap()
        );
    }
}

#[specforge_test(
    behavior = "product_new_query_correctness",
    verify = "unscheduled features have zero MilestoneDeliversFeature edges"
)]
fn an_unscheduled_feature_is_one_no_milestone_lists() {
    // A journey or module listing a feature does not schedule it; a
    // milestone does.
    let g = progress()
        .n("mod1", "module")
        .edge("mod1", "f4", "features");
    let uf = json_of("unscheduled_features", json!({}), &g);
    assert_eq!(uf["features"], json!(["f3", "f4"]));
    let g = g.edge("ms4", "f4", "features");
    let uf = json_of("unscheduled_features", json!({}), &g);
    assert_eq!(uf["features"], json!(["f3"]));
}

#[specforge_test(
    behavior = "surface_owner_workload",
    verify = "owner-workload returns grouped ownership statistics"
)]
fn owner_workload_groups_entities_by_owner() {
    let ow = json_of("owner_workload", json!({}), &progress());
    assert_eq!(
        ow,
        json!({
            "owners": [
                {"owner": "al", "entity_ids": ["f1", "f2", "ms1"], "entity_count": 3,
                    "by_kind": {"features": 2, "milestones": 1, "deliverables": 0, "releases": 0}},
                {"owner": "bo", "entity_ids": ["d1", "r1"], "entity_count": 2,
                    "by_kind": {"features": 0, "milestones": 0, "deliverables": 1, "releases": 1}},
                {"owner": "cy", "entity_ids": ["f3"], "entity_count": 1,
                    "by_kind": {"features": 1, "milestones": 0, "deliverables": 0, "releases": 0}},
            ],
            "unowned_count": 9, "total_entities": 15,
            "total": 3, "offset": 0, "limit": 100, "has_more": false,
        })
    );
    // One page of owners, per the shared offset/limit contract.
    let page = json_of(
        "owner_workload",
        json!({"offset": 1, "limit": 1}),
        &progress(),
    );
    assert_eq!(page["owners"][0]["owner"], "bo");
    assert_eq!(
        (page["total"].clone(), page["has_more"].clone()),
        (json!(3), json!(true))
    );
    assert_eq!(page["unowned_count"], 9);
    let human = human_of("owner_workload", json!({}), &progress());
    assert!(
        human
            .starts_with("owner  entities  features  milestones  deliverables  releases\nal     3"),
        "{human}"
    );
}

#[specforge_test(
    behavior = "surface_owner_workload",
    verify = "owner-workload reports unowned entities"
)]
fn owner_workload_counts_the_unowned() {
    let ow = json_of("owner_workload", json!({}), &progress());
    assert_eq!(ow["unowned_count"], 9);
    let human = human_of("owner_workload", json!({}), &progress());
    assert!(human.contains("Unowned: 9 of 15 entities"), "{human}");
}

#[specforge_test(
    behavior = "pe_query_owner_workload",
    verify = "single owner across multiple kinds returns correct breakdown"
)]
fn one_owner_across_kinds_is_broken_down_by_kind() {
    let g = G::default()
        .node("f1", "feature", json!({"owner": "al"}))
        .node("ms1", "milestone", json!({"owner": "al"}))
        .node("d1", "deliverable", json!({"owner": "al"}))
        .node("r1", "release", json!({"owner": "al"}))
        .node("r2", "release", json!({"owner": "al"}));
    let ow = json_of("owner_workload", json!({}), &g);
    assert_eq!(ow["owners"].as_array().unwrap().len(), 1);
    assert_eq!(
        ow["owners"][0]["by_kind"],
        json!({"features": 1, "milestones": 1, "deliverables": 1, "releases": 2})
    );
    assert_eq!(ow["owners"][0]["entity_count"], 5);
}

#[specforge_test(
    behavior = "pe_query_owner_workload",
    verify = "entities without owner contribute to unowned_count"
)]
fn an_entity_without_an_owner_is_unowned() {
    // A module has no owner field: it is not counted at all.
    let g = G::default()
        .node("f1", "feature", json!({"owner": "al"}))
        .n("f2", "feature")
        .node("ms1", "milestone", json!({"owner": ""}))
        .n("mod1", "module");
    let ow = json_of("owner_workload", json!({}), &g);
    assert_eq!(
        (ow["unowned_count"].clone(), ow["total_entities"].clone()),
        (json!(2), json!(3))
    );
    let owned: u64 = ow["owners"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["entity_count"].as_u64().unwrap())
        .sum();
    assert_eq!(owned + 2, 3);
}

#[specforge_test(
    behavior = "pe_query_owner_workload",
    verify = "empty graph returns zero totals"
)]
fn an_empty_graph_has_no_workload() {
    let ow = json_of("owner_workload", json!({}), &G::default());
    assert_eq!(
        ow,
        json!({"owners": [], "unowned_count": 0, "total_entities": 0,
            "total": 0, "offset": 0, "limit": 100, "has_more": false})
    );
}

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
    let out = RUNTIME.with(|runtime| {
        run_in(
            runtime,
            "feature_ordering",
            json!({}),
            &layered_with_a_cycle(),
            "json",
        )
    });
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
    let out = RUNTIME.with(|runtime| run_in(runtime, "critical_path", json!({}), &g, "json"));
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
    let error = RUNTIME.with(|runtime| {
        run_in(
            runtime,
            "module_coupling",
            json!({"offset": -1}),
            &layers(),
            "json",
        )
    });
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
            {"kind": "deliverable", "total": 1, "by_status": [{"status": "(none)", "count": 1}]},
            {"kind": "persona", "total": 2, "by_status": [{"status": "(none)", "count": 2}]},
            {"kind": "channel", "total": 2, "by_status": [{"status": "(none)", "count": 2}]},
            {"kind": "release", "total": 1, "by_status": [{"status": "(none)", "count": 1}]},
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
        "kind         status       count\n\
         feature      (none)       1\n\
         feature      done         1\n\
         feature      in_progress  1\n\
         milestone    (none)       2\n\
         milestone    active       1\n\
         deliverable  (none)       1\n\
         persona      (none)       2\n\
         channel      (none)       2\n\
         release      (none)       1\n"
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
    ("deliverable_traceability", "deliverable", "d1"),
    ("feature_deliverables", "feature", "f1"),
    ("persona_channels", "persona", "dev"),
    ("deliverable_personas", "deliverable", "d1"),
    ("deliverable_completion", "deliverable", "d1"),
    ("release_completion", "release", "r1"),
    ("deliverable_priority", "deliverable", "d1"),
    ("module_depth", "module", "mod1"),
    ("deliverable_dependents", "deliverable", "d1"),
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
        "unscheduled_features",
        "owner_workload",
        "feature_ordering",
        "critical_path",
        "module_coupling",
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
