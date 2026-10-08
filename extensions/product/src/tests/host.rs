//! The command suites' harness: each `cmd__product_<id>` is called as the
//! host calls it, `ExtensionCalls::run_command` with the typed
//! `CommandInput` and the strictly decoded answer (ADR 0013), over the
//! in-process runtime serving this crate's own declaration through the
//! guest's routing (`guest_call`, what `component_guest!` calls). So a
//! change to a command is tested before any blob is built. What only the
//! vendored blob proves is `specforge-component`'s (`tests/product_blob.rs`).

use serde_json::{json, Value};
use specforge_protocol_types::{CommandEvidence, CommandFormat, CommandInput, RawGraph};
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::ExtensionCalls;

pub(super) const PRODUCT: &str = "@specforge/product";

/// A graph built entity by entity, in the graph export's shape.
#[derive(Default)]
pub(super) struct G {
    pub(super) nodes: Vec<Value>,
    pub(super) edges: Vec<Value>,
}

impl G {
    pub(super) fn node(mut self, id: &str, kind: &str, fields: Value) -> Self {
        self.nodes
            .push(json!({"id": id, "kind": kind, "title": id, "fields": fields}));
        self
    }

    pub(super) fn n(self, id: &str, kind: &str) -> Self {
        self.node(id, kind, json!({}))
    }

    /// A feature with `status`.
    pub(super) fn feature(self, id: &str, status: &str) -> Self {
        self.node(id, "feature", json!({"status": status}))
    }

    pub(super) fn edge(mut self, source: &str, target: &str, label: &str) -> Self {
        self.edges
            .push(json!({"source": source, "target": target, "label": label}));
        self
    }

    pub(super) fn graph(&self) -> Value {
        json!({"nodes": self.nodes, "edges": self.edges})
    }
}

/// What a command printed: its exit code, stdout and stderr.
pub(super) struct Out {
    pub(super) exit: i64,
    pub(super) stdout: String,
    pub(super) stderr: String,
}

impl Out {
    /// Its stdout, one JSON object.
    pub(super) fn json(&self) -> Value {
        assert_eq!(self.exit, 0, "{}", self.stderr);
        assert!(self.stderr.is_empty(), "{}", self.stderr);
        let value: Value = serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {}", self.stdout));
        assert!(value.is_object(), "one root object: {value}");
        value
    }

    /// Its stderr, one error object, and nothing on stdout.
    pub(super) fn error(&self) -> Value {
        assert_ne!(self.exit, 0);
        assert_eq!(self.stdout, "", "nothing on stdout");
        serde_json::from_str(&self.stderr)
            .unwrap_or_else(|e| panic!("stderr is not JSON ({e}): {}", self.stderr))
    }
}

pub(super) fn runtime() -> InProcessRuntime {
    InProcessRuntime::new().with(crate::specforge_extension_build)
}

/// Run `cmd__product_<id>` with `args` over `g` in `format`.
pub(super) fn run_in(
    runtime: &InProcessRuntime,
    id: &str,
    args: Value,
    g: &G,
    format: &str,
) -> Out {
    run_with(runtime, id, args, g, format, None)
}

/// [`run_in`], with `evidence` as the input's recorded-test evidence.
pub(super) fn run_with(
    runtime: &InProcessRuntime,
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
pub(super) fn run_input(
    runtime: &InProcessRuntime,
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
    pub(super) static RUNTIME: InProcessRuntime = runtime();
}

pub(super) fn json_of(id: &str, args: Value, g: &G) -> Value {
    RUNTIME.with(|runtime| run_in(runtime, id, args, g, "json").json())
}

pub(super) fn human_of(id: &str, args: Value, g: &G) -> String {
    let out = run_in(&runtime(), id, args, g, "human");
    assert_eq!(out.exit, 0, "{}", out.stderr);
    out.stdout
}

/// The graph most tests ask about: milestones, journeys, personas, channels,
/// a deliverable and a term over three features.
pub(super) fn plan() -> G {
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

/// Deliverables over journeys and modules: d1 holds j1, j2 and mod1, d2
/// only mod2, d3 nothing, d4 a journey without a persona. f1 is reached
/// both ways, f2 through journeys, f3 through modules; f4 depends on f1,
/// f5 on f4, and f6 only relates to f1.
pub(super) fn shipping() -> G {
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
