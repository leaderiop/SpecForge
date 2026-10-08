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
