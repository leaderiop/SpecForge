//! The agent hosts' adapters (ADR 0292): one [`HostRenderer`] per host, which knows the host's file layout, config
//! formats and hook protocol and nothing of what a contribution means. The CLI wires them to the operations.

mod claude_code;
mod codex;
mod cursor;
mod hook_json;
mod json_merge;
mod markdown;
mod toml_merge;
mod tools;
mod trampoline;

use specforge_ops::host::{Host, HostRenderer};

/// The adapter of `host`; none for a host whose adapter is not written yet.
pub fn renderer(host: Host) -> Option<Box<dyn HostRenderer>> {
    match host {
        Host::ClaudeCode => Some(Box::new(claude_code::ClaudeCode)),
        Host::Codex => Some(Box::new(codex::Codex)),
        Host::Cursor => Some(Box::new(cursor::Cursor)),
    }
}

/// Every adapter there is, in the order of [`Host::ALL`].
pub fn renderers() -> Vec<Box<dyn HostRenderer>> {
    Host::ALL.into_iter().filter_map(renderer).collect()
}
