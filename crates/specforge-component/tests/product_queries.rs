//! `@specforge/product`'s commands not yet moved to `extensions/product/src/tests/`
//! (plan 13): the builtin component's `cmd__product_*` exports, each given a
//! `CommandInput` over a graph built here, in the format asked for (ADR 0011).
//! What each answers, and how one that cannot answer fails.

use serde_json::{Value, json};
use specforge_component::{ComponentRuntime, builtins};
use specforge_protocol_types::{CommandEvidence, CommandFormat, CommandInput, RawGraph};
use specforge_test::prelude::*;
use specforge_wasm::ExtensionCalls;
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
    run_with(runtime, id, args, g, format, None)
}

/// [`run_in`], with `evidence` as the input's recorded-test evidence.
fn run_with(
    runtime: &ComponentRuntime,
    id: &str,
    args: Value,
    g: &G,
    format: &str,
    evidence: Option<Value>,
) -> Out {
    let evidence = evidence.map_or(CommandEvidence::None, |evidence| {
        serde_json::from_value(evidence).expect("evidence in the host's wire shape")
    });
    run_input(runtime, id, args, g, format, "2026-10-03", evidence)
}

/// Run `cmd__product_<id>` as the host calls it (`ExtensionCalls::run_command`:
/// the typed input, the strictly decoded answer), on `today` and `evidence`.
fn run_input(
    runtime: &ComponentRuntime,
    id: &str,
    args: Value,
    g: &G,
    format: &str,
    today: &str,
    evidence: CommandEvidence,
) -> Out {
    let input = CommandInput {
        args: args
            .as_object()
            .cloned()
            .expect("a command's args are an object"),
        cwd: "/p".into(),
        format: CommandFormat::parse(format).expect("human or json"),
        today: today.into(),
        graph: RawGraph::new(g.graph().to_string()).expect("one JSON value"),
        evidence,
    };
    let export = format!("cmd__product_{id}");
    let output = ExtensionCalls::new(runtime)
        .run_command(PRODUCT, &export, &input)
        .unwrap_or_else(|error| panic!("{export}: {error}"));
    Out {
        exit: output.exit_code.into(),
        stdout: output.stdout,
        stderr: output.stderr,
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

/// The graph most tests ask about: milestones, journeys, personas, channels,
/// a deliverable and a term over three features.
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
        .n("gloss", "term")
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

fn not_found_suggesting(id: &str, args: Value, g: &G) -> Value {
    let error = RUNTIME.with(|runtime| run_in(runtime, id, args, g, "json").error());
    assert_eq!(error["code"], "ENTITY_NOT_FOUND", "{error}");
    error
}

// ── coverage matrices ─────────────────────────────────────────────────────

/// Personas, channels and deliverables over four features:
/// - j1 (dev, cli): f1, f2. j2 (dev, cli, web): f2, f3. j3 (ops, web): f4.
///   j4 (no persona or channel): f1. idle and none have no journey.
/// - d1: j1, j4. d2: j2, m1, m2. d3: m1, m2. m1: f1. m2: f2.
fn reach() -> G {
    G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .n("f4", "feature")
        .n("dev", "persona")
        .n("ops", "persona")
        .n("idle", "persona")
        .n("cli", "channel")
        .n("web", "channel")
        .n("none", "channel")
        .n("j1", "journey")
        .n("j2", "journey")
        .n("j3", "journey")
        .n("j4", "journey")
        .n("d1", "deliverable")
        .n("d2", "deliverable")
        .n("d3", "deliverable")
        .n("m1", "module")
        .n("m2", "module")
        .edge("j1", "dev", "persona")
        .edge("j2", "dev", "persona")
        .edge("j3", "ops", "persona")
        .edge("j1", "cli", "channels")
        .edge("j2", "cli", "channels")
        .edge("j2", "web", "channels")
        .edge("j3", "web", "channels")
        .edge("j1", "f1", "features")
        .edge("j1", "f2", "features")
        .edge("j2", "f2", "features")
        .edge("j2", "f3", "features")
        .edge("j3", "f4", "features")
        .edge("j4", "f1", "features")
        .edge("d1", "j1", "journeys")
        .edge("d1", "j4", "journeys")
        .edge("d2", "j2", "journeys")
        .edge("d2", "m1", "modules")
        .edge("d2", "m2", "modules")
        .edge("d3", "m1", "modules")
        .edge("d3", "m2", "modules")
        .edge("m1", "f1", "features")
        .edge("m2", "f2", "features")
}

/// The entry for `id` under `key` in a matrix payload.
fn matrix_entry(payload: &Value, key: &str, id_key: &str, id: &str) -> Value {
    payload[key]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e[id_key] == id)
        .unwrap_or_else(|| panic!("{id} not in {payload}"))
        .clone()
}

/// `n` personas, each the only one of its journey over one feature.
fn many_personas(n: usize) -> G {
    let mut g = G::default().n("f", "feature");
    for i in 0..n {
        let (p, j) = (format!("p{i:02}"), format!("j{i:02}"));
        g = g.n(&p, "persona").n(&j, "journey").edge(&j, &p, "persona");
    }
    g
}

fn ids_under(payload: &Value, key: &str, id_key: &str) -> Vec<String> {
    payload[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e[id_key].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test(
    behavior = "surface_coverage_matrix",
    verify = "product:coverage-matrix returns PersonaCoverageMatrixPayload"
)]
fn coverage_matrix_answers_its_payload() {
    let cm = json_of("coverage_matrix", json!({}), &reach());
    assert_eq!(
        cm,
        json!({
            "personas": [
                {"persona_id": "dev", "reachable_features": ["f1", "f2", "f3"],
                    "unreachable_features": ["f4"], "coverage_ratio": 0.75, "journey_count": 2},
                {"persona_id": "idle", "reachable_features": [],
                    "unreachable_features": ["f1", "f2", "f3", "f4"], "coverage_ratio": 0.0,
                    "journey_count": 0},
                {"persona_id": "ops", "reachable_features": ["f4"],
                    "unreachable_features": ["f1", "f2", "f3"], "coverage_ratio": 0.25,
                    "journey_count": 1},
            ],
            "total_features": 4, "overall_coverage": 1.0 / 3.0,
            "total": 3, "offset": 0, "limit": 100, "has_more": false,
        })
    );
    let human = human_of("coverage_matrix", json!({}), &reach());
    assert_eq!(
        human,
        "persona  reachable  unreachable  coverage\n\
         dev      3          1            75%\n\
         idle     0          4            0%\n\
         ops      1          3            25%\n\
         Overall coverage: 33%\n"
    );
}

#[specforge_test(
    behavior = "surface_coverage_matrix",
    verify = "no personas returns empty matrix"
)]
fn coverage_matrix_of_no_personas_is_empty() {
    let cm = json_of(
        "coverage_matrix",
        json!({}),
        &G::default().n("f1", "feature"),
    );
    assert_eq!(
        cm,
        json!({"personas": [], "total_features": 1, "overall_coverage": null,
            "total": 0, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "surface_channel_coverage_matrix",
    verify = "product:channel-coverage-matrix returns ChannelCoverageMatrixPayload"
)]
fn channel_coverage_matrix_answers_its_payload() {
    let cm = json_of("channel_coverage_matrix", json!({}), &reach());
    assert_eq!(
        cm,
        json!({
            "channels": [
                {"channel_id": "cli", "reachable_features": ["f1", "f2", "f3"],
                    "unreachable_features": ["f4"], "coverage_ratio": 0.75, "journey_count": 2},
                {"channel_id": "none", "reachable_features": [],
                    "unreachable_features": ["f1", "f2", "f3", "f4"], "coverage_ratio": 0.0,
                    "journey_count": 0},
                {"channel_id": "web", "reachable_features": ["f2", "f3", "f4"],
                    "unreachable_features": ["f1"], "coverage_ratio": 0.75, "journey_count": 2},
            ],
            "total_features": 4, "overall_coverage": 0.5,
            "total": 3, "offset": 0, "limit": 100, "has_more": false,
        })
    );
    let page = json_of(
        "channel_coverage_matrix",
        json!({"offset": 1, "limit": 1}),
        &reach(),
    );
    assert_eq!(ids_under(&page, "channels", "channel_id"), ["none"]);
    assert_eq!(
        (page["has_more"].clone(), page["overall_coverage"].clone()),
        (json!(true), json!(0.5))
    );
    let human = human_of("channel_coverage_matrix", json!({}), &reach());
    assert!(
        human.starts_with(
            "channel  reachable  unreachable  coverage\ncli      3          1            75%\n"
        ),
        "{human}"
    );
    assert!(human.ends_with("Overall coverage: 50%\n"), "{human}");
}

#[specforge_test(
    behavior = "surface_channel_coverage_matrix",
    verify = "no channels returns empty matrix"
)]
fn channel_coverage_matrix_of_no_channels_is_empty() {
    let cm = json_of("channel_coverage_matrix", json!({}), &G::default());
    assert_eq!(
        cm,
        json!({"channels": [], "total_features": 0, "overall_coverage": null,
            "total": 0, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "pe_query_persona_coverage_matrix",
    verify = "persona with journeys covering all features has coverage_ratio=1.0"
)]
fn a_persona_whose_journeys_cover_every_feature_is_at_one() {
    let g = reach()
        .edge("j1", "ops", "persona")
        .edge("j2", "ops", "persona");
    let ops = matrix_entry(
        &json_of("coverage_matrix", json!({}), &g),
        "personas",
        "persona_id",
        "ops",
    );
    assert_eq!(ops["coverage_ratio"], 1.0);
    assert_eq!(ops["unreachable_features"], json!([]));
}

#[specforge_test(
    behavior = "pe_query_persona_coverage_matrix",
    verify = "persona with no journeys has coverage_ratio=0.0"
)]
fn a_persona_without_journeys_is_at_zero() {
    let idle = matrix_entry(
        &json_of("coverage_matrix", json!({}), &reach()),
        "personas",
        "persona_id",
        "idle",
    );
    assert_eq!(
        (
            idle["coverage_ratio"].clone(),
            idle["journey_count"].clone()
        ),
        (json!(0.0), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_persona_coverage_matrix",
    verify = "overall_coverage is arithmetic mean of persona ratios"
)]
fn overall_persona_coverage_is_the_mean_of_the_ratios() {
    let cm = json_of("coverage_matrix", json!({}), &reach());
    let ratios: Vec<f64> = cm["personas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["coverage_ratio"].as_f64().unwrap())
        .collect();
    assert_eq!(
        cm["overall_coverage"].as_f64().unwrap(),
        ratios.iter().sum::<f64>() / 3.0
    );
    // Over every persona, not the page.
    let page = json_of("coverage_matrix", json!({"limit": 1}), &reach());
    assert_eq!(page["overall_coverage"], cm["overall_coverage"]);
}

#[specforge_test(
    behavior = "pe_query_persona_coverage_matrix",
    verify = "diamond topology deduplicates shared features"
)]
fn a_feature_two_of_a_personas_journeys_share_counts_once() {
    // dev reaches f2 through j1 and j2.
    let dev = matrix_entry(
        &json_of("coverage_matrix", json!({}), &reach()),
        "personas",
        "persona_id",
        "dev",
    );
    assert_eq!(dev["reachable_features"], json!(["f1", "f2", "f3"]));
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "channel with journeys covering all features has coverage_ratio=1.0"
)]
fn a_channel_whose_journeys_cover_every_feature_is_at_one() {
    let g = reach().edge("j4", "web", "channels");
    let web = matrix_entry(
        &json_of("channel_coverage_matrix", json!({}), &g),
        "channels",
        "channel_id",
        "web",
    );
    assert_eq!(web["coverage_ratio"], 1.0);
    assert_eq!(web["journey_count"], 3);
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "channel with no journeys has coverage_ratio=0.0"
)]
fn a_channel_without_journeys_is_at_zero() {
    let none = matrix_entry(
        &json_of("channel_coverage_matrix", json!({}), &reach()),
        "channels",
        "channel_id",
        "none",
    );
    assert_eq!(none["coverage_ratio"], 0.0);
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "overall_coverage is arithmetic mean of channel ratios"
)]
fn overall_channel_coverage_is_the_mean_of_the_ratios() {
    // (0.75 + 0 + 0.75) / 3.
    let cm = json_of("channel_coverage_matrix", json!({}), &reach());
    assert_eq!(cm["overall_coverage"], 0.5);
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "diamond topology deduplicates shared features"
)]
fn a_feature_two_of_a_channels_journeys_share_counts_once() {
    // web reaches f2 and f3 through j2, f4 through j3; cli f2 through j1 and j2.
    let cm = json_of("channel_coverage_matrix", json!({}), &reach());
    let cli = matrix_entry(&cm, "channels", "channel_id", "cli");
    assert_eq!(cli["reachable_features"], json!(["f1", "f2", "f3"]));
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "empty channel graph returns empty matrix"
)]
fn a_graph_without_channels_has_an_empty_channel_matrix() {
    let cm = json_of("channel_coverage_matrix", json!({}), &many_personas(2));
    assert_eq!(
        (
            cm["channels"].clone(),
            cm["total"].clone(),
            cm["overall_coverage"].clone()
        ),
        (json!([]), json!(0), json!(null))
    );
}

