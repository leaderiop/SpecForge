requirement INVARIANT-133 "Surface Contribution Uniqueness" {
  category "Invariant"
  tier     v2
  text     """
    No two extensions MAY contribute the same name within a surface type.
    No two commands MAY route under one name (`specforge <short> <cli
    name>`); a later one is E039 at load. MCP tool names
    MUST be unique across all extensions. MCP resource URI templates MUST
    be unique across all extensions. Duplicate contributions MUST produce
    E039 at extension load time.
  """
}

requirement INVARIANT-134 "Surface Sandbox Ceiling" {
  category "Invariant"
  tier     v2
  text     """
    Every surface export (a cmd__ command, an mcp__ tool or resource)
    MUST run with no capability: no preopened directory, environment,
    arguments, inherited stdio or network. That is the ceiling: no
    declaration grants an export more. MCP resources MUST NOT write
    files. A surface's sandbox key, which a guest built before ADR 0037
    may still declare, grants nothing and is W153 at load.
  """
}
