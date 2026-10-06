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
use specforge_protocol_types::{CommandInput, RawGraph};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::ExtensionCalls;
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

/// The probe command's input: the bait's port as its arg, the bait's
/// directory as the project root, an empty graph.
fn command_input(bait: &Bait) -> CommandInput<RawGraph> {
    CommandInput {
        args: json!({"port": bait.port()}).as_object().unwrap().clone(),
        cwd: bait.dir(),
        graph: RawGraph::new(r#"{"nodes":[],"edges":[]}"#.to_string()).unwrap(),
        ..CommandInput::default()
    }
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
    let output = ExtensionCalls::new(&runtime)
        .run_command(PROBE, "cmd__probe", &command_input(&bait))
        .expect("the probe answers");
    let out: Value = serde_json::from_str(&output.stdout).unwrap();
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
    let report = ExtensionCalls::new(&runtime)
        .call_mcp_tool(PROBE, "mcp__probe_tool", &input)
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
    let read = ExtensionCalls::new(&runtime)
        .read_mcp_resource(PROBE, "mcp__probe_resource", &uri)
        .expect("the probe answers");
    assert_eq!(read.mime_type, "application/json");
    let content: Value = serde_json::from_str(&read.content).unwrap();
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
    let calls = ExtensionCalls::new(&runtime);
    let output = calls
        .run_command(PROBE, "cmd__probe", &command_input(&bait))
        .unwrap();
    let command: Value = serde_json::from_str(&output.stdout).unwrap();
    let tool = calls
        .call_mcp_tool(
            PROBE,
            "mcp__probe_tool",
            &json!({"dir": bait.dir(), "port": bait.port()}),
        )
        .unwrap();
    let uri = format!("specforge://ext/probe{}", bait.dir());
    let resource = calls
        .read_mcp_resource(PROBE, "mcp__probe_resource", &uri)
        .unwrap();
    let resource: Value = serde_json::from_str(&resource.content).unwrap();

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

/// The component runtime keeps the contract every adapter of the
/// `WasmRuntime` port keeps (`specforge_wasm::testing::assert_runtime_contract`,
/// also run over the in-process runtime): the probe's `trap` command panics.
#[specforge_test(
    behavior = "call_extension_exports",
    verify = "both runtimes report an unknown extension, an unrouted export, a guest error and a guest panic as traps"
)]
#[specforge_test(port = "WasmRuntime", verify = "WasmRuntime contract is satisfied")]
fn the_component_runtime_keeps_the_runtime_contract() {
    specforge_wasm::testing::assert_runtime_contract(&probe_runtime(), PROBE, "cmd__trap");
}