#[specforge_test(
    behavior = "product_new_query_correctness",
    verify = "coverage matrix ratios are mathematically correct"
)]
fn each_coverage_ratio_is_reachable_over_total() {
    for (id, key, id_key) in [
        ("coverage_matrix", "personas", "persona_id"),
        ("channel_coverage_matrix", "channels", "channel_id"),
    ] {
        let cm = json_of(id, json!({}), &reach());
        let total = cm["total_features"].as_f64().unwrap();
        for e in cm[key].as_array().unwrap() {
            let reachable = e["reachable_features"].as_array().unwrap().len() as f64;
            let unreachable = e["unreachable_features"].as_array().unwrap().len() as f64;
            assert_eq!(reachable + unreachable, total, "{id}: {}", e[id_key]);
            assert_eq!(
                e["coverage_ratio"].as_f64().unwrap(),
                reachable / total,
                "{id}: {e}"
            );
        }
    }
}

#[specforge_test(
    behavior = "product_query_pagination_required",
    verify = "matrix query with limit=10 returns at most 10 entries"
)]
fn a_matrix_query_with_limit_ten_returns_ten() {
    let cm = json_of("coverage_matrix", json!({"limit": 10}), &many_personas(12));
    assert_eq!(cm["personas"].as_array().unwrap().len(), 10);
    assert_eq!(
        (cm["total"].clone(), cm["has_more"].clone()),
        (json!(12), json!(true))
    );
}

