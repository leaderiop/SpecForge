//! A surface export is granted no capability. The sandbox probe
//! (`fixtures/sandbox-probe`, vendored as `probe.wasm`) declares a CLI
//! command, an MCP tool and an MCP resource whose sandbox overrides ask for
//! every capability; each export tries to list and read a directory it is
//! told about, write a file into it, read the environment, its arguments and
//! stdin, connect to a port the test listens on, and resolve a name, and
//! reports what it got. The probe runs through the component runtime the
//! host runs every extension in, called as the host dispatches each surface.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use crate::probe_support;

use std::sync::Arc;

use probe_support::{
    ALL, Bait, PROBE, call_ports, command_input, declared_surfaces, granted_nothing, lent,
    ports_with_http, probe_runtime, reported,
};
use serde_json::{Value, json};
use specforge_effects::testing::{FakeFs, FakeGit, FakeHttp, FakeProcess};
use specforge_effects::{Ports, ProcessClass};
use specforge_protocol_types::Operation;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::protocol::load_declaration;
use specforge_wasm::testing::assert_port_contract;
use specforge_wasm::{ExtensionCalls, WasmCallResult, WasmRuntime};

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

#[specforge_test(
    invariant = "surface_sandbox_ceiling",
    verify = "a surface's sandbox override grants nothing and is W153 at load"
)]
fn the_probe_loads_with_one_w153_per_override() {
    let loaded = load_declaration(&probe_runtime(), PROBE).expect("the probe loads");
    assert!(
        loaded.warnings.iter().all(|warning| warning.code == "W153"),
        "no W138: {:?}",
        loaded.warnings
    );
    let items: Vec<&str> = [
        "#0.commands[probe]",
        "#0.mcp_tools[probe.tool]",
        "#0.mcp_resources[probe-resource]",
    ]
    .into_iter()
    .filter(|item| loaded.warnings.iter().any(|w| w.message.contains(item)))
    .collect();
    assert_eq!(items.len(), 3, "{:?}", loaded.warnings);
    assert_eq!(loaded.warnings.len(), 3, "{:?}", loaded.warnings);
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

/// Whatever the export is and whatever it is told about, the host's
/// context holds: the probe tries the root, a directory and a port the test
/// offers it, and reaches none of them.
#[specforge_test(
    behavior = "enforce_wasm_sandbox",
    verify = "an export reaches no directory: the root and a directory it is told about can be neither listed, read nor written"
)]
#[specforge_test(
    behavior = "enforce_wasm_sandbox",
    verify = "an export reaches no network: it can neither connect to a listening port nor resolve a name"
)]
#[specforge_test(
    constraint = "wasm_sandbox_enforcement",
    verify = "direct filesystem access from Wasm is blocked"
)]
#[specforge_test(
    constraint = "wasm_sandbox_enforcement",
    verify = "direct network access from Wasm is blocked"
)]
#[specforge_test(
    constraint = "wasm_sandbox_enforcement",
    verify = "an extension that tries every capability a component can reach is granted none"
)]
fn an_export_reaches_no_file_and_no_socket() {
    let runtime = probe_runtime();
    let bait = Bait::new();
    let report = ExtensionCalls::new(&runtime)
        .call_mcp_tool(
            PROBE,
            "mcp__probe_tool",
            &json!({"dir": bait.dir(), "port": bait.port()}),
        )
        .expect("the probe answers");
    granted_nothing(&report, ALL);
    bait.untouched();
}

#[specforge_test(
    behavior = "grant_a_host_port_call",
    verify = "the sandbox probe reaches each of the four ports through its grant: a command three, a collector process"
)]
fn the_probe_reaches_each_port_through_its_grant() {
    let runtime = probe_runtime();
    let (http, process) = (
        Arc::new(FakeHttp::new().route("https://example.test/a", 200, "answer")),
        Arc::new(FakeProcess::new().answering("probe-tool", ProcessClass::Exited(4), "out")),
    );
    let dir = tempfile::tempdir().unwrap();
    let ports = Ports {
        http: http.clone(),
        process: process.clone(),
        fs: Arc::new(FakeFs::new().with_file("/project/a.md", "a")),
        git: Arc::new(FakeGit::repo(dir.path(), "abc123", Vec::new())),
    };
    let lent = lent(&ports);
    let call = |port: &str, target: &str, kind| {
        reported(call_ports(&runtime, port, target, kind, Some(lent.clone())))
    };

    let answer = call("http", "https://example.test/a", Operation::Command);
    assert_eq!(answer["ok"], 200, "{answer}");
    assert_eq!(answer["bytes"], 6, "{answer}");
    let answer = call("fs", "a.md", Operation::Command);
    assert_eq!(answer["ok"], true, "{answer}");
    let answer = call("git", "HEAD", Operation::Command);
    assert_eq!(answer["ok"], "abc123", "{answer}");
    let answer = call("process", "probe-tool", Operation::Collect);
    assert_eq!(answer["ok"], 4, "{answer}");

    // The gate keeps a command from a process; asking is E090.
    let answer = call("process", "probe-tool", Operation::Command);
    assert_eq!(answer["denied"], "call_kind", "{answer}");
    assert_eq!(process.requests().len(), 1, "only the collector's ran");
    assert_eq!(lent.take_denials().len(), 1);

    // Outside its grant, nothing runs: a host it does not declare, a program it does not declare.
    let answer = call("http", "https://elsewhere.test/", Operation::Command);
    assert_eq!(answer["denied"], "scope", "{answer}");
    assert_eq!(http.requests().len(), 1);
    let answer = call("process", "cat", Operation::Collect);
    assert_eq!(answer["denied"], "scope", "{answer}");
    assert_eq!(process.requests().len(), 1);
    assert_eq!(lent.take_denials().len(), 2);
}

#[specforge_test(
    behavior = "link_host_ports",
    verify = "a pure call that calls a port is answered denied(call-kind) and no port runs"
)]
fn the_probes_pass_is_denied_its_call_kind() {
    let runtime = probe_runtime();
    let input = json!({"port": "fs", "target": "a.md"}).to_string();
    let answer = reported(runtime.call_export(PROBE, "__pass_ports", input.as_bytes()));
    assert_eq!(answer["denied"], "call_kind", "{answer}");
}

#[specforge_test(
    behavior = "grant_a_host_port_call",
    verify = "the in-process runtime answers a port call as the component runtime does"
)]
fn the_component_runtime_keeps_the_port_contract() {
    let runtime = probe_runtime();
    let http = Arc::new(FakeHttp::new().route("https://example.test/a", 200, "answer"));
    let ports = ports_with_http(http.clone());
    assert_port_contract(
        &runtime,
        PROBE,
        "cmd__ports",
        &|| lent(&ports),
        "https://example.test/a",
        "https://elsewhere.test/a",
    );
    assert_eq!(http.requests().len(), 1, "only the granted request ran");
}

#[test]
fn a_guest_after_a_trap_is_lent_a_fresh_session() {
    let runtime = probe_runtime();
    let trapped = runtime.call_with_ports(PROBE, "cmd__trap", b"{}", Operation::Command, None);
    assert!(matches!(trapped, WasmCallResult::Trap(_)));
    let answer = reported(call_ports(&runtime, "fs", "a", Operation::Command, None));
    assert_eq!(answer["denied"], "scope", "{answer}");
}
