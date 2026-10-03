//! The sandbox probe: an extension whose surface exports try every
//! capability a guest could reach (directories, the environment, arguments,
//! stdin, the network) and report what they got. The host's tests run it
//! through the component runtime to prove a surface export is granted no
//! capability, even one whose declaration asks for some. The component blob
//! is vendored at `fixtures/sandbox-probe/probe.wasm` (refresh:
//! `cd fixtures/sandbox-probe && cargo build --release --target wasm32-wasip2`,
//! then copy `target/wasm32-wasip2/release/sandbox_probe.wasm` to `probe.wasm`).

use serde_json::{json, Value};
use specforge_extension_sdk::prelude::*;
use std::io::Read;
use std::net::ToSocketAddrs;

#[specforge_extension_sdk::extension(
    name = "@test/probe",
    version = "0.1.0",
    short = "Sandbox probe"
)]
struct Probe;

/// Every surface asks for every capability: a declared override must grant
/// none of them.
fn surfaces() -> Value {
    let everything = json!({"fs_read": true, "fs_write": true, "network": true});
    json!([{
        "commands": [
            {
                "id": "probe",
                "title": "Probe the sandbox",
                "description": "Try every capability and report what was granted",
                "export": "cmd__probe",
                "args": [
                    {"name": "port", "arg_type": "integer", "description": "A port the host listens on"}
                ],
                "sandbox": everything
            },
            {
                "id": "trap",
                "title": "Trap",
                "description": "Panic",
                "export": "cmd__trap",
                "args": []
            }
        ],
        "mcp_tools": [
            {
                "name": "probe.tool",
                "description": "Try every capability and report what was granted",
                "export": "mcp__probe_tool",
                "input_schema": {"type": "object", "properties": {
                    "dir": {"type": "string"}, "port": {"type": "integer"}
                }},
                "sandbox": everything
            }
        ],
        "mcp_resources": [
            {
                "uri_template": "specforge://ext/probe/{dir}",
                "name": "probe-resource",
                "description": "Try every capability and report what was granted",
                "export": "mcp__probe_resource",
                "mime_type": "application/json",
                "sandbox": everything
            }
        ]
    }])
}

impl Contributions for Probe {
    fn contribute(c: &mut ContributionsBuilder) {
        // Surfaces are described for an extension that contributes entities.
        c.kind("probe_target", |k| {
            k.description("Something to probe").testable(false);
        });
        c.raw_category("surfaces", surfaces());
    }
}

/// What the guest got when it tried each capability, `dir` being a
/// directory it was told about (holding `secret.txt`) and `port` one the
/// host listens on.
fn report(dir: &str, port: Option<i64>) -> Value {
    let attempt = |result: std::io::Result<()>| match result {
        Ok(()) => json!({"granted": true}),
        Err(e) => json!({"granted": false, "error": e.to_string()}),
    };
    let mut stdin = Vec::new();
    let stdin_read = std::io::stdin().read_to_end(&mut stdin);
    json!({
        "read_root": attempt(std::fs::read_dir("/").map(|_| ())),
        "read_dir": attempt(std::fs::read_dir(dir).map(|_| ())),
        "read_file": attempt(std::fs::read(format!("{dir}/secret.txt")).map(|_| ())),
        "write_file": attempt(std::fs::write(format!("{dir}/probe.txt"), b"written")),
        "env_vars": std::env::vars_os().count(),
        "args": std::env::args_os().count(),
        "stdin_bytes": stdin_read.map(|_| stdin.len()).unwrap_or(0),
        "connect": attempt(match port {
            Some(port) => std::net::TcpStream::connect(("127.0.0.1", port as u16)).map(|_| ()),
            None => Err(std::io::Error::other("no port given")),
        }),
        "resolve": attempt(("localhost", 80).to_socket_addrs().map(|_| ())),
    })
}

fn dispatch(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    let value: Value = serde_json::from_slice(input).unwrap_or(Value::Null);
    let port = |args: &Value| args.get("port").and_then(Value::as_i64);
    match export {
        "cmd__probe" => {
            let input: CommandInput = match serde_json::from_value(value) {
                Ok(input) => input,
                Err(e) => return Some(Err(format!("invalid command input: {e}"))),
            };
            let args = Value::Object(input.args.clone());
            let out = json!({
                "args": args,
                "cwd": input.cwd,
                "nodes": input.graph.nodes().iter().map(|n| n.id.clone()).collect::<Vec<_>>(),
                "sandbox": report(&input.cwd, port(&args)),
            });
            let output = CommandOutput {
                exit_code: 3,
                stdout: format!("{out}\n"),
                stderr: "probed\n".to_string(),
            };
            Some(Ok(output.to_bytes()))
        }
        "cmd__trap" => panic!("the probe trapped"),
        "mcp__probe_tool" => {
            let dir = value.get("dir").and_then(Value::as_str).unwrap_or("/");
            Some(Ok(report(dir, port(&value)).to_string().into_bytes()))
        }
        "mcp__probe_resource" => {
            let uri = value.get("uri").and_then(Value::as_str).unwrap_or_default();
            let dir = uri.strip_prefix("specforge://ext/probe").unwrap_or("/");
            let content = json!({"uri": uri, "sandbox": report(dir, None)});
            Some(Ok(json!({"content": content.to_string(), "mime_type": "application/json"})
                .to_string()
                .into_bytes()))
        }
        _ => None,
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build, handler = dispatch);