#[specforge_test(
    behavior = "product_query_pagination_required",
    verify = "offset=limit retrieves the second page"
)]
fn offset_equal_to_limit_is_the_second_page() {
    let g = many_personas(12);
    let first = json_of("coverage_matrix", json!({"limit": 5}), &g);
    let second = json_of("coverage_matrix", json!({"offset": 5, "limit": 5}), &g);
    let all = json_of("coverage_matrix", json!({}), &g);
    let ids = ids_under(&all, "personas", "persona_id");
    assert_eq!(ids_under(&first, "personas", "persona_id"), ids[..5]);
    assert_eq!(ids_under(&second, "personas", "persona_id"), ids[5..10]);
}

#[specforge_test(
    behavior = "product_query_pagination_required",
    verify = "limit>1000 is clamped to 1000"
)]
fn a_matrix_limit_above_a_thousand_is_a_thousand() {
    for id in [
        "coverage_matrix",
        "channel_coverage_matrix",
        "feature_overlap",
    ] {
        let page = json_of(id, json!({"limit": 5000}), &reach());
        assert_eq!(page["limit"], 1000, "{id}");
        let page = json_of(id, json!({"limit": 0}), &reach());
        assert_eq!(page["limit"], 1, "{id}");
        let error =
            RUNTIME.with(|runtime| run_in(runtime, id, json!({"offset": -1}), &reach(), "json"));
        assert_eq!(error.exit, 2, "{id}");
        assert_eq!(error.error()["code"], "INVALID_INPUT");
    }
}

#[specforge_test(
    behavior = "surface_feature_overlap",
    verify = "feature-overlap returns FeatureOverlapPayload JSON"
)]
fn feature_overlap_answers_its_payload() {
    let fo = json_of("feature_overlap", json!({}), &reach());
    assert_eq!(
        fo,
        json!({
            "overlapping_features": [
                {"feature_id": "f1", "deliverable_ids": ["d1", "d2", "d3"], "deliverable_count": 3},
                {"feature_id": "f2", "deliverable_ids": ["d1", "d2", "d3"], "deliverable_count": 3},
            ],
            "count": 2, "total_features": 4,
            "total": 2, "offset": 0, "limit": 100, "has_more": false,
        })
    );
    let page = json_of("feature_overlap", json!({"offset": 1}), &reach());
    assert_eq!(
        ids_under(&page, "overlapping_features", "feature_id"),
        ["f2"]
    );
    assert_eq!(page["count"], 2);
    let human = human_of("feature_overlap", json!({}), &reach());
    assert_eq!(
        human,
        "feature  deliverables  ids\n\
         f1       3             d1, d2, d3\n\
         f2       3             d1, d2, d3\n\
         2 of 4 features shared by two or more deliverables\n"
    );
}

#[specforge_test(
    behavior = "surface_feature_overlap",
    verify = "no overlapping features returns empty list"
)]
fn feature_overlap_without_shared_features_is_empty() {
    let fo = json_of("feature_overlap", json!({}), &plan());
    assert_eq!(
        fo,
        json!({"overlapping_features": [], "count": 0, "total_features": 3,
            "total": 0, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_overlap",
    verify = "feature in two deliverables appears in overlap list"
)]
fn a_feature_two_deliverables_reach_overlaps() {
    let g = G::default()
        .n("f", "feature")
        .n("j", "journey")
        .n("m", "module")
        .n("a", "deliverable")
        .n("b", "deliverable")
        .edge("j", "f", "features")
        .edge("m", "f", "features")
        .edge("a", "j", "journeys")
        .edge("b", "m", "modules");
    let fo = json_of("feature_overlap", json!({}), &g);
    assert_eq!(
        fo["overlapping_features"],
        json!([{"feature_id": "f", "deliverable_ids": ["a", "b"], "deliverable_count": 2}])
    );
}

#[specforge_test(
    behavior = "pe_query_feature_overlap",
    verify = "feature in one deliverable is excluded"
)]
fn a_feature_one_deliverable_reaches_does_not_overlap() {
    // f3 is d2's alone, f4 no deliverable's.
    let fo = json_of("feature_overlap", json!({}), &reach());
    let ids = ids_under(&fo, "overlapping_features", "feature_id");
    assert!(
        !ids.contains(&"f3".to_string()) && !ids.contains(&"f4".to_string()),
        "{fo}"
    );
}

#[specforge_test(
    behavior = "pe_query_feature_overlap",
    verify = "feature reachable via journey path and module path from same deliverable counts once"
)]
fn a_deliverable_reaching_a_feature_both_ways_counts_once() {
    // d2 reaches f2 through j2 and through m2.
    let fo = json_of("feature_overlap", json!({}), &reach());
    let f2 = matrix_entry(&fo, "overlapping_features", "feature_id", "f2");
    assert_eq!(f2["deliverable_ids"], json!(["d1", "d2", "d3"]));
    // A deliverable alone, both ways, is no overlap.
    let g = G::default()
        .n("f", "feature")
        .n("j", "journey")
        .n("m", "module")
        .n("a", "deliverable")
        .edge("j", "f", "features")
        .edge("m", "f", "features")
        .edge("a", "j", "journeys")
        .edge("a", "m", "modules");
    assert_eq!(json_of("feature_overlap", json!({}), &g)["count"], 0);
}

#[specforge_test(
    behavior = "pe_query_feature_overlap",
    verify = "diamond topology correctly deduplicates"
)]
fn a_diamond_lists_each_feature_and_deliverable_once() {
    // d1 reaches f1 through j1 and j4; d2 and d3 through m1.
    let fo = json_of("feature_overlap", json!({}), &reach());
    let f1 = matrix_entry(&fo, "overlapping_features", "feature_id", "f1");
    assert_eq!(
        (
            f1["deliverable_ids"].clone(),
            f1["deliverable_count"].clone()
        ),
        (json!(["d1", "d2", "d3"]), json!(3))
    );
    let ids = ids_under(&fo, "overlapping_features", "feature_id");
    assert_eq!(ids, ["f1", "f2"]);
}

