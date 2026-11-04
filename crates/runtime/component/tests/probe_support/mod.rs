//! The sandbox probe (`fixtures/sandbox-probe`, `specforge_component::committed::PROBE`)
//! and what a test needs to run it: the bait it is told about and must not
//! reach, and what its report says. Shared by `sandbox_probe.rs` and
//! `sandbox_limits.rs`.

// Each test binary uses the part of the probe it needs.
#![allow(
    dead_code,
    reason = "shared by sandbox_probe and sandbox_limits: each binary uses part of the probe"
)]

use serde_json::{Value, json};
use specforge_component::ComponentRuntime;
use specforge_effects::testing::{self, MemoryConsent};
use specforge_effects::{
    EffectJournal, EffectsCeiling, Grant, Http, HttpOutcome, HttpRequest, OutcomeClass, Ports,
    Provenance, Redirects, Roots,
};
use specforge_effects::{ResolveOutcome, ResolveRequest, TlsOutcome, TlsRequest};
use specforge_protocol_types::{CommandInput, Operation, RawGraph};
use specforge_wasm::PortCall;
use specforge_wasm::protocol::load_declaration;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};
use std::net::TcpListener;
use std::sync::{Arc, Barrier};
use std::time::Duration;

pub const PROBE: &str = "@test/probe";

pub fn probe_runtime() -> ComponentRuntime {
    let path = specforge_component::committed::PROBE.path();
    let blob = std::fs::read(path).expect("vendored sandbox probe blob");
    let runtime = ComponentRuntime::new();
    runtime
        .load(PROBE, &blob, specforge_wasm::Provenance::third_party(&blob))
        .expect("the probe instantiates");
    runtime
}

/// The surfaces the probe declares, as it describes them.
pub fn declared_surfaces(runtime: &ComponentRuntime) -> Value {
    match runtime.call_export(PROBE, "__describe", br#"{"category":"surfaces"}"#) {
        WasmCallResult::Ok(bytes) => {
            let envelope: Value = serde_json::from_slice(&bytes).unwrap();
            envelope["items"][0].clone()
        }
        WasmCallResult::Trap(trap) => panic!("describe trapped: {trap:?}"),
    }
}

/// A directory holding `secret.txt` and a port with a listener behind it:
/// what the probe is told about and must not reach.
pub struct Bait {
    dir: tempfile::TempDir,
    listener: TcpListener,
}

impl Bait {
    pub fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("secret.txt"), "secret").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        Bait { dir, listener }
    }

    pub fn dir(&self) -> String {
        self.dir.path().display().to_string()
    }

    pub fn port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }

    /// Nothing reached the bait: no file was written and no connection
    /// arrived.
    pub fn untouched(&self) {
        assert!(
            !self.dir.path().join("probe.txt").exists(),
            "the probe wrote a file"
        );
        assert_eq!(
            self.listener.accept().map(|_| ()).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "the probe connected"
        );
    }
}

/// `report` says no capability was granted: every attempt failed, and the
/// environment, arguments and stdin were empty.
pub fn granted_nothing(report: &Value, attempts: &[&str]) {
    for attempt in attempts {
        assert_eq!(report[attempt]["granted"], false, "{attempt}: {report}");
    }
    assert_eq!(report["env_vars"], 0, "{report}");
    assert_eq!(report["args"], 0, "{report}");
    assert_eq!(report["stdin_bytes"], 0, "{report}");
}

/// The probe command's input: the bait's port as its arg, the bait's
/// directory as the project root, an empty graph.
pub fn command_input(bait: &Bait) -> CommandInput<RawGraph> {
    CommandInput {
        args: json!({"port": bait.port()}).as_object().unwrap().clone(),
        cwd: bait.dir(),
        graph: RawGraph::new(r#"{"nodes":[],"edges":[]}"#.to_string()).unwrap(),
        ..CommandInput::default()
    }
}

pub const ALL: &[&str] = &[
    "read_root",
    "read_dir",
    "read_file",
    "write_file",
    "connect",
    "resolve",
];

// ── The probe's `ports` export (ADR 0335 D4) ─────────────────────────────────

/// What the probe is lent: `ports` under the probe's own grant, every approval given. Declared by an instance of its
/// own, as a load applies the limits it declares, over those a test set.
pub fn lent(ports: &Ports) -> PortCall {
    let declared = load_declaration(&probe_runtime(), PROBE)
        .expect("the probe declares itself")
        .declaration;
    let roots = Roots::new("/project", "/project/spec");
    let grant = Grant::decide(
        &declared,
        &Provenance::third_party(PROBE.as_bytes()),
        &EffectsCeiling::default(),
        &roots,
        &MemoryConsent::approving_everything(),
    );
    PortCall::online(ports, EffectJournal::detached(), roots, grant, Vec::new())
}

/// What the probe's `cmd__ports` reports of calling `port` on `target`.
pub fn call_ports(
    runtime: &ComponentRuntime,
    port: &str,
    target: &str,
    kind: Operation,
    lent: Option<PortCall>,
) -> WasmCallResult {
    let input = json!({"port": port, "target": target}).to_string();
    runtime.call_with_ports(PROBE, "cmd__ports", input.as_bytes(), kind, lent)
}

/// The JSON a call answered.
pub fn reported(result: WasmCallResult) -> Value {
    match result {
        WasmCallResult::Ok(bytes) => {
            serde_json::from_slice(&bytes).expect("the probe reports JSON")
        }
        WasmCallResult::Trap(trap) => panic!("the probe trapped: {trap:?}"),
    }
}

/// An HTTP port that waits on `gate` once started, then sleeps `sleep`, then answers 200: a port call that takes as
/// long as a test needs.
pub struct SlowHttp {
    pub started: Arc<Barrier>,
    pub sleep: Duration,
}

impl Http for SlowHttp {
    fn request(&self, request: &HttpRequest, _redirects: &dyn Redirects) -> HttpOutcome {
        self.started.wait();
        std::thread::sleep(self.sleep);
        HttpOutcome {
            class: OutcomeClass::Ok,
            status: Some(200),
            final_url: request.url.clone(),
            headers: Vec::new(),
            body: Vec::new(),
            detail: None,
        }
    }

    fn resolve(&self, request: &ResolveRequest) -> ResolveOutcome {
        testing::FakeHttp::new().resolve(request)
    }

    fn tls(&self, request: &TlsRequest) -> TlsOutcome {
        testing::FakeHttp::new().tls(request)
    }
}

/// [`testing::ports`] with `http`.
pub fn ports_with_http(http: Arc<dyn Http>) -> Ports {
    Ports {
        http,
        ..testing::ports()
    }
}
