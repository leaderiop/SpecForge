//! The surface descriptors: the commands, MCP tools and resources, prompts, agents, skills, slash commands and hooks an
//! extension contributes (ADR 0012, ADR 0292). Split out of `lib.rs` before K20c's hook members landed.

use serde::{Deserialize, Serialize};

use crate::*;

/// Describes all surface contributions (CLI commands, MCP tools, MCP resources).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SurfaceDescriptor {
    #[serde(default)]
    pub commands: Vec<CommandDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_tools: Vec<McpToolDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_resources: Vec<McpResourceDescriptor>,
    /// The MCP prompts it contributes (protocol 1.10.0, ADR 0291).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<PromptDescriptor>,
    /// Agents, skills, slash commands and hooks for a host (protocol 1.10.0, ADR 0292).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<AgentDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<SkillDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slash_commands: Vec<SlashCommandDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<HookDescriptor>,
}

/// Describes a CLI command contributed by an extension.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandDescriptor {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub export: String,
    #[serde(default)]
    pub args: Vec<CommandArgDescriptor>,
    /// It answers an edit plan the host applies (protocol 1.9.0, ADR 0289).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub writes: bool,
    /// Records what a person says: the host runs it only for a person at a terminal (protocol 1.12.0, ADR 0301).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub human: bool,
    /// Local codes of the extension: the command is refused (`gate_failed`) while one fails in the `subject`'s scope
    /// (protocol 1.10.0, ADR 0291).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<String>,
    /// The arg holding the entity the command acts on; it scopes `gates`. Absent, the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reads_state: bool,
    /// The project files the command reads; the input's `files` carries their facts (protocol 1.11.0, ADR 0293).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<FileRead>,
}

/// Describes an argument to a CLI command.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandArgDescriptor {
    pub name: String,
    pub arg_type: CommandArgType,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The least value of an integer arg: `0` for a count. Absent, any
    /// integer (wire compatible both ways: a host or guest that does not
    /// know it ignores it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
}

/// Type of a command argument.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandArgType {
    #[default]
    String,
    Path,
    Bool,
    #[serde(rename = "enum")]
    Enum {
        values: Vec<String>,
    },
    Integer,
}

/// Describes an MCP tool contributed by an extension.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub export: String,
    pub input_schema: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

/// Describes an MCP resource contributed by an extension.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpResourceDescriptor {
    pub uri_template: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub export: String,
    pub mime_type: String,
}