#[specforge_test(
    behavior = "product_new_query_correctness",
    verify = "feature overlap detected via both journey and module paths"
)]
fn overlap_is_found_through_journeys_and_through_modules() {
    let base = || {
        G::default()
            .n("f", "feature")
            .n("j1", "journey")
            .n("j2", "journey")
            .n("m1", "module")
            .n("m2", "module")
            .n("a", "deliverable")
            .n("b", "deliverable")
            .edge("j1", "f", "features")
            .edge("j2", "f", "features")
            .edge("m1", "f", "features")
            .edge("m2", "f", "features")
    };
    for g in [
        base()
            .edge("a", "j1", "journeys")
            .edge("b", "j2", "journeys"),
        base().edge("a", "m1", "modules").edge("b", "m2", "modules"),
        base()
            .edge("a", "j1", "journeys")
            .edge("b", "m2", "modules"),
    ] {
        let fo = json_of("feature_overlap", json!({}), &g);
        assert_eq!(
            fo["overlapping_features"][0]["deliverable_ids"],
            json!(["a", "b"]),
            "{fo}"
        );
    }
}

// ── term analytics ────────────────────────────────────────────────────────

/// A glossary: api -> rest -> http -> tcp, grpc -> proto, and lone, which
/// links nowhere.
fn glossary() -> G {
    G::default()
        .n("api", "term")
        .n("rest", "term")
        .n("http", "term")
        .n("tcp", "term")
        .n("grpc", "term")
        .n("proto", "term")
        .n("lone", "term")
        .edge("api", "rest", "see_also")
        .edge("rest", "http", "see_also")
        .edge("http", "tcp", "see_also")
        .edge("grpc", "proto", "see_also")
}

/// Terms named `ids` with a see_also from each pair's first to its second.
fn terms(ids: &[&str], links: &[(&str, &str)]) -> G {
    let mut g = G::default();
    for id in ids {
        g = g.n(id, "term");
    }
    for (a, b) in links {
        g = g.edge(a, b, "see_also");
    }
    g
}

fn related(term: &str, args: Value, g: &G) -> Value {
    let mut args = args;
    args["term"] = json!(term);
    json_of("term_graph", args, g)["related_terms"].clone()
}

#[specforge_test(
    behavior = "surface_term_graph",
    verify = "term-graph returns TermGraphPayload JSON"
)]
fn term_graph_answers_its_payload() {
    let tg = json_of(
        "term_graph",
        json!({"term": "api", "max_hops": 2}),
        &glossary(),
    );
    assert_eq!(
        tg,
        json!({"term_id": "api", "related_terms": ["http", "rest"], "max_hops": 2})
    );
    let human = human_of("term_graph", json!({"term": "api"}), &glossary());
    assert_eq!(
        human,
        "Terms related to 'api' within 1 see_also hop:\n  rest\n"
    );
}

#[specforge_test(
    behavior = "surface_term_graph",
    verify = "max-hops flag is respected and capped at 5"
)]
fn term_graph_follows_max_hops_up_to_five() {
    let chain = terms(
        &["t0", "t1", "t2", "t3", "t4", "t5", "t6", "t7"],
        &[
            ("t0", "t1"),
            ("t1", "t2"),
            ("t2", "t3"),
            ("t3", "t4"),
            ("t4", "t5"),
            ("t5", "t6"),
            ("t6", "t7"),
        ],
    );
    assert_eq!(
        related("t0", json!({"max_hops": 3}), &chain),
        json!(["t1", "t2", "t3"])
    );
    let capped = json_of("term_graph", json!({"term": "t0", "max_hops": 7}), &chain);
    assert_eq!(capped["max_hops"], 5);
    assert_eq!(
        capped["related_terms"],
        json!(["t1", "t2", "t3", "t4", "t5"])
    );
    // Not a count: refused.
    let error = RUNTIME.with(|runtime| {
        run_in(
            runtime,
            "term_graph",
            json!({"term": "t0", "max_hops": -1}),
            &chain,
            "json",
        )
    });
    assert_eq!(error.exit, 2);
    assert_eq!(error.error()["code"], "INVALID_INPUT");
}

#[specforge_test(
    behavior = "surface_term_graph",
    verify = "missing term ID returns error with suggestion"
)]
fn term_graph_of_a_mistyped_term_suggests_the_nearest() {
    let error = not_found_suggesting("term_graph", json!({"term": "apj"}), &glossary());
    assert_eq!(error["suggestion"], "api");
    assert_eq!(error["message"], "term 'apj' not found");
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "term with see_also returns related terms at hop 1"
)]
fn a_terms_see_also_is_related_at_one_hop() {
    assert_eq!(
        related("rest", json!({"max_hops": 1}), &glossary()),
        json!(["http"])
    );
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "term with 2-hop chain returns transitive terms when maxHops=2"
)]
fn a_two_hop_chain_is_related_at_two_hops() {
    assert_eq!(
        related("rest", json!({"max_hops": 2}), &glossary()),
        json!(["http", "tcp"])
    );
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "term with no see_also returns empty related_terms"
)]
fn a_term_without_see_also_has_no_related_terms() {
    assert_eq!(
        related("lone", json!({"max_hops": 5}), &glossary()),
        json!([])
    );
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "source term is excluded from related_terms"
)]
fn the_term_is_not_its_own_relation() {
    let cycle = terms(&["a", "b"], &[("a", "b"), ("b", "a"), ("a", "a")]);
    assert_eq!(related("a", json!({"max_hops": 5}), &cycle), json!(["b"]));
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "omitted maxHops defaults to 1"
)]
fn max_hops_defaults_to_one() {
    let tg = json_of("term_graph", json!({"term": "api"}), &glossary());
    assert_eq!(
        (tg["max_hops"].clone(), tg["related_terms"].clone()),
        (json!(1), json!(["rest"]))
    );
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "maxHops=10 is clamped to 5"
)]
fn ten_hops_are_five() {
    let tg = json_of(
        "term_graph",
        json!({"term": "api", "max_hops": 10}),
        &glossary(),
    );
    assert_eq!(tg["max_hops"], 5);
    assert_eq!(tg["related_terms"], json!(["http", "rest", "tcp"]));
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "maxHops=0 returns empty related_terms"
)]
fn zero_hops_relate_nothing() {
    assert_eq!(
        related("api", json!({"max_hops": 0}), &glossary()),
        json!([])
    );
}

#[specforge_test(
    behavior = "product_query_correctness",
    verify = "term graph respects maxHops boundary"
)]
fn each_hop_adds_exactly_the_terms_one_further() {
    // Along api -> rest -> http -> tcp, hop h reaches the first h terms.
    let order = ["rest", "http", "tcp"];
    for hops in 0..=5usize {
        let mut expected: Vec<&str> = order[..hops.min(3)].to_vec();
        expected.sort_unstable();
        assert_eq!(
            related("api", json!({"max_hops": hops}), &glossary()),
            json!(expected),
            "{hops} hops"
        );
    }
}

