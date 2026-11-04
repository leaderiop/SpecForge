//! The operation a call performs on a loaded extension (ADR 0013): a wire type, so a capability names the call
//! kinds it is for (ADR 0335 D2).

use std::fmt;

use serde::{Deserialize, Serialize};

/// The operation a call performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Handshake,
    Describe,
    Command,
    McpTool,
    McpResource,
    Pass,
    Collect,
    Validate,
    Scan,
    Migrate,
    Prompt,
    Render,
    Hook,
}

impl Operation {
    /// Whether the host writes its answer's text into its own messages, help and listings (the
    /// declaration, a pass's diagnostics and summary, a validator's verdict, a prompt's messages), so the answer is read
    /// printable (ADR 0244). A command's, an MCP tool's or resource's answer is the extension's own
    /// output, and a collector's or scanner's is data matched against the project: read as answered.
    pub const fn reads_text(self) -> bool {
        matches!(
            self,
            Operation::Handshake
                | Operation::Describe
                | Operation::Pass
                | Operation::Validate
                | Operation::Prompt
                | Operation::Render
                | Operation::Hook
        )
    }

    /// Whether a call of this kind may use a `specforge:host` port at all (ADR 0335 D4): the rest get what they need
    /// as their input.
    pub const fn is_effectful(self) -> bool {
        matches!(
            self,
            Operation::Command | Operation::McpTool | Operation::McpResource | Operation::Collect
        )
    }

    /// Every operation.
    pub const ALL: [Operation; 13] = [
        Operation::Handshake,
        Operation::Describe,
        Operation::Command,
        Operation::McpTool,
        Operation::McpResource,
        Operation::Pass,
        Operation::Collect,
        Operation::Validate,
        Operation::Scan,
        Operation::Migrate,
        Operation::Prompt,
        Operation::Render,
        Operation::Hook,
    ];

    /// How a message names the operation.
    pub fn label(self) -> &'static str {
        match self {
            Operation::Handshake => "handshake",
            Operation::Describe => "describe",
            Operation::Command => "command",
            Operation::McpTool => "MCP tool",
            Operation::McpResource => "MCP resource",
            Operation::Pass => "compiler pass",
            Operation::Collect => "collector",
            Operation::Validate => "custom validator",
            Operation::Scan => "scanner",
            Operation::Migrate => "migration hook",
            Operation::Prompt => "MCP prompt",
            Operation::Render => "renderer",
            Operation::Hook => "hook",
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests;
