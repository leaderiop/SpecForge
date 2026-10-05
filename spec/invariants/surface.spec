// Surface contribution invariants — uniqueness and sandbox guarantees

invariant surface_contribution_uniqueness "Surface Contribution Uniqueness" {
  guarantee """
    No two extensions MAY contribute the same name within a surface type.
    CLI command IDs MUST be unique across all extensions. MCP tool names
    MUST be unique across all extensions. MCP resource URI templates MUST
    be unique across all extensions. Duplicate contributions MUST produce
    E039 at extension load time.
  """
  risk      medium
  verify property "no two extensions can register the same CLI command ID"
  verify property "no two extensions can register the same MCP tool name"
  verify unit "duplicate surface contribution produces E039"
}

invariant surface_sandbox_ceiling "Surface Sandbox Ceiling" {
  guarantee """
    Every surface export (a cmd__ command, an mcp__ tool or resource)
    MUST run with no capability: no preopened directory, environment,
    arguments, inherited stdio or network. That is the ceiling, so no
    per-contribution sandbox override MAY grant an export more: MCP
    resources MUST NOT write files, and CLI commands MUST NOT exceed the
    extension's SandboxPolicy.
  """
  risk      high
  verify unit "a surface export whose sandbox override asks for every capability is granted none"
}