#[specforge_test(
    behavior = "surface_term_clusters",
    verify = "product:term-clusters returns TermClusterPayload"
)]
fn term_clusters_answers_its_payload() {
    let tc = json_of("term_clusters", json!({}), &glossary());
    assert_eq!(
        tc,
        json!({
            "clusters": [
                {"cluster_id": 1, "term_ids": ["api", "http", "rest", "tcp"], "term_count": 4},
                {"cluster_id": 2, "term_ids": ["grpc", "proto"], "term_count": 2},
            ],
            "cluster_count": 2, "isolated_count": 1, "total_terms": 7,
        })
    );
    let human = human_of("term_clusters", json!({}), &glossary());
    assert_eq!(
        human,
        "cluster  terms  ids\n\
         1        4      api, http, rest, tcp\n\
         2        2      grpc, proto\n\
         2 clusters, 1 isolated of 7 terms\n"
    );
}

#[specforge_test(
    behavior = "surface_term_clusters",
    verify = "no terms returns zero clusters and zero isolated"
)]
fn term_clusters_of_no_terms_is_empty() {
    assert_eq!(
        json_of("term_clusters", json!({}), &G::default().n("f1", "feature")),
        json!({"clusters": [], "cluster_count": 0, "isolated_count": 0, "total_terms": 0})
    );
}

fn cluster_ids(g: &G) -> Value {
    let tc = json_of("term_clusters", json!({}), g);
    Value::Array(
        tc["clusters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["term_ids"].clone())
            .collect(),
    )
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "three terms in a connected chain produce one cluster of size 3"
)]
fn a_chain_of_three_terms_is_one_cluster() {
    // b links to both, so a and c are clustered through it.
    let g = terms(&["a", "b", "c"], &[("b", "a"), ("b", "c")]);
    let tc = json_of("term_clusters", json!({}), &g);
    assert_eq!(
        tc["clusters"],
        json!([{"cluster_id": 1, "term_ids": ["a", "b", "c"], "term_count": 3}])
    );
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "two disconnected pairs produce two clusters of size 2"
)]
fn two_disconnected_pairs_are_two_clusters() {
    let g = terms(&["a", "b", "c", "d"], &[("a", "b"), ("d", "c")]);
    assert_eq!(cluster_ids(&g), json!([["a", "b"], ["c", "d"]]));
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "isolated term with no see_also edges is counted in isolated_count"
)]
fn a_term_without_see_also_is_isolated() {
    let tc = json_of("term_clusters", json!({}), &glossary());
    assert_eq!(tc["isolated_count"], 1);
    assert!(!cluster_ids(&glossary()).to_string().contains("lone"));
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "empty term graph returns zero clusters and zero isolated"
)]
fn an_empty_glossary_has_no_clusters() {
    let tc = json_of("term_clusters", json!({}), &G::default());
    assert_eq!(
        (
            tc["cluster_count"].clone(),
            tc["isolated_count"].clone(),
            tc["total_terms"].clone()
        ),
        (json!(0), json!(0), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "total_terms equals sum of cluster sizes plus isolated_count"
)]
fn cluster_sizes_and_isolated_terms_add_up_to_every_term() {
    let tc = json_of("term_clusters", json!({}), &glossary());
    let sizes: u64 = tc["clusters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["term_count"].as_u64().unwrap())
        .sum();
    assert_eq!(
        sizes + tc["isolated_count"].as_u64().unwrap(),
        tc["total_terms"].as_u64().unwrap()
    );
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "clusters sorted by size descending"
)]
fn the_largest_cluster_comes_first() {
    let g = terms(
        &["a", "b", "x", "y", "z", "m", "n"],
        &[("a", "b"), ("x", "y"), ("y", "z"), ("n", "m")],
    );
    assert_eq!(
        cluster_ids(&g),
        json!([["x", "y", "z"], ["a", "b"], ["m", "n"]])
    );
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "result is deterministic across repeated queries"
)]
fn term_clusters_are_the_same_every_time_and_in_any_order() {
    let first = json_of("term_clusters", json!({}), &glossary());
    assert_eq!(json_of("term_clusters", json!({}), &glossary()), first);
    let mut g = glossary();
    g.nodes.reverse();
    g.edges.reverse();
    assert_eq!(json_of("term_clusters", json!({}), &g), first);
}

/// A hub and six spokes linked in a ring and by two chords: 14 links over
/// 7 terms, an average of 2; the hub has 6 connections, each spoke 3 or 4.
fn hub() -> G {
    terms(
        &["h", "s1", "s2", "s3", "s4", "s5", "s6"],
        &[
            ("h", "s1"),
            ("h", "s2"),
            ("h", "s3"),
            ("h", "s4"),
            ("h", "s5"),
            ("h", "s6"),
            ("s1", "s2"),
            ("s2", "s3"),
            ("s3", "s4"),
            ("s4", "s5"),
            ("s5", "s6"),
            ("s6", "s1"),
            ("s1", "s3"),
            ("s2", "s4"),
        ],
    )
}

#[specforge_test(
    behavior = "surface_term_density",
    verify = "product:term-density returns TermDensityPayload"
)]
fn term_density_answers_its_payload() {
    let td = json_of("term_density", json!({}), &glossary());
    assert_eq!(
        td,
        json!({"total_terms": 7, "total_see_also": 4, "avg_connections": 4.0 / 7.0,
            "max_connections": 2, "hub_terms": [], "isolated_terms": ["lone"]})
    );
    let human = human_of("term_density", json!({}), &hub());
    assert_eq!(
        human,
        "Terms:           7\n\
         see_also edges:  14\n\
         Avg connections: 2.00\n\
         Max connections: 6\n\
         Hubs (1):        h\n\
         Isolated (0):    -\n"
    );
}

#[specforge_test(
    behavior = "surface_term_density",
    verify = "empty graph returns zero stats"
)]
fn term_density_of_no_terms_is_zero() {
    assert_eq!(
        json_of("term_density", json!({}), &G::default()),
        json!({"total_terms": 0, "total_see_also": 0, "avg_connections": null,
            "max_connections": 0, "hub_terms": [], "isolated_terms": []})
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "5 terms with 8 edges computes correct average"
)]
fn five_terms_with_eight_links_average_one_point_six() {
    let g = terms(
        &["a", "b", "c", "d", "e"],
        &[
            ("a", "b"),
            ("a", "c"),
            ("a", "d"),
            ("a", "e"),
            ("b", "c"),
            ("c", "d"),
            ("d", "e"),
            ("e", "b"),
        ],
    );
    let td = json_of("term_density", json!({}), &g);
    assert_eq!(
        (td["total_see_also"].clone(), td["avg_connections"].clone()),
        (json!(8), json!(1.6))
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "term with 6 connections in a graph averaging 2 is a hub"
)]
fn six_connections_against_an_average_of_two_is_a_hub() {
    let td = json_of("term_density", json!({}), &hub());
    assert_eq!(td["avg_connections"], 2.0);
    assert_eq!(td["hub_terms"], json!(["h"]));
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "term with zero connections is listed in isolated_terms"
)]
fn a_term_without_links_is_isolated() {
    assert_eq!(
        json_of("term_density", json!({}), &glossary())["isolated_terms"],
        json!(["lone"])
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "empty term graph returns total_terms=0 and avg_connections=null"
)]
fn an_empty_glossary_has_no_average() {
    let td = json_of("term_density", json!({}), &G::default().n("f1", "feature"));
    assert_eq!(
        (td["total_terms"].clone(), td["avg_connections"].clone()),
        (json!(0), json!(null))
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "max_connections reflects the most-connected term"
)]
fn max_connections_is_the_hubs() {
    assert_eq!(
        json_of("term_density", json!({}), &hub())["max_connections"],
        6
    );
    // rest and http each link two terms, one either way.
    assert_eq!(
        json_of("term_density", json!({}), &glossary())["max_connections"],
        2
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "result is deterministic across repeated queries"
)]
fn term_density_is_the_same_every_time_and_in_any_order() {
    let first = json_of("term_density", json!({}), &hub());
    assert_eq!(json_of("term_density", json!({}), &hub()), first);
    let mut g = hub();
    g.nodes.reverse();
    g.edges.reverse();
    assert_eq!(json_of("term_density", json!({}), &g), first);
}

