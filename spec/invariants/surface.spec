// Surface contribution invariants — uniqueness and sandbox guarantees

use "behaviors/surface-contributions"
invariant surface_contribution_uniqueness "Surface Contribution Uniqueness" {
  guarantee """
    No two extensions MAY contribute the same name within a surface type.
    CLI command IDs MUST be unique across all extensions. MCP tool names
    MUST be unique across all extensions. MCP resource URI templates MUST
    be unique across all extensions. Duplicate contributions MUST produce
    E039 at extension load time.
  """
  enforced_by [register_surface_contributions, toggle_surface_contributions]
  risk medium

  verify property "no two extensions can register the same CLI command ID"
  verify property "no two extensions can register the same MCP tool name"
  verify unit "duplicate surface contribution produces E039"

}

invariant surface_sandbox_ceiling "Surface Sandbox Ceiling" {
  guarantee """
    Per-contribution sandbox overrides can only restrict permissions below
    the type ceiling, never expand beyond it. MCP resources MUST NOT have
    fs_write access. CLI commands MUST NOT exceed the extension's
    SandboxPolicy. Surface dispatch MUST enforce the ceiling before
    calling any Wasm export.
  """
  enforced_by [enforce_surface_sandbox, dispatch_surface_command, dispatch_surface_mcp_tool, dispatch_surface_mcp_resource, validate_surface_exports]
  risk high

  verify property "sandbox override cannot expand beyond type ceiling"
  verify unit "MCP resource with fs_write override is rejected"
  verify unit "CLI command sandbox intersected with extension sandbox policy"

}
