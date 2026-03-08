// Surface contribution types — CLI commands, MCP tools, MCP resources
//
// Extensions declare surface contributions in their manifest to extend
// the CLI and MCP server dynamically. Core dispatches to Wasm exports
// using the naming convention cmd__{id} and mcp__{name}.

use types/core
use types/wasm
use types/mcp

// ── Surface Contribution Container ──────────────────────────

type SurfaceContributions {
  commands          CommandContribution[]      @optional
  mcp_tools         McpToolContribution[]      @optional
  mcp_resources     McpResourceContribution[]  @optional
}

// ── CLI Command Contributions ───────────────────────────────

type CommandContribution {
  id                string              @readonly
  title             string
  description       string              @optional
  category          string              @optional
  // Wasm export name: cmd__{id}
  export            string              @readonly
  args              CommandArg[]         @optional
  sandbox           SurfaceSandboxOverride @optional
}

type CommandArg {
  name              string              @readonly
  arg_type          CommandArgType
  required          boolean             @optional
  default_value     string              @optional
  description       string              @optional
  // For enum_arg: allowed values
  values            string[]            @optional
}

type CommandArgType = string_arg | path_arg | bool_arg | enum_arg | integer_arg

type CommandInput {
  args              FieldMap
  flags             FieldMap            @optional
  cwd               string
}

type CommandOutput {
  exit_code         integer
  stdout            string              @optional
  stderr            string              @optional
}

// ── MCP Tool Contributions ──────────────────────────────────

type McpToolContribution {
  name              string              @readonly
  description       string              @optional
  category          McpToolCategory     @optional
  // Wasm export name: mcp__{name}
  export            string              @readonly
  input_schema      JsonSchema
  sandbox           SurfaceSandboxOverride @optional
}

// ── MCP Resource Contributions ──────────────────────────────

type McpResourceContribution {
  uri_template      string              @readonly
  name              string              @readonly
  description       string              @optional
  // Wasm export name: mcp__{name}
  export            string              @readonly
  mime_type         string              @optional
  sandbox           SurfaceSandboxOverride @optional
}

// ── Sandbox Override ────────────────────────────────────────

// Per-contribution sandbox ceiling override. Can only restrict
// below the type ceiling, never expand beyond it.
type SurfaceSandboxOverride {
  fs_read           string[]            @optional
  fs_write          string[]            @optional
  // Domain allowlist for network access
  network           string[]            @optional
}

// ── Auto-Promotion ──────────────────────────────────────────

type AutoPromotedMcpTool {
  source_command    string              @readonly
  source_extension  string              @readonly
  // MCP tool name: specforge.{ext_short}.{cmd_id}
  mcp_tool_name     string              @readonly
  derived_input_schema JsonSchema
}

// ── Surface Registry ────────────────────────────────────────

type SurfaceRegistryEntry {
  surface_type      SurfaceType
  contribution_name string              @readonly
  extension_name    string              @readonly
  export_name       string              @readonly
  enabled           boolean
}

type SurfaceType = command | mcp_tool | mcp_resource | auto_promoted_tool

// ── Surface Errors ──────────────────────────────────────────

type SurfaceError {
  extension_name    string              @readonly
  surface_type      SurfaceType
  contribution_id   string
  message           string
  export_name       string              @optional
}