// ── dates and effort ──────────────────────────────────────────────────────

/// Milestones around the date the tests pass as today (2026-10-03):
/// done (2026-08-01, completed), late (2026-09-01, in progress, high),
/// next (2026-11-01, critical), a and b (both 2026-12-15) and undated.
fn timeline() -> G {
    G::default()
        .node(
            "done",
            "milestone",
            json!({"target_date": "2026-08-01", "status": "completed"}),
        )
        .node(
            "late",
            "milestone",
            json!({"target_date": "2026-09-01", "status": "in_progress", "priority": "high"}),
        )
        .node(
            "next",
            "milestone",
            json!({"target_date": "2026-11-01", "priority": "critical"}),
        )
        .node("b", "milestone", json!({"target_date": "2026-12-15"}))
        .node("a", "milestone", json!({"target_date": "2026-12-15"}))
        .n("undated", "milestone")
}

fn timeline_order(t: &Value) -> Vec<(String, bool)> {
    t["milestones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["milestone_id"].as_str().unwrap().to_string(),
                m["is_overdue"].as_bool().unwrap(),
            )
        })
        .collect()
}

fn ids_flagged(order: &[(&str, bool)]) -> Vec<(String, bool)> {
    order.iter().map(|(id, o)| (id.to_string(), *o)).collect()
}

#[specforge_test(
    behavior = "surface_milestone_timeline",
    verify = "milestone-timeline returns MilestoneTimelinePayload JSON"
)]
fn milestone_timeline_answers_its_payload() {
    let t = json_of("milestone_timeline", json!({}), &timeline());
    assert_eq!(
        t,
        json!({
            "milestones": [
                {"milestone_id": "done", "target_date": "2026-08-01", "status": "completed",
                    "is_overdue": false, "priority": null},
                {"milestone_id": "late", "target_date": "2026-09-01", "status": "in_progress",
                    "is_overdue": true, "priority": "high"},
                {"milestone_id": "next", "target_date": "2026-11-01", "status": null,
                    "is_overdue": false, "priority": "critical"},
                {"milestone_id": "a", "target_date": "2026-12-15", "status": null,
                    "is_overdue": false, "priority": null},
                {"milestone_id": "b", "target_date": "2026-12-15", "status": null,
                    "is_overdue": false, "priority": null},
                {"milestone_id": "undated", "target_date": null, "status": null,
                    "is_overdue": false, "priority": null},
            ],
            "overdue_count": 1,
        })
    );
}

#[specforge_test(
    behavior = "surface_milestone_timeline",
    verify = "as-of flag overrides current date for overdue calculation"
)]
fn as_of_overrides_the_hosts_today() {
    let t = json_of(
        "milestone_timeline",
        json!({"as_of": "2026-12-01"}),
        &timeline(),
    );
    assert_eq!(t["overdue_count"], 2);
    let early = json_of(
        "milestone_timeline",
        json!({"as_of": "2026-01-01"}),
        &timeline(),
    );
    assert_eq!(early["overdue_count"], 0);
    // A date that is not one is refused, from --as-of or from the host.
    for args in [
        json!({"as_of": "2026-02-30"}),
        json!({"as_of": "tomorrow"}),
        json!({"as_of": 3}),
    ] {
        let out = RUNTIME.with(|runtime| {
            run_in(
                runtime,
                "milestone_timeline",
                args.clone(),
                &timeline(),
                "json",
            )
        });
        assert_eq!(out.exit, 2, "{args}");
        assert_eq!(out.error()["code"], "INVALID_INPUT", "{args}");
    }
    // The host passed no date: no "as of" to default to.
    let out = run_input(
        &runtime(),
        "milestone_timeline",
        json!({}),
        &timeline(),
        "json",
        "",
        CommandEvidence::None,
    );
    assert_eq!(out.exit, 2, "{}", out.stderr);
    assert!(out.stderr.contains("--as-of"), "{}", out.stderr);
}

#[specforge_test(
    behavior = "surface_milestone_timeline",
    verify = "human format marks overdue milestones"
)]
fn the_human_timeline_marks_the_overdue() {
    let human = human_of("milestone_timeline", json!({}), &timeline());
    assert_eq!(
        human,
        "milestone  target_date  status       priority  overdue\n\
         done       2026-08-01   completed    -\n\
         late       2026-09-01   in_progress  high      OVERDUE\n\
         next       2026-11-01   -            critical\n\
         a          2026-12-15   -            -\n\
         b          2026-12-15   -            -\n\
         undated    -            -            -\n\
         1 overdue as of 2026-10-03\n"
    );
}

#[specforge_test(
    behavior = "pe_query_milestone_timeline",
    verify = "milestones sorted by target_date ascending"
)]
fn the_timeline_is_earliest_first() {
    let t = json_of("milestone_timeline", json!({}), &timeline());
    let dates: Vec<&str> = t["milestones"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["target_date"].as_str())
        .collect();
    let mut sorted = dates.clone();
    sorted.sort_unstable();
    assert_eq!(dates, sorted);
}

#[specforge_test(
    behavior = "pe_query_milestone_timeline",
    verify = "undated milestones appear after dated milestones"
)]
fn undated_milestones_come_last() {
    let g =
        timeline()
            .n("aaa", "milestone")
            .node("bad", "milestone", json!({"target_date": "soon"}));
    let t = json_of("milestone_timeline", json!({}), &g);
    let order: Vec<String> = timeline_order(&t).into_iter().map(|(id, _)| id).collect();
    assert_eq!(&order[5..], ["aaa", "bad", "undated"]);
}

#[specforge_test(
    behavior = "pe_query_milestone_timeline",
    verify = "overdue milestone with status=in_progress is flagged in query result"
)]
fn an_in_progress_milestone_past_its_date_is_overdue() {
    let t = json_of("milestone_timeline", json!({}), &timeline());
    assert_eq!(timeline_order(&t)[1], ("late".to_string(), true));
}

