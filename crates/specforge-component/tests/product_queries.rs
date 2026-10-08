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
