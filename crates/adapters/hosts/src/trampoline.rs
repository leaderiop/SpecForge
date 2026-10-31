//! The body of a slash command's skill: a trampoline to the prompt or command it stands for (ADR 0292).

use specforge_ops::host::{Host, SlashCall};

/// The flag a CLI fallback names its host with.
const HOST_FLAG: &str = "--host";

/// `head` (how the host hands the user's text over), then the call.
pub fn body(host: Host, head: &str, call: &SlashCall) -> String {
    let host = host.as_str();
    match call {
        SlashCall::Prompt { mcp_name, cli } => format!(
            "{head}Call the MCP tool `specforge.prompt` with `{{\"name\": \"{mcp_name}\", \"argv\": <that text>, \"host\": \"{host}\"}}` and follow the workflow it returns. A step that names a tool runs that tool; a refused step (`gate_failed`) is reported to the user with its diagnostics, never worked around. Without the MCP server, run `{cli} {HOST_FLAG} {host} -- <that text>` and follow its output."
        ),
        SlashCall::Command { mcp_tool, cli } => format!(
            "{head}Call the MCP tool `{mcp_tool}` with the arguments read from that text by the tool's schema, and report what it returns. A writing command previews when the user asks for it (`dry_run: true`) and otherwise applies. A refused step (`gate_failed`) is reported to the user with its diagnostics, never worked around. Without the MCP server, run `{cli} {HOST_FLAG} {host} <that text>` and report its output."
        ),
    }
}