#[specforge_test(
    behavior = "pe_query_milestone_timeline",
    verify = "completed milestone past target_date is not flagged overdue"
)]
fn a_completed_milestone_is_never_overdue() {
    let t = json_of(
        "milestone_timeline",
        json!({"as_of": "2030-01-01"}),
        &timeline(),
    );
    assert_eq!(timeline_order(&t)[0], ("done".to_string(), false));
    assert_eq!(t["overdue_count"], 4);
}

#[specforge_test(
    behavior = "pe_query_milestone_timeline",
    verify = "empty milestone set returns empty timeline"
)]
fn no_milestones_is_an_empty_timeline() {
    assert_eq!(
        json_of("milestone_timeline", json!({}), &G::default()),
        json!({"milestones": [], "overdue_count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_milestone_timeline",
    verify = "same as_of_date produces identical timeline across repeated queries"
)]
fn one_as_of_date_is_one_timeline() {
    let args = json!({"as_of": "2026-11-15"});
    let first = json_of("milestone_timeline", args.clone(), &timeline());
    assert_eq!(
        json_of("milestone_timeline", args.clone(), &timeline()),
        first
    );
    assert_eq!(
        timeline_order(&first),
        ids_flagged(&[
            ("done", false),
            ("late", true),
            ("next", true),
            ("a", false),
            ("b", false),
            ("undated", false)
        ])
    );
}

#[specforge_test(
    behavior = "pe_query_milestone_timeline",
    verify = "milestone timeline is deterministic across repeated queries"
)]
fn the_timeline_is_the_same_every_time_and_in_any_order() {
    let first = json_of("milestone_timeline", json!({}), &timeline());
    let mut g = timeline();
    g.nodes.reverse();
    assert_eq!(json_of("milestone_timeline", json!({}), &g), first);
    assert_eq!(json_of("milestone_timeline", json!({}), &timeline()), first);
}

/// A milestone started 30 days before the tests' today with three of its
/// five features done and one in progress, and one with no dates.
fn velocity() -> G {
    G::default()
        .node(
            "ms",
            "milestone",
            json!({"start_date": "2026-09-03", "target_date": "2026-12-01"}),
        )
        .node("nodate", "milestone", json!({}))
        .node("due", "milestone", json!({"target_date": "2026-09-23"}))
        .feature("f1", "done")
        .feature("f2", "done")
        .feature("f3", "done")
        .feature("f4", "in_progress")
        .n("f5", "feature")
        .edge("ms", "f1", "features")
        .edge("ms", "f2", "features")
        .edge("ms", "f3", "features")
        .edge("ms", "f4", "features")
        .edge("ms", "f5", "features")
        .edge("nodate", "f1", "features")
        .edge("nodate", "f4", "features")
        .edge("due", "f4", "features")
        .edge("due", "f5", "features")
}

#[specforge_test(
    behavior = "surface_milestone_velocity",
    verify = "milestone-velocity returns MilestoneVelocityPayload JSON"
)]
fn milestone_velocity_answers_its_payload() {
    let v = json_of(
        "milestone_velocity",
        json!({"milestone": "ms"}),
        &velocity(),
    );
    assert_eq!(
        v,
        json!({"milestone_id": "ms", "total_features": 5, "done_features": 3,
            "in_progress_features": 1, "remaining_features": 1, "completion_ratio": 0.6,
            "days_elapsed": 30, "days_remaining": 20, "features_per_day": 0.1})
    );
    let human = human_of(
        "milestone_velocity",
        json!({"milestone": "ms"}),
        &velocity(),
    );
    assert_eq!(
        human,
        "Milestone: ms (as of 2026-10-03)\n\
         Features: 5 total, 3 done, 1 in progress, 1 remaining\n\
         Completion: 60%\n\
         Days elapsed: 30\n\
         Features per day: 0.10\n\
         Days remaining: 20\n"
    );
    let later = json_of(
        "milestone_velocity",
        json!({"milestone": "ms", "as_of": "2026-10-13"}),
        &velocity(),
    );
    assert_eq!(
        (
            later["days_elapsed"].clone(),
            later["features_per_day"].clone()
        ),
        (json!(40), json!(0.075))
    );
}

#[specforge_test(
    behavior = "surface_milestone_velocity",
    verify = "missing milestone ID returns error with suggestion"
)]
fn milestone_velocity_of_a_mistyped_milestone_suggests_the_nearest() {
    let error = not_found_suggesting(
        "milestone_velocity",
        json!({"milestone": "mss"}),
        &velocity(),
    );
    assert_eq!(error["suggestion"], "ms");
}

#[specforge_test(
    behavior = "pe_query_milestone_velocity",
    verify = "milestone with 3 done and 2 remaining returns correct counts"
)]
fn three_done_of_five_are_counted_by_status() {
    let v = json_of(
        "milestone_velocity",
        json!({"milestone": "ms"}),
        &velocity(),
    );
    let counts = |k: &str| v[k].as_u64().unwrap();
    assert_eq!(
        (
            counts("done_features"),
            counts("total_features") - counts("done_features")
        ),
        (3, 2)
    );
    assert_eq!(
        counts("total_features"),
        counts("done_features") + counts("in_progress_features") + counts("remaining_features")
    );
}

#[specforge_test(
    behavior = "pe_query_milestone_velocity",
    verify = "milestone with no done features returns null velocity"
)]
fn a_milestone_with_nothing_done_has_no_velocity() {
    // due's date is the start: 10 days elapsed, nothing done.
    let v = json_of(
        "milestone_velocity",
        json!({"milestone": "due"}),
        &velocity(),
    );
    assert_eq!(
        (
            v["days_elapsed"].clone(),
            v["features_per_day"].clone(),
            v["days_remaining"].clone()
        ),
        (json!(10), json!(null), json!(null))
    );
}

#[specforge_test(
    behavior = "pe_query_milestone_velocity",
    verify = "milestone without target_date returns null days_elapsed"
)]
fn a_milestone_without_dates_has_no_days() {
    let v = json_of(
        "milestone_velocity",
        json!({"milestone": "nodate"}),
        &velocity(),
    );
    assert_eq!(
        (
            v["days_elapsed"].clone(),
            v["days_remaining"].clone(),
            v["features_per_day"].clone()
        ),
        (json!(null), json!(null), json!(null))
    );
    assert_eq!(v["completion_ratio"], 0.5);
}

#[specforge_test(
    behavior = "pe_query_milestone_velocity",
    verify = "velocity calculation is mathematically correct"
)]
fn velocity_is_done_over_elapsed_and_paces_what_is_left() {
    for (as_of, elapsed) in [
        ("2026-09-04", 1),
        ("2026-09-10", 7),
        ("2026-10-03", 30),
        ("2027-09-03", 365),
    ] {
        let v = json_of(
            "milestone_velocity",
            json!({"milestone": "ms", "as_of": as_of}),
            &velocity(),
        );
        let pace = 3.0 / elapsed as f64;
        assert_eq!(v["days_elapsed"], elapsed, "{as_of}");
        assert_eq!(v["features_per_day"].as_f64().unwrap(), pace, "{as_of}");
        // 2 left at 3 per elapsed days: 2 * elapsed / 3 days, rounded up.
        assert_eq!(v["days_remaining"], (2 * elapsed + 2) / 3, "{as_of}");
    }
    // Before the start no day has elapsed.
    let v = json_of(
        "milestone_velocity",
        json!({"milestone": "ms", "as_of": "2026-01-01"}),
        &velocity(),
    );
    assert_eq!(
        (v["days_elapsed"].clone(), v["features_per_day"].clone()),
        (json!(0), json!(null))
    );
}

