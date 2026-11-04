//! The gate (ADR 0335 D2.2): which ports, read or written, each call kind may use at all, whatever an extension
//! declares. It is the host's, fixed: a pure call kind never writes and never reaches the network or a process.

use specforge_protocol_types::Operation;

/// An effect port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Port {
    Http,
    Fs,
    Process,
    Git,
}

impl Port {
    pub fn name(self) -> &'static str {
        match self {
            Port::Http => "http",
            Port::Fs => "fs",
            Port::Process => "process",
            Port::Git => "git",
        }
    }
}

/// Whether a call of kind `op` may use `port`, writing when `write` (fs and git only).
pub fn gate(op: Operation, port: Port, write: bool) -> bool {
    use Operation::*;
    match (port, write) {
        (Port::Fs, false) => !matches!(op, Handshake | Describe),
        (Port::Git, false) => matches!(
            op,
            Pass | Validate | Hook | Collect | Command | McpTool | McpResource | CheckValues
        ),
        (Port::Fs | Port::Git, true) => matches!(op, Collect | Command),
        (Port::Http, _) => matches!(op, Collect | Command | McpTool | McpResource | CheckValues),
        (Port::Process, _) => op == Collect,
    }
}
