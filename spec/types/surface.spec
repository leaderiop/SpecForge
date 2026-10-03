// Surface contribution types — CLI commands, MCP tools, MCP resources
//
// Extensions declare surface contributions in their manifest to extend
// the CLI and MCP server dynamically. Core dispatches to Wasm exports
// using the naming convention cmd__{id} and mcp__{name}.

use "types/core"
use "types/graph"
use "types/mcp"
use "types/wasm"

// ── Surface Contribution Container ──────────────────────────

type SurfaceContributions {
  commands      CommandContribution[]     @optional
  mcp_tools     McpToolContribution[]     @optional
  mcp_resources McpResourceContribution[] @optional
  verify unit "SurfaceContributions schema is valid"
}

// ── CLI Command Contributions ───────────────────────────────

type CommandContribution {
  id          string                 @readonly
  title       string
  description string                 @optional
  category    string                 @optional
  // Wasm export name: cmd__{id}
  export      string                 @readonly
  args        CommandArg[]           @optional
  sandbox     SurfaceSandboxOverride @optional
  verify unit "CommandContribution schema is valid"
}

// On the command line (specforge {ext_short} {id with _ as -}), a required
// arg is positional, in declaration order; any other, and every bool_arg, is
// --{name with _ as -}. --path names the project and is the host's on every
// command, as --help is, and --format (human, the default, or json): a
// command with an arg named path, help or format, or two args of one name,
// is refused on the command line (exit 2). A list of values is a string arg
// the command splits (--tags a,b): there is no list arg type (ADR 0011).
type CommandArg {
  name          string   @readonly
  arg_type      CommandArgType
  required      boolean  @optional
  default_value string   @optional
  description   string   @optional
  // For enum_arg: allowed values
  values        string[] @optional
  verify unit "CommandArg schema is valid"
}

type CommandArgType = string_arg | path_arg | bool_arg | enum_arg | integer_arg

// What a cmd__{id} export receives. args holds the declared args the
// caller set (the CLI's parsed command line, or the auto-promoted MCP
// tool's arguments), typed as declared; cwd is the project root; graph is
// the compiled project's graph in the graph export's shape (entities
// sorted by id, edges by source, target and label); format is the output
// the caller asked for (the CLI's --format, json over MCP); today is the
// host's date at the call, UTC. The export reads no files and no clock:
// the same call serves the CLI and MCP (ADR 0011).
type CommandInput {
  args   FieldMap
  cwd    string
  graph  Graph
  format CommandFormat
  today  string
  verify unit "CommandInput schema is valid"
}

// human: the extension's layout for a reader (a table with a header row
// where the payload is tabular); json: one root object, the command's
// payload type. The host owns the flag; the extension renders both.
type CommandFormat = human | json

type CommandOutput {
  exit_code integer
  stdout    string @optional
  stderr    string @optional
  verify unit "CommandOutput schema is valid"
}

// ── MCP Tool Contributions ──────────────────────────────────

// input_schema and output_schema must be JSON objects: a tool with
// another value is E055 and is not registered.
type McpToolContribution {
  name          string                 @readonly
  description   string
  category      McpToolCategory        @optional
  // Wasm export name: mcp__{name}
  export        string                 @readonly
  input_schema  JsonSchema
  output_schema JsonSchema             @optional
  sandbox       SurfaceSandboxOverride @optional
  verify unit "McpToolContribution schema is valid"
}

// ── MCP Resource Contributions ──────────────────────────────

type McpResourceContribution {
  uri_template string                 @readonly
  name         string                 @readonly
  description  string                 @optional
  // Wasm export name: mcp__{name}
  export       string                 @readonly
  mime_type    string                 @optional
  sandbox      SurfaceSandboxOverride @optional
  verify unit "McpResourceContribution schema is valid"
}

// ── Sandbox Override ────────────────────────────────────────

// Per-contribution sandbox ceiling override. Can only restrict
// below the type ceiling, never expand beyond it.
type SurfaceSandboxOverride {
  fs_read  string[] @optional
  fs_write string[] @optional
  // Domain allowlist for network access
  network  string[] @optional
  verify unit "SurfaceSandboxOverride schema is valid"
}

// ── Auto-Promotion ──────────────────────────────────────────

type AutoPromotedMcpTool {
  source_command       string @readonly
  source_extension     string @readonly
  // MCP tool name: specforge.{ext_short}.{cmd_id}
  mcp_tool_name        string @readonly
  derived_input_schema JsonSchema
  verify unit "AutoPromotedMcpTool schema is valid"
}

// ── Surface Registry ────────────────────────────────────────

type SurfaceRegistryEntry {
  surface_type      SurfaceType
  contribution_name string @readonly
  extension_name    string @readonly
  export_name       string @readonly
  verify unit "SurfaceRegistryEntry schema is valid"
}

type SurfaceType = command | mcp_tool | mcp_resource | auto_promoted_tool

// ── Surface Errors ──────────────────────────────────────────

type SurfaceError {
  extension_name  string @readonly
  surface_type    SurfaceType
  contribution_id string
  message         string
  export_name     string @optional
  verify unit "SurfaceError schema is valid"
}