/// A milestone with an xs feature done and an xl one pending, one with
/// a feature of each effort and one without, and an empty one.
fn efforts() -> G {
    G::default()
        .node("x1", "feature", json!({"effort": "xs", "status": "done"}))
        .node("x2", "feature", json!({"effort": "xl"}))
        .node("e1", "feature", json!({"effort": "xs", "status": "done"}))
        .node("e2", "feature", json!({"effort": "s"}))
        .node("e3", "feature", json!({"effort": "m", "status": "done"}))
        .node("e4", "feature", json!({"effort": "l"}))
        .node("e5", "feature", json!({"effort": "xl", "status": "done"}))
        .node("e6", "feature", json!({"status": "done"}))
        .n("ms_pair", "milestone")
        .n("ms_all", "milestone")
        .n("ms_empty", "milestone")
        .edge("ms_pair", "x1", "features")
        .edge("ms_pair", "x2", "features")
        .edge("ms_all", "e1", "features")
        .edge("ms_all", "e2", "features")
        .edge("ms_all", "e3", "features")
        .edge("ms_all", "e4", "features")
        .edge("ms_all", "e5", "features")
        .edge("ms_all", "e6", "features")
}

#[specforge_test(
    behavior = "surface_weighted_milestone_completion",
    verify = "weighted-milestone-completion returns effort breakdown"
)]
fn weighted_completion_answers_its_breakdown() {
    let w = json_of(
        "weighted_milestone_completion",
        json!({"milestone": "ms_all"}),
        &efforts(),
    );
    assert_eq!(
        w,
        json!({"milestone_id": "ms_all", "total_effort": 22, "done_effort": 15,
        "completion_ratio": 15.0 / 22.0, "effort_breakdown": [
            {"effort_level": "xs", "total": 1, "done": 1},
            {"effort_level": "s", "total": 1, "done": 0},
            {"effort_level": "m", "total": 2, "done": 2},
            {"effort_level": "l", "total": 1, "done": 0},
            {"effort_level": "xl", "total": 1, "done": 1},
        ]})
    );
    let human = human_of(
        "weighted_milestone_completion",
        json!({"milestone": "ms_pair"}),
        &efforts(),
    );
    assert_eq!(
        human,
        "Milestone: ms_pair\n\
         Weighted completion: 11% (1/9 effort points done)\n\
         effort  weight  features  done\n\
         xs      1       1         1\n\
         xl      8       1         0\n"
    );
}

#[specforge_test(
    behavior = "surface_weighted_milestone_completion",
    verify = "weighted-milestone-completion with unknown ID returns ENTITY_NOT_FOUND"
)]
fn weighted_completion_of_no_milestone_is_not_found() {
    let error = not_found_suggesting(
        "weighted_milestone_completion",
        json!({"milestone": "ms_al"}),
        &efforts(),
    );
    assert_eq!(error["suggestion"], "ms_all");
}

#[specforge_test(
    behavior = "pe_query_weighted_milestone_completion",
    verify = "milestone with xs(done) + xl(pending) returns ratio 1/9"
)]
fn xs_done_and_xl_pending_is_one_ninth() {
    let w = json_of(
        "weighted_milestone_completion",
        json!({"milestone": "ms_pair"}),
        &efforts(),
    );
    assert_eq!(
        (w["done_effort"].clone(), w["total_effort"].clone()),
        (json!(1), json!(9))
    );
    assert_eq!(w["completion_ratio"], 1.0 / 9.0);
}

#[specforge_test(
    behavior = "pe_query_weighted_milestone_completion",
    verify = "feature without effort defaults to m=3 weight"
)]
fn a_feature_without_effort_weighs_three() {
    let g = G::default()
        .feature("f", "done")
        .node("g", "feature", json!({"effort": "xs"}))
        .n("ms", "milestone")
        .edge("ms", "f", "features")
        .edge("ms", "g", "features");
    let w = json_of(
        "weighted_milestone_completion",
        json!({"milestone": "ms"}),
        &g,
    );
    assert_eq!(
        (w["done_effort"].clone(), w["total_effort"].clone()),
        (json!(3), json!(4))
    );
}

#[specforge_test(
    behavior = "pe_query_weighted_milestone_completion",
    verify = "empty milestone returns null completion_ratio"
)]
fn an_empty_milestone_has_no_weighted_ratio() {
    assert_eq!(
        json_of(
            "weighted_milestone_completion",
            json!({"milestone": "ms_empty"}),
            &efforts()
        ),
        json!({"milestone_id": "ms_empty", "total_effort": 0, "done_effort": 0,
            "completion_ratio": null, "effort_breakdown": []})
    );
}

#[specforge_test(
    behavior = "product_effort_weight_correctness",
    verify = "each effort level maps to its Fibonacci weight"
)]
fn each_effort_level_weighs_its_fibonacci_number() {
    for (effort, weight) in [("xs", 1), ("s", 2), ("m", 3), ("l", 5), ("xl", 8)] {
        let g = G::default()
            .node("f", "feature", json!({"effort": effort}))
            .n("ms", "milestone")
            .edge("ms", "f", "features");
        let w = json_of(
            "weighted_milestone_completion",
            json!({"milestone": "ms"}),
            &g,
        );
        assert_eq!(w["total_effort"], weight, "{effort}");
    }
}

#[specforge_test(
    behavior = "product_effort_weight_correctness",
    verify = "missing effort defaults to m weight"
)]
fn a_missing_or_unknown_effort_weighs_as_m() {
    for fields in [json!({}), json!({"effort": "huge"})] {
        let g = G::default()
            .node("f", "feature", fields.clone())
            .n("ms", "milestone")
            .edge("ms", "f", "features");
        let w = json_of(
            "weighted_milestone_completion",
            json!({"milestone": "ms"}),
            &g,
        );
        assert_eq!(w["total_effort"], 3, "{fields}");
        assert_eq!(
            w["effort_breakdown"],
            json!([{"effort_level": "m", "total": 1, "done": 0}])
        );
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
    ("term_graph", "term", "gloss"),
    ("milestone_velocity", "milestone", "ms1"),
    ("weighted_milestone_completion", "milestone", "ms1"),
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
        "coverage_matrix",
        "channel_coverage_matrix",
        "feature_overlap",
        "term_clusters",
        "term_density",
        "milestone_timeline",
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
    // Raw bytes on purpose: inputs no host sends (a missing field, a
    // malformed value), so the call is `call_export`, not `run_command`.
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
