//! The one rule for an explicit MCP tool's input schema, run by the host
//! and by the SDK guest alike (ADR 0239), as [`crate::command_args`] is a
//! command's: the names the host's call target takes out of every MCP call,
//! which no tool's schema may declare.

use serde_json::Value;
use std::fmt;

/// The arguments the host's MCP call target reads from every call that
/// reads a project (`path`, naming it): the host takes them out of the call
/// before a tool's schema check or export sees them, so no explicit tool may
/// declare one. The host's `specforge_mcp::target::TargetArgument` names
/// exactly these.
pub const TARGET_ARGUMENTS: &[&str] = &["path"];

/// Every argument the host owns in a tool call: the target's, and `settings`, which the host adds to every call to
/// carry the extension's own (ADR 0288; a client's is dropped). No explicit tool may declare one.
pub const HOST_ARGUMENTS: &[&str] = &["path", "settings"];

/// Why the host refuses an explicit MCP tool: MCP serves no tool for it
/// (I017), and the SDK refuses to build an extension declaring it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRefusal {
    /// The target argument its input schema declares.
    pub argument: &'static str,
}

impl fmt::Display for ToolRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let takes = if TARGET_ARGUMENTS.contains(&self.argument) {
            "the call target takes"
        } else {
            "the host sends with every call"
        };
        write!(
            f,
            "its input schema declares '{}', which {takes}",
            self.argument
        )
    }
}

/// The refusal of `input_schema`, if any: the first host argument it
/// declares as a top-level property or a required name. A schema that is no
/// object declares nothing here (E055 is the registry's refusal of it).
pub fn refusal(input_schema: &Value) -> Option<ToolRefusal> {
    let declares = |name: &str| {
        input_schema["properties"].get(name).is_some()
            || input_schema["required"]
                .as_array()
                .is_some_and(|required| required.iter().any(|n| n == name))
    };
    HOST_ARGUMENTS
        .iter()
        .find(|name| declares(name))
        .map(|argument| ToolRefusal { argument })
}

#[cfg(test)]
mod tests;
