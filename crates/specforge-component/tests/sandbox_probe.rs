//! A surface export is granted no capability. The sandbox probe
//! (`fixtures/sandbox-probe`, vendored as `probe.wasm`) declares a CLI
//! command, an MCP tool and an MCP resource whose sandbox overrides ask for
//! every capability; each export tries to list and read a directory it is
//! told about, write a file into it, read the environment, its arguments and
//! stdin, connect to a port the test listens on, and resolve a name, and
//! reports what it got. The probe runs through the component runtime the
//! host runs every extension in, called as the host dispatches each surface.

use serde_json::{Value, json};
use specforge_component::ComponentRuntime;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};
use std::net::TcpListener;
use std::path::Path;

const PROBE: &str = "@test/probe";

fn probe_runtime() -> ComponentRuntime {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sandbox-probe/probe.wasm");
    let blob = std::fs::read(path).expect("vendored sandbox probe blob");
    let runtime = ComponentRuntime::new();
    runtime
        .load_module_bytes(PROBE, &blob)
        .expect("the probe instantiates");
    runtime
}

/// The surfaces the probe declares, as it describes them.
fn declared_surfaces(runtime: &ComponentRuntime) -> Value {
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
struct Bait {
    dir: tempfile::TempDir,
    listener: TcpListener,
}

impl Bait {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("secret.txt"), "secret").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        Bait { dir, listener }
    }

    fn dir(&self) -> String {
        self.dir.path().display().to_string()
    }

    fn port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }

    /// Nothing reached the bait: no file was written and no connection
    /// arrived.
    fn untouched(&self) {
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
fn granted_nothing(report: &Value, attempts: &[&str]) {
    for attempt in attempts {
        assert_eq!(report[attempt]["granted"], false, "{attempt}: {report}");
    }
    assert_eq!(report["env_vars"], 0, "{report}");
    assert_eq!(report["args"], 0, "{report}");
    assert_eq!(report["stdin_bytes"], 0, "{report}");
}

const ALL: &[&str] = &[
    "read_root",
    "read_dir",
    "read_file",
    "write_file",
    "connect",
    "resolve",
];

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "a cmd__ export is granted no capability, whatever sandbox its declaration asks for"
)]
fn a_command_export_is_granted_no_capability() {
    let runtime = probe_runtime();
    let declared = declared_surfaces(&runtime);
    assert_eq!(
        declared["commands"][0]["sandbox"],
        json!({"fs_read": true, "fs_write": true, "network": true})
    );

    let bait = Bait::new();
    let input = json!({
        "args": {"port": bait.port()},
        "cwd": bait.dir(),
        "graph": {"nodes": [], "edges": []},
    });
    let output = specforge_wasm::dispatch_surface_command(
        PROBE,
        "cmd__probe",
        input.to_string().as_bytes(),
        &runtime,
    )
    .expect("the probe answers");
    let out: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(out["cwd"], bait.dir(), "told the project root");
    granted_nothing(&out["sandbox"], ALL);
    bait.untouched();
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_tool",
    verify = "an mcp__ tool export is granted no capability, whatever sandbox its declaration asks for"
)]
fn an_mcp_tool_export_is_granted_no_capability() {
    let runtime = probe_runtime();
    let declared = declared_surfaces(&runtime);
    assert_eq!(declared["mcp_tools"][0]["sandbox"]["network"], true);

    let bait = Bait::new();
    let input = json!({"dir": bait.dir(), "port": bait.port()});
    let report = specforge_wasm::dispatch_surface_mcp_tool(
        PROBE,
        "mcp__probe_tool",
        input.to_string().as_bytes(),
        &runtime,
    )
    .expect("the probe answers");
    granted_nothing(&report, ALL);
    bait.untouched();
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_resource",
    verify = "fs_write denied for resource contributions"
)]
fn an_mcp_resource_export_cannot_write() {
    let runtime = probe_runtime();
    let declared = declared_surfaces(&runtime);
    assert_eq!(declared["mcp_resources"][0]["sandbox"]["fs_write"], true);

    let bait = Bait::new();
    let uri = format!("specforge://ext/probe{}", bait.dir());
    let (content, mime) =
        specforge_wasm::dispatch_surface_mcp_resource(PROBE, "mcp__probe_resource", &uri, &runtime)
            .expect("the probe answers");
    assert_eq!(mime, "application/json");
    let content: Value = serde_json::from_slice(&content).unwrap();
    assert_eq!(content["uri"], uri);
    assert_eq!(
        content["sandbox"]["write_file"]["granted"], false,
        "{content}"
    );
    granted_nothing(
        &content["sandbox"],
        &["read_root", "read_dir", "read_file", "resolve"],
    );
    bait.untouched();
}

#[specforge_test(
    invariant = "surface_sandbox_ceiling",
    verify = "a surface export whose sandbox override asks for every capability is granted none"
)]
fn no_override_expands_the_ceiling() {
    let runtime = probe_runtime();
    let declared = declared_surfaces(&runtime);
    let everything = json!({"fs_read": true, "fs_write": true, "network": true});
    for surface in ["commands", "mcp_tools", "mcp_resources"] {
        assert_eq!(declared[surface][0]["sandbox"], everything, "{surface}");
    }

    let bait = Bait::new();
    let command = json!({"args": {"port": bait.port()}, "cwd": bait.dir(), "graph": {"nodes": [], "edges": []}});
    let output = specforge_wasm::dispatch_surface_command(
        PROBE,
        "cmd__probe",
        command.to_string().as_bytes(),
        &runtime,
    )
    .unwrap();
    let command: Value = serde_json::from_slice(&output.stdout).unwrap();
    let tool = specforge_wasm::dispatch_surface_mcp_tool(
        PROBE,
        "mcp__probe_tool",
        json!({"dir": bait.dir(), "port": bait.port()})
            .to_string()
            .as_bytes(),
        &runtime,
    )
    .unwrap();
    let uri = format!("specforge://ext/probe{}", bait.dir());
    let (resource, _) =
        specforge_wasm::dispatch_surface_mcp_resource(PROBE, "mcp__probe_resource", &uri, &runtime)
            .unwrap();
    let resource: Value = serde_json::from_slice(&resource).unwrap();

    granted_nothing(&command["sandbox"], ALL);
    granted_nothing(&tool, ALL);
    granted_nothing(
        &resource["sandbox"],
        &[
            "read_root",
            "read_dir",
            "read_file",
            "write_file",
            "resolve",
        ],
    );
    bait.untouched();
}
