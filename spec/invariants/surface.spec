// Surface contribution invariants — uniqueness and sandbox guarantees

invariant surface_contribution_uniqueness "Surface Contribution Uniqueness" {
  guarantee """
    No two extensions MAY contribute the same name within a surface type.
    CLI command IDs MUST be unique across all extensions. MCP tool names
    MUST be unique across all extensions. MCP resource URI templates MUST
    be unique across all extensions. Duplicate contributions MUST produce
    E039 at extension load time.
  """
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
  risk high

  verify property "sandbox override cannot expand beyond type ceiling"
  verify unit "MCP resource with fs_write override is rejected"
  verify unit "CLI command sandbox intersected with extension sandbox policy"

}
